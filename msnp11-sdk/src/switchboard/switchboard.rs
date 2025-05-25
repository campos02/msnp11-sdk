use crate::client::Client;
use crate::connection_error::ConnectionError;
use crate::event::Event;
use crate::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
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
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub struct Switchboard {
    event_tx: mpsc::Sender<Event>,
    event_rx: mpsc::Receiver<Event>,
    sb_tx: mpsc::Sender<Vec<u8>>,
    internal_tx: broadcast::Sender<InternalEvent>,
    tr_id: Arc<tokio::sync::Mutex<usize>>,
    session_id: Option<String>,
    cki_string: String,
    display_picture: Option<Vec<u8>>,
    msn_object: Option<String>,
    user_email: String,
}

impl Switchboard {
    pub(crate) async fn new(
        server: &str,
        port: &str,
        cki_string: &str,
        display_picture: Option<Vec<u8>>,
        msn_object: Option<String>,
        user_email: String,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (event_tx, event_rx) = mpsc::channel::<Event>(64);
        let (sb_tx, mut sb_rx) = mpsc::channel::<Vec<u8>>(16);
        let (internal_tx, _) = broadcast::channel::<InternalEvent>(64);

        let socket = TcpStream::connect(format!("{server}:{port}")).await?;
        let (mut rd, mut wr) = socket.into_split();

        let internal_task_tx = internal_tx.clone();
        tokio::spawn(async move {
            while let Ok(base64_messages) = Client::socket_messages_to_base64(&mut rd).await {
                for base64_message in base64_messages {
                    let internal_event = into_internal_event(&base64_message);
                    internal_task_tx
                        .send(internal_event)
                        .expect("Error sending internal event to channel");
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
            tr_id: Arc::new(tokio::sync::Mutex::new(0)),
            session_id: None,
            cki_string: cki_string.to_string(),
            display_picture,
            msn_object,
            user_email,
        })
    }

    async fn listen_to_internal_events(
        &self,
        mut internal_rx: broadcast::Receiver<InternalEvent>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let internal_tx = self.internal_tx.clone();
        let sb_tx = self.sb_tx.clone();
        let event_tx = self.event_tx.clone();
        let display_picture = self.display_picture.clone();
        let msn_object = self.msn_object.clone();
        let user_email = self.user_email.clone();
        let tr_id = self.tr_id.clone();

        tokio::spawn(async move {
            while let Ok(event) = internal_rx.recv().await {
                match event {
                    InternalEvent::ServerReply(reply) => {
                        let event = into_event(&reply);
                        event_tx
                            .send(event)
                            .await
                            .expect("Error sending event to channel");
                    }

                    InternalEvent::P2PInvite {
                        destination,
                        message: invite,
                    } => {
                        if destination != user_email {
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
                        let Some(context) =
                            invite_parameters.find(|line| line.contains("Context: "))
                        else {
                            continue;
                        };

                        let context = context.replace("Context: ", "");
                        let Some(msn_object) = msn_object.clone() else {
                            continue;
                        };

                        if context != STANDARD.encode(msn_object) {
                            continue;
                        }

                        let Ok(session) = DisplayPictureSession::new_from_invite(&invite) else {
                            continue;
                        };

                        let Ok(ack_payload) = DisplayPictureSession::acknowledge(invite) else {
                            continue;
                        };

                        let mut tr_id = tr_id.lock().await;
                        if Msg::send_p2p(
                            &mut tr_id,
                            &sb_tx,
                            &internal_tx,
                            ack_payload,
                            from.as_str(),
                        )
                        .await
                        .is_err()
                        {
                            continue;
                        }

                        let Ok(ok_payload) = session.ok(from.as_str(), to) else {
                            continue;
                        };

                        if Msg::send_p2p(
                            &mut tr_id,
                            &sb_tx,
                            &internal_tx,
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
                            &mut tr_id,
                            &sb_tx,
                            &internal_tx,
                            preparation_payload,
                            from.as_str(),
                        )
                        .await
                        .is_err()
                        {
                            continue;
                        }

                        let Some(display_picture) = display_picture.clone() else {
                            continue;
                        };

                        let Ok(data_payloads) = session.data(display_picture) else {
                            continue;
                        };

                        for data_payload in data_payloads {
                            if Msg::send_p2p(
                                &mut tr_id,
                                &sb_tx,
                                &internal_tx,
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
                        if destination != user_email {
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

                        let mut tr_id = tr_id.lock().await;
                        if Msg::send_p2p(
                            &mut tr_id,
                            &sb_tx,
                            &internal_tx,
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

    pub async fn receive_event(&mut self) -> Result<Event, ConnectionError> {
        self.event_rx
            .recv()
            .await
            .ok_or(ConnectionError::Disconnected)
    }

    pub fn event_queue_size(&self) -> usize {
        self.event_rx.len()
    }

    pub(crate) async fn login(&self, email: &String) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Subscribe first so no events are missed
        let internal_rx = self.internal_tx.subscribe();

        let mut tr_id = self.tr_id.lock().await;
        Usr::send(
            &mut tr_id,
            &self.sb_tx,
            &self.internal_tx,
            email,
            &self.cki_string,
        )
        .await?;

        self.listen_to_internal_events(internal_rx).await?;
        Ok(())
    }

    pub(crate) async fn answer(
        &mut self,
        email: String,
        session_id: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Subscribe first so no events are missed
        let internal_rx = self.internal_tx.subscribe();

        let mut tr_id = self.tr_id.lock().await;
        Ans::send(
            &mut tr_id,
            &self.sb_tx,
            &self.internal_tx,
            &email,
            &self.cki_string,
            session_id,
        )
        .await?;

        self.listen_to_internal_events(internal_rx).await?;
        self.session_id = Some(session_id.to_owned());
        Ok(())
    }

    pub async fn invite(&mut self, email: &String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tr_id = self.tr_id.lock().await;
        self.session_id = Some(Cal::send(&mut tr_id, &self.sb_tx, &self.internal_tx, email).await?);
        Ok(())
    }

    pub fn get_session_id(&self) -> Option<String> {
        self.session_id.clone()
    }

    pub async fn send_text_message(
        &self,
        message: &PlainText,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tr_id = self.tr_id.lock().await;
        Msg::send_text_message(&mut tr_id, &self.sb_tx, &self.internal_tx, message).await
    }

    pub async fn send_nudge(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tr_id = self.tr_id.lock().await;
        Msg::send_nudge(&mut tr_id, &self.sb_tx, &self.internal_tx).await
    }

    pub async fn send_typing_user(
        &self,
        email: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tr_id = self.tr_id.lock().await;
        Msg::send_typing_user(&mut tr_id, &self.sb_tx, email).await
    }

    pub async fn request_contact_display_picture(
        &self,
        email: &String,
        msn_object: &String,
    ) -> Result<(), Box<dyn Error>> {
        let mut tr_id = self.tr_id.lock().await;
        let mut session = DisplayPictureSession::new();
        let mut internal_rx = self.internal_tx.subscribe();

        let invite = session.invite(email, &self.user_email, msn_object)?;
        Msg::send_p2p(&mut tr_id, &self.sb_tx, &self.internal_tx, invite, email).await?;

        while let Ok(event) = internal_rx.recv().await {
            if let InternalEvent::P2POk {
                destination,
                message: ok,
            } = event
            {
                if destination != self.user_email {
                    continue;
                }

                let ack = DisplayPictureSession::acknowledge(ok)?;
                Msg::send_p2p(&mut tr_id, &self.sb_tx, &self.internal_tx, ack, email).await?;
                break;
            }
        }

        while let Ok(event) = internal_rx.recv().await {
            if let InternalEvent::P2PDataPreparation {
                destination,
                message: data_preparation,
            } = event
            {
                if destination != self.user_email {
                    continue;
                }

                let ack = DisplayPictureSession::acknowledge(data_preparation)?;
                Msg::send_p2p(&mut tr_id, &self.sb_tx, &self.internal_tx, ack, email).await?;
                break;
            }
        }

        let mut picture: Vec<u8> = Vec::new();
        while let Ok(event) = internal_rx.recv().await {
            if let InternalEvent::P2PData {
                destination,
                message: data,
            } = event
            {
                if destination != self.user_email {
                    continue;
                }

                let binary_header = data[..48].to_vec();
                let mut cursor = Cursor::new(binary_header);
                let (_, binary_header) = BinaryHeader::from_reader((&mut cursor, 0))?;

                picture.extend_from_slice(&data[..(data.len() - 4)]);
                if picture.len() >= binary_header.total_data_size as usize {
                    let ack = DisplayPictureSession::acknowledge(data)?;
                    Msg::send_p2p(&mut tr_id, &self.sb_tx, &self.internal_tx, ack, email).await?;
                    break;
                }
            }
        }

        let bye = session.bye(email, &self.user_email)?;
        Msg::send_p2p(&mut tr_id, &self.sb_tx, &self.internal_tx, bye, email).await?;

        self.event_tx
            .send(Event::DisplayPicture {
                email: email.to_owned(),
                data: picture,
            })
            .await?;

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), SendError<Vec<u8>>> {
        let command = "OUT\r\n";
        trace!("C: {command}");
        self.sb_tx.send(command.as_bytes().to_vec()).await
    }
}
