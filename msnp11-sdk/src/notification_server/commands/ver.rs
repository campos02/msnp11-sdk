use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Ver;

impl Ver {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let command = format!("VER {tr_id} MSNP11 CVR0\r\n");
        ns_tx.send(command.as_bytes().to_vec()).await?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "VER" => {
                    if args[1] == tr_id.to_string() {
                        return if args[2] != "MSNP11" {
                            Err(MsnpError::ProtocolNotSupported.into())
                        } else {
                            Ok(())
                        };
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
