use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Sbp;

impl Sbp {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        guid: &str,
        display_name: &str,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let display_name = urlencoding::encode(display_name);
        let command = format!("SBP {tr_id} {guid} MFN {display_name}\r\n");
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
                    "SBP" => {
                        if args[1] == tr_id.to_string()
                            && args[2] == guid
                            && args[3] == "MFN"
                            && args[4] == display_name
                        {
                            return Ok(());
                        }
                    }

                    "201" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::InvalidArgument);
                        }
                    }

                    "208" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::InvalidContact);
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
