use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct UsrS;

impl UsrS {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        token: &String,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let command = format!("USR {tr_id} TWN S {token}\r\n");
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
                    "USR" => {
                        if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                            && *args.get(2).unwrap_or(&"") == "OK"
                        {
                            return Ok(());
                        }
                    }

                    "500" | "910" | "921" => {
                        if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                            return Err(SdkError::ServerError);
                        }
                    }

                    "911" | "923" | "928" => {
                        if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                            return Err(SdkError::ServerIsBusy);
                        }
                    }

                    _ => (),
                }
            }
        }
    }
}
