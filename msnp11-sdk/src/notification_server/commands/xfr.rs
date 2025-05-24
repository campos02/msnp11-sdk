use crate::connection_error::ConnectionError;
use crate::event::Event;
use crate::internal_event::InternalEvent;
use crate::switchboard::switchboard::Switchboard;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Xfr;

impl Xfr {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        event_tx: &mpsc::Sender<Event>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        display_picture: &Option<Vec<u8>>,
        msn_object: &Option<String>,
        user_email: &String,
    ) -> Result<Switchboard, Box<dyn Error + Send + Sync>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let command = format!("XFR {tr_id} SB\r\n");
        ns_tx.send(command.as_bytes().to_vec()).await?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "XFR" => {
                    if args[1] == tr_id.to_string() && args[2] == "SB" {
                        let server_and_port = args[3].split(":").collect::<Vec<&str>>();

                        return Switchboard::new(
                            server_and_port[0],
                            server_and_port[1],
                            args[5],
                            event_tx.clone(),
                            internal_tx.clone(),
                            display_picture.clone(),
                            msn_object.clone(),
                            user_email.clone(),
                        )
                        .await;
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
