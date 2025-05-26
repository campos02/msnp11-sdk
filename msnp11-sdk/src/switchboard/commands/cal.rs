use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Cal;

impl Cal {
    pub async fn send(
        tr_id: &AtomicU32,
        sb_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        email: &String,
    ) -> Result<String, SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let command = format!("CAL {tr_id} {email}\r\n");
        sb_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) =
            internal_rx.recv().await.or(Err(SdkError::ReceivingError))?
        {
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
                        return Err(SdkError::InvalidContact);
                    }
                }

                "215" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidContact);
                    }
                }

                "216" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::ContactIsOffline);
                    }
                }

                "217" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::ContactIsOffline);
                    }
                }

                _ => (),
            }
        }

        Err(SdkError::Disconnected)
    }
}
