use crate::enums::event::Event;
use crate::errors::sdk_error::SdkError;
use crate::event_handler::EventHandler;
use crate::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
use crate::receive_split::receive_split;
use crate::switchboard_server::commands::{ans, cal, msg, usr};
use crate::switchboard_server::event_matcher::{into_event, into_internal_event};
use crate::switchboard_server::p2p;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use crate::switchboard_server::p2p::display_picture_session::DisplayPictureSession;
use crate::user_data::UserData;
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

/// Represents a messaging session with one or more contacts. The official MSN clients usually create a new session every time a conversation
/// window is opened and leave it once it's closed.
#[derive(Debug, uniffi::Object)]
pub struct Switchboard {
    event_tx: async_channel::Sender<Event>,
    event_rx: async_channel::Receiver<Event>,
    sb_tx: mpsc::Sender<Vec<u8>>,
    internal_tx: broadcast::Sender<InternalEvent>,
    tr_id: Arc<AtomicU32>,
    session_id: Arc<RwLock<Option<String>>>,
    cki_string: String,
    user_data: Arc<RwLock<UserData>>,
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
        let internal_task_tx = internal_tx.clone();
        let event_task_tx = event_tx.clone();

        tokio::spawn(async move {
            'outer: while let Ok(messages) = receive_split(&mut rd).await {
                for message in messages {
                    let internal_event = into_internal_event(&message);
                    if let Err(error) = internal_task_tx.send(internal_event) {
                        error!("{error}");
                    }

                    let event = into_event(&message);
                    if let Some(event) = event {
                        if let Err(error) = event_task_tx.send(event).await {
                            error!("{error}");
                            break 'outer;
                        }
                    }
                }
            }

            if let Err(error) = event_task_tx.send(Event::Disconnected).await {
                error!("{error}");
            }

            event_task_tx.close();
        });

        let event_task_tx = event_tx.clone();
        tokio::spawn(async move {
            while let Some(message) = sb_rx.recv().await {
                if let Err(error) = wr.write_all(message.as_slice()).await {
                    error!("{error}");
                }
            }

            if let Err(error) = event_task_tx.send(Event::Disconnected).await {
                error!("{error}");
            }

            event_task_tx.close();
        });

        Ok(Self {
            event_tx,
            event_rx,
            sb_tx,
            internal_tx,
            tr_id: Arc::new(AtomicU32::new(0)),
            session_id: Arc::new(RwLock::new(None)),
            cki_string: cki_string.to_string(),
            user_data,
        })
    }

    async fn handle_p2p_events(&self) -> Result<(), SdkError> {
        let sb_tx = self.sb_tx.clone();
        let mut internal_rx = self.internal_tx.subscribe();
        let mut command_internal_rx = self.internal_tx.subscribe();

        let tr_id = self.tr_id.clone();
        let user_data = self.user_data.clone();

        tokio::spawn(async move {
            while let Ok(event) = internal_rx.recv().await {
                match event {
                    InternalEvent::P2PInvite {
                        destination,
                        message: invite,
                    } => {
                        let _ = p2p::send_display_picture::handle_invite(
                            destination,
                            invite,
                            user_data.clone(),
                            &mut command_internal_rx,
                            tr_id.clone(),
                            sb_tx.clone(),
                        )
                        .await;
                    }

                    InternalEvent::P2PBye {
                        destination,
                        message: bye,
                    } => {
                        let _ = p2p::send_display_picture::handle_bye(
                            destination,
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
            }
        });

        Ok(())
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

        self.handle_p2p_events().await
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

        self.handle_p2p_events().await?;

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

    /// Returns the session ID, if defined.
    pub async fn get_session_id(&self) -> Result<String, SdkError> {
        let session_id = self.session_id.read().await;
        session_id.clone().ok_or(SdkError::CouldNotGetSessionId)
    }

    /// Sends a plain text message to the session.
    pub async fn send_text_message(&self, message: &PlainText) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        msg::send_text_message(&self.tr_id, &self.sb_tx, &mut internal_rx, message).await
    }

    /// Sends a nudge to the session.
    pub async fn send_nudge(&self) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        msg::send_nudge(&self.tr_id, &self.sb_tx, &mut internal_rx).await
    }

    /// Sends an "is writing..." notification to the session.
    pub async fn send_typing_user(&self, email: &str) -> Result<(), SdkError> {
        msg::send_typing_user(&self.tr_id, &self.sb_tx, email).await
    }

    /// Requests a contact's display picture and handles the transfer process. A [DisplayPicture][Event::DisplayPicture] event
    /// is received once the transfer is complete.
    pub async fn request_contact_display_picture(
        &self,
        email: &str,
        msn_object: &str,
    ) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        let mut session = DisplayPictureSession::new();

        let invite;
        {
            let user_data = self.user_data.read().await;
            let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;
            invite = session.invite(email, user_email, msn_object)?
        }

        {
            let mut internal_rx = self.internal_tx.subscribe();
            msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, invite, email).await?;
        }

        let mut picture = Vec::new();
        loop {
            match internal_rx.recv().await.or(Err(SdkError::ReceivingError))? {
                InternalEvent::P2PShouldAck {
                    destination,
                    message,
                } => {
                    {
                        let user_data = self.user_data.read().await;
                        let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;

                        if destination != *user_email {
                            continue;
                        }
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    let ack = DisplayPictureSession::acknowledge(&message)?;
                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email).await?;
                }

                InternalEvent::P2PInvite {
                    destination,
                    message: invite,
                } => {
                    {
                        let user_data = self.user_data.read().await;
                        let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;

                        if destination != *user_email {
                            continue;
                        }
                    }

                    let mut internal_rx = self.internal_tx.subscribe();
                    let invite_string = unsafe { str::from_utf8_unchecked(invite.as_slice()) };
                    let mut invite_parameters = invite_string.lines();

                    invite_parameters.next();
                    let to = invite_parameters.next().ok_or(SdkError::P2PInviteError)?;

                    {
                        let user_data = self.user_data.read().await;
                        let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;

                        if !to.contains(format!("msnmsgr:{user_email}").as_str()) {
                            continue;
                        }
                    }

                    let from = invite_parameters
                        .next()
                        .ok_or(SdkError::P2PInviteError)?
                        .replace("From: <msnmsgr:", "")
                        .replace(">", "");

                    let session = DisplayPictureSession::new_from_invite(&invite)
                        .or(Err(SdkError::P2PInviteError))?;

                    let decline = session
                        .decline(from.as_str(), to)
                        .or(Err(SdkError::P2PInviteError))?;

                    msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, decline, email)
                        .await?;
                }

                InternalEvent::P2PData {
                    destination,
                    message: data,
                } => {
                    {
                        let user_data = self.user_data.read().await;
                        let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;

                        if destination != *user_email {
                            continue;
                        }
                    }

                    let binary_header = data.get(..48).ok_or(SdkError::BinaryHeaderReadingError)?;
                    let mut cursor = Cursor::new(binary_header);
                    let (_, binary_header) = BinaryHeader::from_reader((&mut cursor, 0))
                        .or(Err(SdkError::BinaryHeaderReadingError))?;

                    picture.extend_from_slice(
                        data.get(48..).ok_or(SdkError::BinaryHeaderReadingError)?,
                    );

                    let data_len = picture.len();
                    trace!("Data received so far: {data_len}");

                    if data_len as u64 == binary_header.total_data_size {
                        let ack = DisplayPictureSession::acknowledge(&data)?;
                        msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, ack, email)
                            .await?;

                        break;
                    }
                }

                _ => (),
            }

            // Introduce some delay so all chunks are received
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let bye;
        {
            let user_data = self.user_data.read().await;
            let user_email = user_data.email.as_ref().ok_or(SdkError::NotLoggedIn)?;
            bye = session.bye(email, user_email)?;
        }

        msg::send_p2p(&self.tr_id, &self.sb_tx, &mut internal_rx, bye, email).await?;
        self.event_tx
            .send(Event::DisplayPicture {
                email: email.to_owned(),
                data: picture,
            })
            .await
            .or(Err(SdkError::TransmittingError))?;

        Ok(())
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
        Ok(())
    }
}
