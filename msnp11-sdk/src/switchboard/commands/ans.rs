use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Ans;

impl Ans {
    pub async fn send(
        tr_id: &mut usize,
        sb_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        email: &String,
        cki_string: &String,
        session_id: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let command = format!("ANS {tr_id} {email} {cki_string} {session_id}\r\n");

        sb_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "ANS" => {
                    if args[1] == tr_id.to_string() && args[2] == "OK" {
                        return Ok(());
                    }
                }

                "911" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::ServerIsBusy.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
