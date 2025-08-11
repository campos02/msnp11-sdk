use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use crate::switchboard_server::switchboard::Switchboard;
use crate::user_data::UserData;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, mpsc};

pub async fn send(
    tr_id: &AtomicU32,
    ns_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    user_data: Arc<RwLock<UserData>>,
) -> Result<Switchboard, SdkError> {
    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("XFR {tr_id} SB\r\n");
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
            if *args.first().unwrap_or(&"") == "XFR"
                && *args.get(1).unwrap_or(&"") == tr_id.to_string()
                && *args.get(2).unwrap_or(&"") == "SB"
                && let Some(cki_string) = args.get(5)
            {
                let mut server_and_port = args.get(3).unwrap_or(&"").split(":");
                if let Some(server) = server_and_port.next()
                    && let Some(port) = server_and_port.next()
                {
                    return Switchboard::new(server, port, cki_string, user_data).await;
                }
            }
        }
    }
}
