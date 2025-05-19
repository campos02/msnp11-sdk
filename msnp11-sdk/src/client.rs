use crate::commands::adc::Adc;
use crate::commands::adg::Adg;
use crate::commands::blp::Blp;
use crate::commands::chg::Chg;
use crate::commands::cvr::Cvr;
use crate::commands::gcf::Gcf;
use crate::commands::gtc::Gtc;
use crate::commands::prp::Prp;
use crate::commands::reg::Reg;
use crate::commands::rem::Rem;
use crate::commands::rmg::Rmg;
use crate::commands::sbp::Sbp;
use crate::commands::syn::Syn;
use crate::commands::usr_i::UsrI;
use crate::commands::usr_s::UsrS;
use crate::commands::uux::Uux;
use crate::commands::ver::Ver;
use crate::connection_error::ConnectionError;
use crate::event::Event;
use crate::event_matcher::{match_event, match_internal_event};
use crate::internal_event::InternalEvent;
use crate::list::List;
use crate::models::personal_message::PersonalMessage;
use crate::models::presence::Presence;
use crate::msnp_error::MsnpError;
use crate::passport_auth::PassportAuth;
use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use core::str;
use log::trace;
use std::error::Error;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::{TcpStream, lookup_host};
use tokio::sync::{broadcast, mpsc};
use tokio::sync::mpsc::error::SendError;

pub struct Client {
    event_tx: mpsc::Sender<Event>,
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

        let event_task_tx = event_tx.clone();
        let internal_task_tx = internal_tx.clone();
        tokio::spawn(async move {
            while let Ok(base64_messages) = Self::socket_messages_to_base64(&mut rd).await {
                for base64_message in base64_messages {
                    let internal_event = match_internal_event(&base64_message);
                    internal_task_tx
                        .send(internal_event)
                        .expect("Error sending internal event to channel");

                    let event = match_event(&base64_message);
                    event_task_tx
                        .send(event)
                        .await
                        .expect("Error sending event to channel");
                }
            }

            event_task_tx
                .send(Event::Disconnected)
                .await
                .expect("Error sending disconnection event");
        });

        tokio::spawn(async move {
            while let Some(message) = ns_rx.recv().await {
                wr.write_all(message.as_slice())
                    .await
                    .expect("Error sending message to socket");
            }
        });

        tokio::spawn(async move { while let Ok(_) = internal_rx.recv().await {} });

        Ok(Self {
            event_tx,
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

        self.start_pinging();
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

    pub async fn set_display_name(&mut self, display_name: &String) -> Result<(), Box<dyn Error>> {
        Prp::send(
            &mut self.tr_id,
            &self.ns_tx,
            &self.internal_tx,
            display_name,
        )
        .await
    }

    pub async fn set_contact_display_name(
        &mut self,
        guid: &String,
        display_name: &String,
    ) -> Result<(), Box<dyn Error>> {
        Sbp::send(
            &mut self.tr_id,
            &self.ns_tx,
            &self.internal_tx,
            guid,
            display_name,
        )
        .await
    }

    pub async fn add_contact(
        &mut self,
        email: &String,
        display_name: &String,
        list: List,
    ) -> Result<Event, Box<dyn Error>> {
        Adc::send(
            &mut self.tr_id,
            &self.ns_tx,
            &self.internal_tx,
            email,
            display_name,
            list,
        )
        .await
    }

    pub async fn remove_contact(
        &mut self,
        email: &String,
        list: List,
    ) -> Result<(), Box<dyn Error>> {
        Rem::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, email, list).await
    }

    pub async fn remove_contact_from_forward_list(
        &mut self,
        guid: &String,
    ) -> Result<(), Box<dyn Error>> {
        Rem::send_with_forward_list(&mut self.tr_id, &self.ns_tx, &self.internal_tx, guid).await
    }

    pub async fn create_group(&mut self, name: &String) -> Result<(), Box<dyn Error>> {
        Adg::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, name).await
    }

    pub async fn delete_group(&mut self, guid: &String) -> Result<(), Box<dyn Error>> {
        Rmg::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, guid).await
    }

    pub async fn rename_group(
        &mut self,
        guid: &String,
        new_name: &String,
    ) -> Result<(), Box<dyn Error>> {
        Reg::send(
            &mut self.tr_id,
            &self.ns_tx,
            &self.internal_tx,
            guid,
            new_name,
        )
        .await
    }

    pub async fn add_contact_to_group(
        &mut self,
        guid: &String,
        group_guid: &String,
    ) -> Result<(), Box<dyn Error>> {
        Adc::send_with_group(
            &mut self.tr_id,
            &self.ns_tx,
            &self.internal_tx,
            guid,
            group_guid,
        )
        .await
    }

    pub async fn remove_contact_from_group(
        &mut self,
        guid: &String,
        group_guid: &String,
    ) -> Result<(), Box<dyn Error>> {
        Rem::send_with_group(
            &mut self.tr_id,
            &self.ns_tx,
            &self.internal_tx,
            guid,
            group_guid,
        )
        .await
    }

    pub async fn set_gtc(&mut self, gtc: &String) -> Result<(), Box<dyn Error>> {
        Gtc::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, gtc).await
    }

    pub async fn set_blp(&mut self, blp: &String) -> Result<(), Box<dyn Error>> {
        Blp::send(&mut self.tr_id, &self.ns_tx, &self.internal_tx, blp).await
    }
    
    pub async fn disconnect(&self) -> Result<(), SendError<Vec<u8>>> {
        let command = "OUT\r\n";
        trace!("C: {command}");
        
        self.ns_tx.send(command.as_bytes().to_vec()).await
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

    fn start_pinging(&self) {
        let event_tx = self.event_tx.clone();
        let ns_tx = self.ns_tx.clone();
        let mut internal_rx = self.internal_tx.subscribe();

        tokio::spawn(async move {
            let command = "PNG\r\n";

            while ns_tx.send(command.as_bytes().to_vec()).await.is_ok() {
                trace!("C: {command}");

                while let Ok(InternalEvent::ServerReply(reply)) = internal_rx.recv().await {
                    trace!("S: {reply}");

                    let args: Vec<&str> = reply.trim().split(' ').collect();
                    match args[0] {
                        "QNG" => {
                            let duration = args[1].parse().unwrap_or_else(|_| 50);
                            tokio::time::sleep(Duration::from_secs(duration)).await;

                            break;
                        }

                        _ => (),
                    }
                }
            }

            event_tx
                .send(Event::Disconnected)
                .await
                .expect("Error sending disconnection event");
        });
    }
}
