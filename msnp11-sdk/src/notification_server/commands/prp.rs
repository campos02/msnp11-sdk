use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Prp;

impl Prp {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        display_name: &String,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let display_name = urlencoding::encode(display_name);
        let command = format!("PRP {tr_id} MFN {display_name}\r\n");
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
                "PRP" => {
                    if args[1] == tr_id.to_string() && args[2] == "MFN" && args[3] == display_name {
                        return Ok(());
                    }
                }

                _ => (),
            }
        }

        Err(SdkError::Disconnected.into())
    }
}
