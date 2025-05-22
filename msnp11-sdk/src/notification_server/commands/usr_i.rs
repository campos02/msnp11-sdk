use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct UsrI;

impl UsrI {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        email: &String,
    ) -> Result<InternalEvent, Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let command = format!("USR {tr_id} TWN I {email}\r\n");
        ns_tx.send(command.as_bytes().to_vec()).await?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "USR" => {
                    if args[1] == tr_id.to_string() && args[2] == "TWN" && args[3] == "S" {
                        return Ok(InternalEvent::GotAuthorizationString(args[4].to_string()));
                    }
                }

                "XFR" => {
                    if args[1] == tr_id.to_string() && args[2] == "NS" {
                        let server_and_port: Vec<&str> = args[3].trim().split(':').collect();
                        return Ok(InternalEvent::RedirectedTo {
                            server: server_and_port[0].to_string(),
                            port: server_and_port[1].to_string(),
                        });
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
