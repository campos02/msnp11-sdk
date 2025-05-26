use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Reg;

impl Reg {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        guid: &String,
        new_name: &String,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let new_name = urlencoding::encode(new_name);
        let command = format!("REG {tr_id} {guid} {new_name}\r\n");
        ns_tx
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
                "REG" => {
                    if args[1] == tr_id.to_string() && args[3] == new_name && args[4] == guid {
                        return Ok(());
                    }
                }

                "224" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "228" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "603" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::ServerError.into());
                    }
                }

                _ => (),
            }
        }

        Err(SdkError::Disconnected.into())
    }
}
