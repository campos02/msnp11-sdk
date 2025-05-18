use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::models::presence::Presence;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Chg;

impl Chg {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        presence: &Presence,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let mut command = format!(
            "CHG {tr_id} {} {}\r\n",
            presence.presence, presence.client_id
        );
        if let Some(msn_object) = &presence.msn_object {
            command = command.replace("\r\n", format!(" {msn_object}\r\n").as_str());
        }

        ns_tx.send(command.as_bytes().to_vec()).await?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "CHG" => {
                    if args[1] == tr_id.to_string() {
                        return Ok(());
                    }
                }

                "201" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
