use crate::commands::chg::Chg;
use crate::commands::cvr::Cvr;
use crate::commands::gcf::Gcf;
use crate::commands::syn::Syn;
use crate::commands::usr_i::UsrI;
use crate::commands::usr_s::UsrS;
use crate::commands::uux::Uux;
use crate::commands::ver::Ver;
use crate::connection_error::ConnectionError;
use crate::event::Event;
use crate::event_matcher::{match_event, match_internal_event};
use crate::internal_event::InternalEvent;
use crate::models::personal_message::PersonalMessage;
use crate::models::presence::Presence;
use crate::msnp_error::MsnpError;
use crate::passport_auth::PassportAuth;
use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use core::str;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::{TcpStream, lookup_host};
use tokio::sync::{broadcast, mpsc};

pub struct Client {
    event_rx: mpsc::Receiver<Event>,
    ns_tx: mpsc::Sender<Vec<u8>>,
    internal_tx: broadcast::Sender<InternalEvent>,
    tr_id: usize,
}

impl Client {
    pub async fn new(server: String, port: String) -> Result<Self, Box<dyn Error>> {
        let server_ip = lookup_host(format!("{server}:{port}"))
            .await?
            .next()
            .ok_or_else(|| ConnectionError::ResolutionError)?
            .ip()
            .to_string();

        let (event_tx, event_rx) = mpsc::channel::<Event>(64);
        let (ns_tx, mut ns_rx) = mpsc::channel::<Vec<u8>>(16);
        let (internal_tx, mut internal_rx) = broadcast::channel::<InternalEvent>(64);

        let socket = TcpStream::connect(format!("{server_ip}:{port}")).await?;
        let (mut rd, mut wr) = socket.into_split();

        let internal_task_tx = internal_tx.clone();
        tokio::spawn(async move {
            while let Ok(base64_messages) = Self::socket_messages_to_base64(&mut rd).await {
                for base64_message in base64_messages {
                    let internal_event = match_internal_event(&base64_message);
                    internal_task_tx
                        .send(internal_event)
                        .expect("Error sending internal event to channel");

                    let event = match_event(&base64_message);
                    event_tx
                        .send(event)
                        .await
                        .expect("Error sending event to channel");
                }
            }
        });

        tokio::spawn(async move {
            while let Some(message) = ns_rx.recv().await {
                wr.write_all(message.as_slice())
                    .await
                    .expect("Error sending message to socket");
            }
        });

        tokio::spawn(async move { while let Ok(event) = internal_rx.recv().await {} });

        Ok(Self {
            event_rx,
            ns_tx,
            internal_tx,
            tr_id: 0,
        })
    }

    pub async fn receive_event(&mut self) -> Result<Event, ConnectionError> {
        self.event_rx
            .recv()
            .await
            .ok_or_else(|| ConnectionError::Disconnected)
    }

    pub fn event_queue_size(&self) -> usize {
        self.event_rx.len()
    }

    pub async fn login(
        &mut self,
        email: String,
        password: String,
        nexus_url: String,
    ) -> Result<Event, Box<dyn Error>> {
        Ver::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx).await?;
        Cvr::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, &email).await?;

        let authorization_string =
            match UsrI::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, &email).await? {
                InternalEvent::GotAuthorizationString(authorization_string) => authorization_string,
                InternalEvent::RedirectedTo { server, port } => {
                    return Ok(Event::RedirectedTo { server, port });
                }

                _ => return Err(MsnpError::CouldNotGetAuthenticationString.into()),
            };

        let auth = PassportAuth::new(nexus_url);
        let token = auth
            .get_passport_token(email, password, authorization_string)
            .await?;

        UsrS::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, &token).await?;
        Syn::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx).await?;
        Gcf::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx).await?;

        Ok(Event::Authenticated)
    }

    pub async fn set_presence(&mut self, presence: &Presence) -> Result<(), Box<dyn Error>> {
        Chg::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, presence).await
    }

    pub async fn set_personal_message(
        &mut self,
        personal_message: &PersonalMessage,
    ) -> Result<(), Box<dyn Error>> {
        Uux::send(
            &mut self.tr_id,
            &self.ns_tx,
            &self.internal_tx,
            personal_message,
        )
        .await
    }

    async fn socket_messages_to_base64(
        rd: &mut OwnedReadHalf,
    ) -> Result<Vec<String>, ConnectionError> {
        let mut buf = vec![0; 1664];
        let received = rd.read(&mut buf).await.unwrap_or_else(|_| 0);

        if received == 0 {
            return Err(ConnectionError::Disconnected);
        }

        let mut messages_bytes = buf[..received].to_vec();
        let mut base64_messages: Vec<String> = Vec::new();

        loop {
            let messages_string = unsafe { str::from_utf8_unchecked(&messages_bytes) };
            let messages: Vec<String> = messages_string
                .lines()
                .map(|line| line.to_string() + "\r\n")
                .collect();

            if messages.len() == 0 {
                break;
            }

            let args: Vec<&str> = messages[0].trim().split(' ').collect();
            match args[0] {
                "GCF" | "UBX" | "MSG" => {
                    let length_index = match args[0] {
                        "UBX" => 2,
                        _ => 3,
                    };

                    let Ok(length) = args[length_index].parse::<usize>() else {
                        continue;
                    };

                    let length = messages[0].len() + length;
                    if length > messages_bytes.len() {
                        let mut buf = vec![0; 1664];
                        let received = rd.read(&mut buf).await.unwrap_or_else(|_| 0);

                        if received == 0 {
                            return Err(ConnectionError::Disconnected);
                        }

                        let mut buf = buf[..received].to_vec();
                        messages_bytes.append(&mut buf);
                        continue;
                    }

                    let new_bytes = messages_bytes[..length].to_vec();
                    messages_bytes = messages_bytes[length..].to_vec();

                    let base64_message = URL_SAFE.encode(&new_bytes);
                    base64_messages.push(base64_message);
                }

                _ => {
                    let new_bytes = messages_bytes[..messages[0].len()].to_vec();
                    messages_bytes = messages_bytes[messages[0].len()..].to_vec();

                    let base64_message = URL_SAFE.encode(&new_bytes);
                    base64_messages.push(base64_message);
                }
            }
        }

        Ok(base64_messages)
    }
}
