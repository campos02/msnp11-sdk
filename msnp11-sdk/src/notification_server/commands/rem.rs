use crate::internal_event::InternalEvent;
use crate::list::List;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Rem;

impl Rem {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        email: &String,
        list: List,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let list = match list {
            List::ForwardList => return Err(SdkError::InvalidArgument.into()),
            List::AllowList => "AL",
            List::BlockList => "BL",
            List::ReverseList => "RL",
            List::PendingList => "PL",
        };

        let command = format!("REM {tr_id} {list} {email}\r\n");
        ns_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) =
            internal_rx.recv().await.or(Err(SdkError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "REM" => {
                    if args[1] == tr_id.to_string() && args[2] == list && args[3] == email.as_str()
                    {
                        return Ok(());
                    }
                }

                "201" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidContact.into());
                    }
                }

                "216" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "603" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::ServerError.into());
                    }
                }

                _ => (),
            }
        }

        Err(SdkError::Disconnected.into())
    }

    pub async fn send_with_forward_list(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        guid: &String,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let command = format!("REM {tr_id} FL {guid}\r\n");
        ns_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) =
            internal_rx.recv().await.or(Err(SdkError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "REM" => {
                    if args[1] == tr_id.to_string() && args[2] == "FL" && args[3] == guid.as_str() {
                        return Ok(());
                    }
                }

                "201" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidContact.into());
                    }
                }

                "216" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "603" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::ServerError.into());
                    }
                }

                _ => (),
            }
        }

        Err(SdkError::Disconnected.into())
    }

    pub async fn send_with_group(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        guid: &String,
        group_guid: &String,
    ) -> Result<(), SdkError> {
        tr_id.fetch_add(1, Ordering::SeqCst);
        let tr_id = tr_id.load(Ordering::SeqCst);

        let command = format!("REM {tr_id} FL {guid} {group_guid}\r\n");
        ns_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) =
            internal_rx.recv().await.or(Err(SdkError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "REM" => {
                    if args[1] == tr_id.to_string()
                        && args[2] == "FL"
                        && args[3] == guid.as_str()
                        && args[4] == group_guid.as_str()
                    {
                        return Ok(());
                    }
                }

                "201" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidContact.into());
                    }
                }

                "216" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "224" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "225" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument.into());
                    }
                }

                "603" => {
                    if args[1] == tr_id.to_string() {
                        return Err(SdkError::ServerError.into());
                    }
                }

                _ => (),
            }
        }

        Err(SdkError::Disconnected.into())
    }
}
