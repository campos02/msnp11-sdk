use crate::errors::sdk_error::SdkError;
use crate::internal_event::InternalEvent;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub async fn send(
    tr_id: &AtomicU32,
    ns_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    guid: &str,
    new_name: &str,
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

    loop {
        if let InternalEvent::ServerReply(reply) =
            internal_rx.recv().await.or(Err(SdkError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match *args.first().unwrap_or(&"") {
                "REG" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                        && *args.get(3).unwrap_or(&"") == new_name
                        && *args.get(4).unwrap_or(&"") == guid
                    {
                        return Ok(());
                    }
                }

                "224" | "228" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument);
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
