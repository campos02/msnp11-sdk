use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::models::personal_message::PersonalMessage;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Uux;

impl Uux {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        personal_message: &PersonalMessage,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();
        let personal_message = quick_xml::se::to_string(personal_message)?;

        *tr_id += 1;
        let command = format!(
            "UUX {tr_id} {}\r\n{personal_message}",
            personal_message.len()
        );

        ns_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "UUX" => {
                    if args[1] == tr_id.to_string() {
                        return Ok(());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
