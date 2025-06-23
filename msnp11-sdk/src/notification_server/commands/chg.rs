use crate::internal_event::InternalEvent;
use crate::models::presence::Presence;
use crate::msnp_status::MsnpStatus;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Chg;

impl Chg {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        presence: &Presence,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let status = match presence.status {
            MsnpStatus::Online => "NLN",
            MsnpStatus::Busy => "BSY",
            MsnpStatus::Away => "AWY",
            MsnpStatus::Idle => "IDL",
            MsnpStatus::OutToLunch => "LUN",
            MsnpStatus::OnThePhone => "PHN",
            MsnpStatus::BeRightBack => "BRB",
            MsnpStatus::AppearOffline => "HDN",
        };

        let mut command = format!("CHG {tr_id} {status} {}\r\n", presence.client_id);

        if let Some(msn_object) = &presence.msn_object {
            command = command.replace(
                "\r\n",
                format!(" {}\r\n", urlencoding::encode(msn_object).to_string()).as_str(),
            );
        }

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
                    "CHG" => {
                        if args[1] == tr_id.to_string() {
                            return Ok(());
                        }
                    }

                    "201" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::InvalidArgument.into());
                        }
                    }

                    _ => (),
                }
            }
        }
    }
}
