use crate::internal_event::InternalEvent;
use crate::models::personal_message::PersonalMessage;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Uux;

impl Uux {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        personal_message: &PersonalMessage,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let personal_message =
            quick_xml::se::to_string(personal_message).or(Err(SdkError::CouldNotSetUserData))?;
        let command = format!(
            "UUX {tr_id} {}\r\n{personal_message}",
            personal_message.len()
        );

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
                if *args.first().unwrap_or(&"") == "UUX"
                    && *args.get(1).unwrap_or(&"") == tr_id.to_string()
                {
                    return Ok(());
                }
            }
        }
    }
}
