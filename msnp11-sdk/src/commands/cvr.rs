use crate::internal_event::InternalEvent;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Cvr;

impl Cvr {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        email: &String,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let command =
            format!("CVR {tr_id} 0x0409 winnt 10 i386 msnp11-sdk 0.01 msmsgs {email}\r\n");
        ns_tx.send(command.as_bytes().to_vec()).await?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            if args[0] == "CVR" && args[1] == tr_id.to_string() {
                break;
            }
        }

        Ok(())
    }
}
