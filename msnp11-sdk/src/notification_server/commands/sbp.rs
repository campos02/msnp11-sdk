use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Sbp;

impl Sbp {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        guid: &String,
        display_name: &String,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();
        let display_name = urlencoding::encode(display_name).to_string();

        *tr_id += 1;
        let command = format!("SBP {tr_id} {guid} MFN {display_name}\r\n");
        ns_tx.send(command.as_bytes().to_vec()).await?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "SBP" => {
                    if args[1] == tr_id.to_string()
                        && args[2] == guid
                        && args[3] == "MFN"
                        && args[4] == display_name
                    {
                        return Ok(());
                    }
                }

                "201" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidContact.into());
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
