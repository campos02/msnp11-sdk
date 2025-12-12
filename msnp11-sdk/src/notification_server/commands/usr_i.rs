use crate::enums::internal_event::InternalEvent;
use crate::errors::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub async fn send(
    tr_id: &AtomicU32,
    ns_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    email: &str,
) -> Result<InternalEvent, SdkError> {
    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("USR {tr_id} TWN I {email}\r\n");
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

            let args: Vec<&str> = reply.split_ascii_whitespace().collect();
            match *args.first().unwrap_or(&"") {
                "USR" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                        && *args.get(2).unwrap_or(&"") == "TWN"
                        && *args.get(3).unwrap_or(&"") == "S"
                        && let Some(authorization_string) = args.get(4)
                    {
                        return Ok(InternalEvent::GotAuthorizationString(
                            authorization_string.to_string(),
                        ));
                    }
                }

                "XFR" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                        && *args.get(2).unwrap_or(&"") == "NS"
                    {
                        let mut server_and_port = args.get(3).unwrap_or(&"").split(":");
                        if let Some(server) = server_and_port.next()
                            && let Some(port) = server_and_port.next()
                        {
                            return Ok(InternalEvent::RedirectedTo {
                                server: server.to_string(),
                                port: port.parse::<u16>().unwrap_or(1863),
                            });
                        }
                    }
                }

                "500" | "601" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::ServerError);
                    }
                }

                "911" | "931" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::ServerIsBusy);
                    }
                }

                _ => (),
            }
        }
    }
}
