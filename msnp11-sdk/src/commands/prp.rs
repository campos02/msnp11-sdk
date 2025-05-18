use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Prp;

impl Prp {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        display_name: &String,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();
        let display_name = urlencoding::encode(display_name).to_string();

        *tr_id += 1;
        let command = format!("PRP {tr_id} MFN {display_name}\r\n");
        ns_tx.send(command.as_bytes().to_vec()).await?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "PRP" => {
                    if args[1] == tr_id.to_string() && args[2] == "MFN" && args[3] == display_name {
                        return Ok(());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
