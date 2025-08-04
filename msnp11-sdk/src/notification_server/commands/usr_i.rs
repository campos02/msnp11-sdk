use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct UsrI;

impl UsrI {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        email: &str,
    ) -> Result<InternalEvent, SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let command = format!("USR {tr_id} TWN I {email}\r\n");
        ns_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        trace!("C: {command}");

        loop {
            if let InternalEvent::ServerReply(reply) =
                internal_rx.recv().await.or(Err(SdkError::ReceivingError))?
            {
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
                                port: server_and_port[1].parse::<u16>().unwrap_or(1863),
                            });
                        }
                    }

                    "500" | "601" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::ServerError);
                        }
                    }

                    "911" | "931" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::ServerIsBusy);
                        }
                    }

                    _ => (),
                }
            }
        }
    }
}
