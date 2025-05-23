use crate::client::Client;
use crate::event::Event;
use crate::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
use crate::msnp_error::MsnpError;
use crate::switchboard::commands::ans::Ans;
use crate::switchboard::commands::cal::Cal;
use crate::switchboard::commands::msg::Msg;
use crate::switchboard::commands::usr::Usr;
use crate::switchboard::event_matcher::{into_event, into_internal_event};
use log::trace;
use std::collections::HashSet;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{broadcast, mpsc};

pub struct Switchboard {
    event_tx: mpsc::Sender<Event>,
    sb_tx: mpsc::Sender<Vec<u8>>,
    internal_tx: broadcast::Sender<InternalEvent>,
    tr_id: usize,
    session_id: Option<String>,
    cki_string: String,
    participants: Arc<Mutex<HashSet<String>>>,
}

impl Switchboard {
    pub async fn new(
        server: &str,
        port: &str,
        cki_string: &str,
        event_tx: mpsc::Sender<Event>,
        internal_tx: broadcast::Sender<InternalEvent>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (sb_tx, mut sb_rx) = mpsc::channel::<Vec<u8>>(16);
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
            sb_tx,
            internal_tx,
            tr_id: 0,
            session_id: None,
            cki_string: cki_string.to_string(),
            participants: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    fn listen_to_internal_events(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut internal_rx = self.internal_tx.subscribe();
        let session_id = self.session_id.clone().ok_or(MsnpError::NotLoggedIn)?;
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            while let Ok(event) = internal_rx.recv().await {
                if let InternalEvent::ServerReply(reply) = event {
                    let event = into_event(&reply, &session_id);
                    event_tx
                        .send(event)
                        .await
                        .expect("Error sending event to channel");
                }
            }
        });

        Ok(())
    }

    pub async fn login(&mut self, email: &String) -> Result<(), Box<dyn Error + Send + Sync>> {
        Usr::send(
            &mut self.tr_id,
            &self.sb_tx,
            &self.internal_tx,
            email,
            &self.cki_string,
        )
        .await?;

        self.participants
            .lock()
            .or(Err(MsnpError::CouldNotGetParticipants))?
            .insert(email.to_owned());

        Ok(())
    }

    pub async fn answer(
        &mut self,
        email: String,
        session_id: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ans::send(
            &mut self.tr_id,
            &self.sb_tx,
            &self.internal_tx,
            &email,
            &self.cki_string,
            session_id,
        )
        .await?;

        self.session_id = Some(session_id.to_owned());
        self.listen_to_internal_events()?;

        self.participants
            .lock()
            .or(Err(MsnpError::CouldNotGetParticipants))?
            .insert(email.to_owned());

        Ok(())
    }

    pub async fn invite(&mut self, email: &String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let had_session_id = self.session_id.is_some();
        self.session_id =
            Some(Cal::send(&mut self.tr_id, &self.sb_tx, &self.internal_tx, email).await?);

        self.participants
            .lock()
            .or(Err(MsnpError::CouldNotGetParticipants))?
            .insert(email.to_owned());

        if !had_session_id {
            self.listen_to_internal_events()?;
        }

        Ok(())
    }

    pub fn get_session_id(&self) -> Option<String> {
        self.session_id.clone()
    }

    pub fn get_participants(&self) -> Result<HashSet<String>, Box<dyn Error + Send + Sync>> {
        Ok(self
            .participants
            .lock()
            .or(Err(MsnpError::CouldNotGetParticipants))?
            .clone())
    }

    pub async fn send_text_message(
        &mut self,
        message: &PlainText,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Msg::send_text_message(&mut self.tr_id, &self.sb_tx, &self.internal_tx, message).await
    }

    pub async fn send_nudge(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Msg::send_nudge(&mut self.tr_id, &self.sb_tx, &self.internal_tx).await
    }

    pub async fn send_typing_user(
        &mut self,
        email: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Msg::send_typing_user(&mut self.tr_id, &self.sb_tx, email).await
    }

    pub async fn disconnect(&self) -> Result<(), SendError<Vec<u8>>> {
        let command = "OUT\r\n";
        trace!("C: {command}");

        self.sb_tx.send(command.as_bytes().to_vec()).await
    }
}
