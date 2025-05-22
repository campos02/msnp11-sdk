use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Adg;

impl Adg {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        name: &String,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();
        let group_name = urlencoding::encode(name).to_string();

        *tr_id += 1;
        let command = format!("ADG {tr_id} {group_name}\r\n");

        ns_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "ADG" => {
                    if args[1] == tr_id.to_string() && args[3] == group_name {
                        return Ok(());
                    }
                }

                "228" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "603" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::ServerError.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
