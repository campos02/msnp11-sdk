use crate::internal_event::InternalEvent;
use crate::user_data::UserData;
use crate::sdk_error::SdkError;
use crate::switchboard::switchboard::Switchboard;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, mpsc};

pub struct Xfr;

impl Xfr {
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
                match args[0] {
                    "XFR" => {
                        if args[1] == tr_id.to_string() && args[2] == "SB" {
                            let server_and_port = args[3].split(":").collect::<Vec<&str>>();

                            return Switchboard::new(
                                server_and_port[0],
                                server_and_port[1],
                                args[5],
                                user_data,
                            )
                            .await;
                        }
                    }

                    _ => (),
                }
            }
        }
    }
}
