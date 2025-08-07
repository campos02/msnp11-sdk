use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Syn;

impl Syn {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let command = format!("SYN {tr_id} 0 0\r\n");
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
                match *args.first().unwrap_or(&"") {
                    "SYN" => {
                        if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                            return Ok(());
                        }
                    }

                    "603" => {
                        if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                            return Err(SdkError::ServerError);
                        }
                    }

                    _ => (),
                }
            }
        }
    }
}
