use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::error::Error;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Ans;

impl Ans {
    pub async fn send(
        tr_id: &AtomicU32,
        sb_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        email: &String,
        cki_string: &String,
        session_id: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let command = format!("ANS {tr_id} {email} {cki_string} {session_id}\r\n");
        sb_tx.send(command.as_bytes().to_vec()).await?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) =
            internal_rx.recv().await.or(Err(SdkError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "ANS" => {
                    if args[1] == tr_id.to_string() && args[2] == "OK" {
                        return Ok(());
                    }
                }

                "911" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::ServerIsBusy.into());
                    }
                }

                _ => (),
            }
        }

        Err(SdkError::Disconnected.into())
    }
}
