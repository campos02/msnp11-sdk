use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::list::List;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Rem;

impl Rem {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        email: &String,
        list: List,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();

        let list = match list {
            List::ForwardList => return Err(MsnpError::InvalidArgument.into()),
            List::AllowList => "AL",
            List::BlockList => "BL",
            List::ReverseList => "RL",
            List::PendingList => "PL",
        };

        *tr_id += 1;
        let command = format!("REM {tr_id} {list} {email}\r\n");

        ns_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
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
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidContact.into());
                    }
                }

                "216" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "603" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::ServerError.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }

    pub async fn send_with_forward_list(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        guid: &String,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let command = format!("REM {tr_id} FL {guid}\r\n");

        ns_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
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
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidContact.into());
                    }
                }

                "216" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "603" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::ServerError.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }

    pub async fn send_with_group(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        guid: &String,
        group_guid: &String,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let command = format!("REM {tr_id} FL {guid} {group_guid}\r\n");

        ns_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
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
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidContact.into());
                    }
                }

                "216" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "224" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "225" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "603" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::ServerError.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
