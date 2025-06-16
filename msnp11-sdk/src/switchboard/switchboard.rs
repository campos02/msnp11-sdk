use crate::event::Event;
use crate::event_handler::EventHandler;
use crate::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
use crate::models::user_data::UserData;
use crate::receive_split_into_base64::receive_split_into_base64;
use crate::sdk_error::SdkError;
use crate::switchboard::commands::ans::Ans;
use crate::switchboard::commands::cal::Cal;
use crate::switchboard::commands::msg::Msg;
use crate::switchboard::commands::usr::Usr;
use crate::switchboard::event_matcher::{into_event, into_internal_event};
use crate::switchboard::p2p::binary_header::BinaryHeader;
use crate::switchboard::p2p::display_picture_session::DisplayPictureSession;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use core::str;
use deku::DekuContainerRead;
use log::trace;
use std::error::Error;
use std::io::Cursor;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};

/// Represents a messaging session with one or more contacts. The official MSNP clients usually create a new session every time a conversation
/// window is opened and leave it when it's closed.
#[derive(Debug, uniffi::Object)]
pub struct Switchboard {
    event_tx: async_channel::Sender<Event>,
    event_rx: async_channel::Receiver<Event>,
    sb_tx: mpsc::Sender<Vec<u8>>,
    internal_tx: broadcast::Sender<InternalEvent>,
    tr_id: Arc<AtomicU32>,
    session_id: Arc<RwLock<Option<String>>>,
    cki_string: String,
    user_data: Arc<Mutex<UserData>>,
}

impl Switchboard {
    pub(crate) async fn new(
        server: &str,
        port: &str,
        cki_string: &str,
        user_data: Arc<Mutex<UserData>>,
    ) -> Result<Self, SdkError> {
        let (event_tx, event_rx) = async_channel::bounded::<Event>(32);
        let (sb_tx, mut sb_rx) = mpsc::channel::<Vec<u8>>(16);
        let (internal_tx, _) = broadcast::channel::<InternalEvent>(128);

        let socket = TcpStream::connect(format!("{server}:{port}"))
            .await
            .or(Err(SdkError::CouldNotConnectToServer))?;

        let (mut rd, mut wr) = socket.into_split();

        let internal_task_tx = internal_tx.clone();
        let event_task_tx = event_tx.clone();
        tokio::spawn(async move {
            while let Ok(base64_messages) = receive_split_into_base64(&mut rd).await {
                for base64_message in base64_messages {
                    let internal_event = into_internal_event(&base64_message);
                    internal_task_tx
                        .send(internal_event)
                        .expect("Error sending internal event to channel");

                    let event = into_event(&base64_message);
                    if let Some(event) = event {
                        event_task_tx
                            .send(event)
                            .await
                            .expect("Error sending event to channel");
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Some(message) = sb_rx.recv().await {
                wr.write_all(message.as_slice())
                    .await
                    .expect("Error sending message to socket");
            }
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
        let user_data_arc = self.user_data.clone();
        let user_email;
        {
            let user_data = self
                .user_data
                .lock()
                .or(Err(SdkError::CouldNotGetUserData))?;

            user_email = user_data
                .email
                .as_ref()
                .ok_or(SdkError::NotLoggedIn)?
                .clone();
        }

        tokio::spawn(async move {
            while let Ok(event) = internal_rx.recv().await {
                match event {
                    InternalEvent::P2PInvite {
                        destination,
                        message: invite,
                    } => {
                        let user_data;
                        {
                            let Ok(user_data_lock) = user_data_arc.lock() else {
                                continue;
                            };

                            user_data = user_data_lock.clone();
                        }

                        if destination != *user_email {
                            continue;
                        }

                        let invite_string =
                            unsafe { str::from_utf8_unchecked(invite.as_slice()) }.to_string();

                        let mut invite_parameters = invite_string.lines();
                        invite_parameters.next();

                        let Some(to) = invite_parameters.next() else {
                            continue;
                        };

                        if !to.contains(format!("msnmsgr:{user_email}").as_str()) {
                            continue;
                        }

                        let Some(from) = invite_parameters.next() else {
                            continue;
                        };

                        let from = from.replace("From: <msnmsgr:", "").replace(">", "");
                        let Ok(session) = DisplayPictureSession::new_from_invite(&invite) else {
                            continue;
                        };

                        let Ok(ack_payload) = DisplayPictureSession::acknowledge(invite) else {
                            continue;
                        };

                        if Msg::send_p2p(
                            &tr_id,
                            &sb_tx,
                            &mut command_internal_rx,
                            ack_payload,
                            from.as_str(),
                        )
                        .await
                        .is_err()
                        {
                            continue;
                        }

                        let Some(context) =
                            invite_parameters.find(|line| line.contains("Context: "))
                        else {
                            continue;
                        };

                        let context = context.replace("Context: ", "");
                        let Some(msn_object) = user_data.msn_object.as_ref() else {
                            continue;
                        };

                        if context != STANDARD.encode((msn_object.to_owned() + "\0").as_bytes()) {
                            continue;
                        }

                        let Ok(ok_payload) = session.ok(from.as_str(), to) else {
                            continue;
                        };

                        if Msg::send_p2p(
                            &tr_id,
                            &sb_tx,
                            &mut command_internal_rx,
                            ok_payload,
                            from.as_str(),
                        )
                        .await
                        .is_err()
                        {
                            continue;
                        }

                        let Ok(preparation_payload) = session.data_preparation() else {
                            continue;
                        };

                        if Msg::send_p2p(
                            &tr_id,
                            &sb_tx,
                            &mut command_internal_rx,
                            preparation_payload,
                            from.as_str(),
                        )
                        .await
                        .is_err()
                        {
                            continue;
                        }

                        let Some(display_picture) = user_data.display_picture.clone() else {
                            continue;
                        };

                        let Ok(data_payloads) = session.data(display_picture) else {
                            continue;
                        };

                        for data_payload in data_payloads {
                            if Msg::send_p2p(
                                &tr_id,
                                &sb_tx,
                                &mut command_internal_rx,
                                data_payload,
                                from.as_str(),
                            )
                            .await
                            .is_err()
                            {
                                continue;
                            }
                        }
                    }

                    InternalEvent::P2PBye {
                        destination,
                        message: bye,
                    } => {
                        if destination != *user_email {
                            continue;
                        }

                        let bye_string =
                            unsafe { str::from_utf8_unchecked(bye.as_slice()) }.to_string();

                        let mut bye_parameters = bye_string.lines();
                        bye_parameters.next();
                        bye_parameters.next();

                        let Some(from) = bye_parameters.next() else {
                            continue;
                        };

                        let from = from.replace("From: <msnmsgr:", "").replace(">", "");
                        let Ok(ack_payload) = DisplayPictureSession::acknowledge(bye) else {
                            continue;
                        };

                        if Msg::send_p2p(
                            &tr_id,
                            &sb_tx,
                            &mut command_internal_rx,
                            ack_payload,
                            from.as_str(),
                        )
                        .await
                        .is_err()
                        {
                            continue;
                        }
                    }

                    _ => (),
                }
            }
        });

        Ok(())
    }

    pub(crate) async fn login(&self, email: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Usr::send(
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
        email: &String,
        session_id: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut internal_rx = self.internal_tx.subscribe();
        Ans::send(
            &self.tr_id,
            &self.sb_tx,
            &mut internal_rx,
            &email,
            &self.cki_string,
            session_id,
        )
        .await?;

        self.handle_p2p_events().await?;

        let mut session_id_lock = self
            .session_id
            .write()
            .or(Err(SdkError::CouldNotSetSessionId))?;

        *session_id_lock = Some(session_id.to_owned());
        Ok(())
    }

    /// Adds a handler closure. If you're using this SDK with Rust, not through a foreign binding, then this is the preferred method of
    /// handling events.
    pub fn add_event_handler_closure<F>(&self, f: F)
    where
        F: Fn(Event) + Send + 'static,
    {
        let event_rx = self.event_rx.clone();
        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                f(event);
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

    /// Invites a new contact to this switchboard session. This makes them temporary chat rooms.
    pub async fn invite(&self, email: &String) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();

        let session_id = Some(Cal::send(&self.tr_id, &self.sb_tx, &mut internal_rx, email).await?);
        let mut session_id_lock = self
            .session_id
            .write()
            .or(Err(SdkError::CouldNotSetSessionId))?;

        *session_id_lock = session_id;
        Ok(())
    }

    /// Returns the session ID, if it's defined.
    pub fn get_session_id(&self) -> Result<Option<String>, SdkError> {
        let session_id = self
            .session_id
            .read()
            .or(Err(SdkError::CouldNotGetSessionId))?;

        Ok(session_id.clone())
    }

    /// Sends a plain text message to the session.
    pub async fn send_text_message(&self, message: &PlainText) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Msg::send_text_message(&self.tr_id, &self.sb_tx, &mut internal_rx, message).await
    }

    /// Sends a nudge to the session.
    pub async fn send_nudge(&self) -> Result<(), SdkError> {
        let mut internal_rx = self.internal_tx.subscribe();
        Msg::send_nudge(&self.tr_id, &self.sb_tx, &mut internal_rx).await
    }

    /// Sends an "is writing..." notification to the session.
    pub async fn send_typing_user(&self, email: &String) -> Result<(), SdkError> {
        Msg::send_typing_user(&self.tr_id, &self.sb_tx, email).await
    }

    /// Requests the contact's display picture and handles the transfer process.
    pub async fn request_contact_display_picture(
        &self,
        email: &String,
        msn_object: &String,
    ) -> Result<(), SdkError> {
        let user_email;
        {
            let user_data = self
                .user_data
                .lock()
                .or(Err(SdkError::CouldNotGetUserData))?;

            user_email = user_data
                .email
                .as_ref()
                .ok_or(SdkError::NotLoggedIn)?
                .clone();
        }

        let mut internal_rx = self.internal_tx.subscribe();
        let mut command_internal_rx = self.internal_tx.subscribe();
        let mut session = DisplayPictureSession::new();
        let invite = session.invite(email, &user_email, msn_object)?;

        Msg::send_p2p(
            &self.tr_id,
            &self.sb_tx,
            &mut command_internal_rx,
            invite,
            email,
        )
        .await?;

        let mut picture: Vec<u8> = Vec::new();
        loop {
            match internal_rx.recv().await.or(Err(SdkError::ReceivingError))? {
                InternalEvent::P2POk {
                    destination,
                    message,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let ack = DisplayPictureSession::acknowledge(message)?;
                    Msg::send_p2p(
                        &self.tr_id,
                        &self.sb_tx,
                        &mut command_internal_rx,
                        ack,
                        email,
                    )
                    .await?;
                }

                InternalEvent::P2PDataPreparation {
                    destination,
                    message,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let ack = DisplayPictureSession::acknowledge(message)?;
                    Msg::send_p2p(
                        &self.tr_id,
                        &self.sb_tx,
                        &mut command_internal_rx,
                        ack,
                        email,
                    )
                    .await?;
                }

                InternalEvent::P2PData {
                    destination,
                    message: data,
                } => {
                    if destination != *user_email {
                        continue;
                    }

                    let binary_header = data[..48].to_vec();
                    let mut cursor = Cursor::new(binary_header);
                    let (_, binary_header) = BinaryHeader::from_reader((&mut cursor, 0))
                        .or(Err(SdkError::BinaryHeaderReadingError))?;

                    picture.extend_from_slice(&data[48..]);
                    if picture.len() == binary_header.total_data_size as usize {
                        let ack = DisplayPictureSession::acknowledge(data)?;
                        Msg::send_p2p(
                            &self.tr_id,
                            &self.sb_tx,
                            &mut command_internal_rx,
                            ack,
                            email,
                        )
                        .await?;
                        break;
                    }
                }

                _ => (),
            }

            // Introduce some delay so all chunks are received
            tokio::time::sleep(Duration::from_millis(150)).await;
        }

        let bye = session.bye(email, &user_email)?;
        Msg::send_p2p(
            &self.tr_id,
            &self.sb_tx,
            &mut command_internal_rx,
            bye,
            email,
        )
        .await?;

        self.event_tx
            .send(Event::DisplayPicture {
                email: email.to_owned(),
                data: picture,
            })
            .await
            .or(Err(SdkError::TransmittingError))?;

        Ok(())
    }

    /// Disconnects from the switchboard.
    pub async fn disconnect(&self) -> Result<(), SdkError> {
        let command = "OUT\r\n";
        trace!("C: {command}");
        self.sb_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))
    }
}
