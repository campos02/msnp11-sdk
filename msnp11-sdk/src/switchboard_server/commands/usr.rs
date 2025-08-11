use crate::errors::sdk_error::SdkError;
use crate::internal_event::InternalEvent;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub async fn send(
    tr_id: &AtomicU32,
    sb_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    email: &str,
    cki_string: &str,
) -> Result<(), SdkError> {
    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("USR {tr_id} {email} {cki_string}\r\n");
    sb_tx
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

                "911" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::ServerIsBusy);
                    }
                }

                _ => (),
            }
        }
    }
}
