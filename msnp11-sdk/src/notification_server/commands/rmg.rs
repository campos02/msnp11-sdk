use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Rmg;

impl Rmg {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        guid: &String,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let command = format!("RMG {tr_id} {guid}\r\n");
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
                    "RMG" => {
                        if args[1] == tr_id.to_string() && args[3] == guid {
                            return Ok(());
                        }
                    }

                    "224" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::InvalidArgument);
                        }
                    }

                    "226" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::InvalidArgument);
                        }
                    }

                    "230" => {
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
