use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Adg;

impl Adg {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        name: &str,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let group_name = urlencoding::encode(name);
        let command = format!("ADG {tr_id} {group_name}\r\n");
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
                    "ADG" => {
                        if args[1] == tr_id.to_string() && args[3] == group_name {
                            return Ok(());
                        }
                    }

                    "228" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::InvalidArgument);
                        }
                    }

                    "603" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::ServerError);
                        }
                    }

                    _ => (),
                }
            }
        }
    }
}
