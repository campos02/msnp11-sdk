use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Cal;

impl Cal {
    pub async fn send(
        tr_id: &mut usize,
        sb_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        email: &String,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let command = format!("CAL {tr_id} {email}\r\n");

        sb_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "CAL" => {
                    if args[1] == tr_id.to_string() && args[2] == "RINGING" {
                        return Ok(args[3].to_string());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidContact.into());
                    }
                }

                "215" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidContact.into());
                    }
                }

                "216" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::ContactIsOffline.into());
                    }
                }

                "217" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::ContactIsOffline.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
