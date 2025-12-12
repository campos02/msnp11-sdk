use crate::enums::internal_event::InternalEvent;
use crate::enums::msnp_list::MsnpList;
use crate::errors::contact_error::ContactError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub async fn send(
    tr_id: &AtomicU32,
    ns_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    email: &str,
    list: MsnpList,
) -> Result<(), ContactError> {
    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let list = match list {
        MsnpList::ForwardList => return Err(ContactError::InvalidArgument),
        MsnpList::AllowList => "AL",
        MsnpList::BlockList => "BL",
        MsnpList::ReverseList => "RL",
        MsnpList::PendingList => "PL",
    };

    let command = format!("REM {tr_id} {list} {email}\r\n");
    ns_tx
        .send(command.as_bytes().to_vec())
        .await
        .or(Err(ContactError::TransmittingError))?;

    trace!("C: {command}");

    loop {
        if let InternalEvent::ServerReply(reply) = internal_rx
            .recv()
            .await
            .or(Err(ContactError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.split_ascii_whitespace().collect();
            match *args.first().unwrap_or(&"") {
                "REM" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                        && *args.get(2).unwrap_or(&"") == list
                        && *args.get(3).unwrap_or(&"") == email
                    {
                        return Ok(());
                    }
                }

                "201" | "216" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::InvalidArgument);
                    }
                }

                "208" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::InvalidContact);
                    }
                }

                "603" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::ServerError);
                    }
                }

                _ => (),
            }
        }
    }
}

pub async fn send_with_forward_list(
    tr_id: &AtomicU32,
    ns_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    guid: &str,
) -> Result<(), ContactError> {
    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("REM {tr_id} FL {guid}\r\n");
    ns_tx
        .send(command.as_bytes().to_vec())
        .await
        .or(Err(ContactError::TransmittingError))?;

    trace!("C: {command}");

    loop {
        if let InternalEvent::ServerReply(reply) = internal_rx
            .recv()
            .await
            .or(Err(ContactError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.split_ascii_whitespace().collect();
            match *args.first().unwrap_or(&"") {
                "REM" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                        && *args.get(2).unwrap_or(&"") == "FL"
                        && *args.get(3).unwrap_or(&"") == guid
                    {
                        return Ok(());
                    }
                }

                "201" | "216" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::InvalidArgument);
                    }
                }

                "208" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::InvalidContact);
                    }
                }

                "603" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::ServerError);
                    }
                }

                _ => (),
            }
        }
    }
}

pub async fn send_with_group(
    tr_id: &AtomicU32,
    ns_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    guid: &str,
    group_guid: &str,
) -> Result<(), ContactError> {
    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("REM {tr_id} FL {guid} {group_guid}\r\n");
    ns_tx
        .send(command.as_bytes().to_vec())
        .await
        .or(Err(ContactError::TransmittingError))?;

    trace!("C: {command}");

    loop {
        if let InternalEvent::ServerReply(reply) = internal_rx
            .recv()
            .await
            .or(Err(ContactError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.split_ascii_whitespace().collect();
            match *args.first().unwrap_or(&"") {
                "REM" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                        && *args.get(2).unwrap_or(&"") == "FL"
                        && *args.get(3).unwrap_or(&"") == guid
                        && *args.get(4).unwrap_or(&"") == group_guid
                    {
                        return Ok(());
                    }
                }

                "201" | "216" | "224" | "225" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::InvalidArgument);
                    }
                }

                "208" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::InvalidContact);
                    }
                }

                "603" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(ContactError::ServerError);
                    }
                }

                _ => (),
            }
        }
    }
}
