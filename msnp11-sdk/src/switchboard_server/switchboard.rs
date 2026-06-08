use crate::enums::event::Event;
use crate::enums::internal_event::InternalEvent;
use crate::errors::messaging_error::MessagingError;
use crate::errors::p2p_error::P2pError;
use crate::errors::sdk_error::SdkError;
#[cfg(feature = "uniffi")]
use crate::event_handler::EventHandler;
#[cfg(feature = "file-transfers")]
use crate::models::file_transfer_request::FileTransferRequest;
use crate::models::plain_text::PlainText;
use crate::models::user_data::UserData;
use crate::receive_split::receive_split;
use crate::switchboard_server::commands::{ans, cal, msg, usr};
use crate::switchboard_server::event_matcher::{into_event, into_internal_event};
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use crate::switchboard_server::p2p::p2p_session::P2pSession;
use core::str;
use deku::DekuContainerRead;
use log::{error, trace};
use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio_util::sync::CancellationToken;

/// Represents a messaging session with one or more contacts. The official MSN clients usually create a new session every time a conversation
/// window is opened and leave it once it's closed.
#[derive(Debug)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct Switchboard {
    event_tx: async_channel::Sender<Event>,
    event_rx: async_channel::Receiver<Event>,
    sb_tx: mpsc::Sender<Vec<u8>>,
    internal_tx: broadcast::Sender<InternalEvent>,
    tr_id: Arc<AtomicU32>,
    session_id: RwLock<Option<String>>,
    cki_string: String,
    user_data: Arc<RwLock<UserData>>,
    cancellation_token: CancellationToken,
}

impl Switchboard {
    pub(crate) async fn new(
        server: &str,
        port: &str,
        cki_string: &str,
        user_data: Arc<RwLock<UserData>>,
    ) -> Result<Self, SdkError> {
        let (event_tx, event_rx) = async_channel::unbounded();
        let (sb_tx, mut sb_rx) = mpsc::channel::<Vec<u8>>(256);
        let (internal_tx, _) = broadcast::channel::<InternalEvent>(256);

        let socket = TcpStream::connect(format!("{server}:{port}"))
            .await
            .or(Err(SdkError::CouldNotConnectToServer))?;

        let (mut rd, mut wr) = socket.into_split();
        let task_internal_tx = internal_tx.clone();
        let task_event_tx = event_tx.clone();

        let cancellation_token = CancellationToken::new();
        let task_cancellation_token = cancellation_token.clone();

        tokio::spawn(async move {
            'outer: while let Ok(messages) =
                receive_split(&mut rd, task_cancellation_token.clone()).await
            {
                for message in messages {
                    let internal_event = into_internal_event(&message);
                    if let Err(error) = task_internal_tx.send(internal_event) {
                        error!("{error}");
                    }

                    let event = into_event(&message);
                    if let Some(event) = event
                        && let Err(error) = task_event_tx.send(event).await
                    {
                        error!("{error}");
                        break 'outer;
                    }
                }
            }

            if let Err(error) = task_event_tx.send(Event::Disconnected).await {
                error!("{error}");
            }

            task_event_tx.close();
            task_cancellation_token.cancel();
        });

        let task_event_tx = event_tx.clone();
        let task_cancellation_token = cancellation_token.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    message = sb_rx.recv() => {
                        if let Some(message) = message {
                            if let Err(error) = wr.write_all(&message).await {
                                error!("{error}")
                            }
                        } else {
                            break;
                        }
                    }

                    _ = task_cancellation_token.cancelled() => {
                        break;
                    }
                }
            }

            if let Err(error) = task_event_tx.send(Event::Disconnected).await {
                error!("{error}");
            }

            task_event_tx.close();
            task_cancellation_token.cancel();
        });

        Ok(Self {
            event_tx,
            event_rx,
            sb_tx,
            internal_tx,
            tr_id: Arc::new(AtomicU32::new(0)),
            session_id: RwLock::new(None),
            cki_string: cki_string.to_string(),
            user_data,
            cancellation_token,
        })
    }

    fn handle_p2p_events(&self) {
        let sb_tx = self.sb_tx.clone();
        #[cfg(feature = "file-transfers")]
        let event_tx = self.event_tx.clone();
        let mut internal_rx = self.internal_tx.subscribe();
        let mut command_internal_rx = self.internal_tx.subscribe();
        let task_cancellation_token = self.cancellation_token.clone();

        let tr_id = self.tr_id.clone();
        let user_data = self.user_data.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = internal_rx.recv() => {
                        if let Ok(event) = event {
                            match event {
                                InternalEvent::DisplayPictureInvite {
                                    to,
                                    from,
                                    branch,
                                    call_id,
                                    session_id,
                                    context,
                                    message: invite,
                                } => {
                                    let mut session = P2pSession::new_from_existing_session(branch, call_id, session_id);
                                    let _ = session.handle_display_picture_invite(
                                            &to,
                                            &from,
                                            &context,
                                            invite,
                                            user_data.clone(),
                                            &mut command_internal_rx,
                                            tr_id.clone(),
                                            sb_tx.clone(),
                                        )
                                        .await;
                                }

                                #[cfg(feature = "file-transfers")]
                                InternalEvent::FileTransferInvite {
                                    to,
                                    from,
                                    branch,
                                    call_id,
                                    session_id,
                                    file_size,
                                    file_name,
                                    message,
                                } => {
                                    if let Ok(ack) = P2pSession::acknowledge(&message) {
                                        let _ = msg::send_p2p(&tr_id, &sb_tx, &mut command_internal_rx, ack, &from).await;
                                    }

                                    let _ = event_tx.send(Event::FileTransferRequest {
                                        email: from.clone(),
                                        file_name,
                                        file_size,
                                        request: FileTransferRequest {
                                            to,
                                            from,
                                            branch: branch.to_string(),
                                            call_id: call_id.to_string(),
                                            session_id,
                                        }
                                    }).await.map_err(|error| error!("{error}"));
                                }

                                InternalEvent::P2pBye {
                                    to,
                                    from,
                                    message: bye,
                                } => {
                                    let _ = P2pSession::handle_bye(
                                        &to,
                                        &from,
                                        bye,
                                        user_data.clone(),
                                        &mut command_internal_rx,
                                        tr_id.clone(),
                                        sb_tx.clone(),
                                    )
                                    .await;
                                }

                                _ => (),
                            }
                        } else {
                            break;
                        }
                    }

                    _ = task_cancellation_token.cancelled() => {
                        break;
                    }
                }
            }
        });
    }

    pub(crate) async fn login(&self, email: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        usr::send(
            &self.tr_id,
            &self.sb_tx,
            &mut internal_rx,
            email,
            &self.cki_string,
        )
        .await?;

        self.handle_p2p_events();
        Ok(())
    }

    pub(crate) async fn answer(
        &self,
        email: &str,
        session_id: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut internal_rx = self.internal_tx.subscribe();
        ans::send(
            &self.tr_id,
            &self.sb_tx,
            &mut internal_rx,
            email,
            &self.cki_string,
            session_id,
        )
        .await?;

        self.handle_p2p_events();
        let mut session_id_lock = self.session_id.write().await;
        *session_id_lock = Some(session_id.to_owned());

        Ok(())
    }

    /// Adds a handler closure. If you're using this SDK with Rust, not through a foreign binding, then this is the preferred method of
    /// handling events.
    pub fn add_event_handler_closure<F, R>(&self, f: F)
    where
        F: Fn(Event) -> R + Send + 'static,
        R: Future<Output = ()> + Send,
    {
        let event_rx = self.event_rx.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                f(event).await;
            }
        });
    }

    #[cfg(feature = "uniffi")]
    /// Adds a new handler that implements the [EventHandler] trait.
    ///
    /// This exists for the foreign language bindings, with which generics don't
    /// work. Prefer [`add_event_handler_closure`][Switchboard::add_event_handler_closure] if using this SDK with Rust.
    pub fn add_event_handler(&self, handler: Arc<dyn EventHandler>) {
        let event_rx = self.event_rx.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                handler.handle(event).await;
            }
        });
    }

    /// Invites a new contact to this switchboard session. This makes it a temporary group chat.
    pub async fn invite(&self, email: &str) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();

        let session_id = Some(cal::send(&self.tr_id, &self.sb_tx, &mut internal_rx, email).await?);
        let mut session_id_lock = self.session_id.write().await;

        *session_id_lock = session_id;
        Ok(())
    }

    /// Returns the session ID.
    pub async fn get_session_id(&self) -> Result<String, MessagingError> {
        let session_id = self.session_id.read().await;
        session_id
            .clone()
            .ok_or(MessagingError::CouldNotGetSessionId)
    }

    /// Sends a plain text message to the session.
    pub async fn send_text_message(&self, message: &PlainText) -> Result<(), MessagingError> {
        let mut internal_rx = self.internal_tx.subscribe();
        msg::send_text_message(&self.tr_id, &self.sb_tx, &mut internal_rx, message).await
    }

    /// Sends a nudge to the session.
    pub async fn send_nudge(&self) -> Result<(), MessagingError> {
        let mut internal_rx = self.internal_tx.subscribe();
        msg::send_nudge(&self.tr_id, &self.sb_tx, &mut internal_rx).await
    }

    /// Sends an "is writing..." notification to the session.
    pub async fn send_typing_user(&self, email: &str) -> Result<(), MessagingError> {
        msg::send_typing_user(&self.tr_id, &self.sb_tx, email).await
    }

    /// Requests a contact's display picture and handles the transfer process. A [DisplayPicture][Event::DisplayPicture] event
    /// is received once the transfer is complete.
    pub async fn request_contact_display_picture(
        &self,
        email: &str,
        msn_object: &str,
    ) -> Result<(), P2pError> {
        let mut internal_rx = self.internal_tx.subscribe();
        let mut session = P2pSession::new();

        let user_email;
        {
            let user_data = self.user_data.read().await;
            user_email = user_data.email.clone().ok_or(P2pError::NotLoggedIn)?;
        }

        let invite = session.picture_invite(email, &user_email, msn_object)?;
        {
            let mut internal_rx = self.internal_tx.subscribe();
            msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, invite, email).await?;
        }

        let mut picture = Vec::new();
        loop {
            match internal_rx.recv().await.or(Err(P2pError::ReceivingError))? {
                InternalEvent::P2pShouldAck {
                    destination,
                    message,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    let ack = P2pSession::acknowledge(&message)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email).await?;
                }

                InternalEvent::DisplayPictureInvite {
                    to,
                    from: _,
                    branch,
                    call_id,
                    session_id,
                    context: _,
                    message: invite,
                } => {
                    if to != *user_email {
                        continue;
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    session = P2pSession::new_from_existing_session(branch, call_id, session_id);

                    let ack = P2pSession::acknowledge(&invite)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email).await?;
                }

                InternalEvent::P2pData {
                    destination,
                    message: data,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let binary_header = data.get(..48).ok_or(P2pError::BinaryHeaderReadingError)?;
                    let mut cursor = Cursor::new(binary_header);
                    let (_, binary_header) = BinaryHeader::from_reader((&mut cursor, 0))
                        .or(Err(P2pError::BinaryHeaderReadingError))?;

                    picture.extend_from_slice(
                        data.get(48..).ok_or(P2pError::BinaryHeaderReadingError)?,
                    );

                    let data_len = picture.len();
                    trace!("Data received so far: {data_len}");

                    if data_len as u64 == binary_header.total_data_size {
                        let ack = P2pSession::acknowledge(&data)?;
                        msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email)
                            .await?;

                        break;
                    }
                }

                _ => (),
            }
        }

        let bye = session.bye(email, &user_email)?;
        msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, bye, email).await?;

        self.event_tx
            .send(Event::DisplayPicture {
                email: email.to_owned(),
                data: picture,
            })
            .await
            .or(Err(P2pError::TransmittingError))?;

        Ok(())
    }

    #[cfg(feature = "file-transfers")]
    pub async fn send_file(
        &self,
        email: &str,
        file_name: &str,
        file: &[u8],
    ) -> Result<(), P2pError> {
        let mut session = P2pSession::new();
        let mut internal_rx = self.internal_tx.subscribe();

        let user_email;
        {
            let user_data = self.user_data.read().await;
            user_email = user_data.email.clone().ok_or(P2pError::NotLoggedIn)?;
        }

        let invite = session.file_invite(email, &user_email, file_name, file.len() as u64)?;
        {
            let mut internal_rx = self.internal_tx.subscribe();
            msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, invite, email).await?;
        }

        loop {
            match internal_rx.recv().await.or(Err(P2pError::ReceivingError))? {
                InternalEvent::P2pShouldAck {
                    destination,
                    message,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    let ack = P2pSession::acknowledge(&message)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email).await?;
                }

                InternalEvent::P2pOk {
                    destination,
                    message,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    let ack = P2pSession::acknowledge(&message)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email).await?;

                    let invite = session.direct_connection_invite(email, &user_email)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, invite, email)
                        .await?;
                }

                InternalEvent::P2pDirectConnectionOk {
                    destination,
                    message,
                    bridge,
                    listening,
                    nonce,
                    ips,
                    port,
                } => {
                    if destination != user_email {
                        continue;
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    let ack = P2pSession::acknowledge(&message)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email).await?;

                    // Use direct connection if possible or fall back to sending through Switchboard
                    if !listening
                        || bridge != "TCPv1"
                        || session
                            .direct_connection_send_file(
                                &ips,
                                port,
                                &nonce,
                                email,
                                &user_email,
                                file,
                            )
                            .await
                            .is_err()
                    {
                        let data_payloads = session
                            .data(file, true)
                            .or(Err(P2pError::CouldNotSendFile))?;

                        for data_payload in data_payloads {
                            msg::send_p2p(
                                &self.tr_id,
                                &self.sb_tx,
                                &mut internal_rx,
                                data_payload,
                                email,
                            )
                            .await?;
                        }

                        let bye = session.bye(email, &user_email)?;
                        msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, bye, email)
                            .await?;
                    }

                    break;
                }

                InternalEvent::P2pDecline {
                    destination,
                    message,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    let ack = P2pSession::acknowledge(&message)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email).await?;

                    return Err(P2pError::FileTransferDeclined);
                }

                _ => (),
            }
        }

        // Check if the transfer was cancelled at some point
        while let Ok(event) = internal_rx.try_recv() {
            if let InternalEvent::P2pBye { to, .. } = event
                && to == *user_email
            {
                return Err(P2pError::FileTransferCancelled);
            }
        }

        Ok(())
    }

    #[cfg(feature = "file-transfers")]
    pub async fn accept_file_request(
        &self,
        request: FileTransferRequest,
    ) -> Result<Vec<u8>, P2pError> {
        {
            let user_data = self.user_data.read().await;
            let user_email = user_data.email.as_ref().ok_or(P2pError::NotLoggedIn)?;

            if request.to != *user_email {
                return Err(P2pError::OtherDestination);
            }
        }

        let user_email = request.to;
        let email = request.from;
        let mut internal_rx = self.internal_tx.subscribe();

        let branch = guid_create::GUID::parse(&request.branch).or(Err(P2pError::P2pInvite))?;
        let call_id = guid_create::GUID::parse(&request.call_id).or(Err(P2pError::P2pInvite))?;
        let mut session =
            P2pSession::new_from_existing_session(branch, call_id, request.session_id);

        let ok = session.ok(&email, &user_email)?;
        {
            let mut internal_rx = self.internal_tx.subscribe();
            msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ok, &email).await?;
        }

        let mut file = Vec::new();
        loop {
            match internal_rx.recv().await.or(Err(P2pError::ReceivingError))? {
                InternalEvent::P2pShouldAck {
                    destination,
                    message,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    let ack = P2pSession::acknowledge(&message)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, &email).await?;
                }

                InternalEvent::P2pDirectConnectionInvite {
                    to,
                    branch,
                    call_id,
                    message: invite,
                } => {
                    if to != *user_email {
                        continue;
                    }

                    let mut session = P2pSession::new_from_existing_session(branch, call_id, 0);
                    let ack = P2pSession::acknowledge(&invite)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, &email).await?;

                    let ok = session.direct_connection_ok(&email, &user_email).await?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ok, &email).await?;
                }

                InternalEvent::P2pData {
                    destination,
                    message: data,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let binary_header = data.get(..48).ok_or(P2pError::BinaryHeaderReadingError)?;
                    let mut cursor = Cursor::new(binary_header);
                    let (_, binary_header) = BinaryHeader::from_reader((&mut cursor, 0))
                        .or(Err(P2pError::BinaryHeaderReadingError))?;

                    file.extend_from_slice(
                        data.get(48..).ok_or(P2pError::BinaryHeaderReadingError)?,
                    );

                    let data_len = file.len();
                    trace!("Data received so far: {data_len}");

                    if data_len as u64 == binary_header.total_data_size {
                        let ack = P2pSession::acknowledge(&data)?;
                        msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, &email)
                            .await?;

                        break;
                    }
                }

                _ => (),
            }
        }

        // Check if the transfer was cancelled at some point
        while let Ok(event) = internal_rx.try_recv() {
            if let InternalEvent::P2pBye { to, .. } = &event
                && *to == *user_email
            {
                return Err(P2pError::FileTransferCancelled);
            }
        }

        Ok(file)
    }

    #[cfg(feature = "file-transfers")]
    pub async fn decline_file_request(&self, request: FileTransferRequest) -> Result<(), P2pError> {
        {
            let user_data = self.user_data.read().await;
            let user_email = user_data.email.as_ref().ok_or(P2pError::NotLoggedIn)?;

            if request.to != *user_email {
                return Err(P2pError::OtherDestination);
            }
        }

        let user_email = request.to;
        let email = request.from;

        let mut internal_rx = self.internal_tx.subscribe();
        let branch = guid_create::GUID::parse(&request.branch).or(Err(P2pError::P2pInvite))?;
        let call_id = guid_create::GUID::parse(&request.call_id).or(Err(P2pError::P2pInvite))?;
        let mut session =
            P2pSession::new_from_existing_session(branch, call_id, request.session_id);

        let decline = session.decline(&email, &user_email)?;
        msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, decline, &email).await
    }

    /// Disconnects from the Switchboard.
    pub async fn disconnect(&self) -> Result<(), SdkError> {
        let command = "OUT\r\n";
        trace!("C: {command}");

        self.sb_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        self.event_tx.close();
        self.cancellation_token.cancel();
        Ok(())
    }
}
