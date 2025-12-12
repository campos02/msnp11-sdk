use crate::enums::event::Event;
use crate::enums::internal_event::InternalEvent;
use crate::enums::msnp_list::MsnpList;
use crate::errors::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub async fn send(
    tr_id: &AtomicU32,
    ns_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    email: &str,
    display_name: &str,
    list: MsnpList,
) -> Result<Event, SdkError> {
    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    if list == MsnpList::ForwardList {
        let encoded_display_name = urlencoding::encode(display_name);
        let command = format!("ADC {tr_id} FL N={email} F={encoded_display_name}\r\n");
        ns_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        trace!("C: {command}");
    } else {
        let list = match list {
            MsnpList::ForwardList => "FL",
            MsnpList::AllowList => "AL",
            MsnpList::BlockList => "BL",
            MsnpList::ReverseList => "RL",
            MsnpList::PendingList => "PL",
        };

        let command = format!("ADC {tr_id} {list} N={email}\r\n");
        ns_tx
            .send(command.as_bytes().to_vec())
            .await
            .or(Err(SdkError::TransmittingError))?;

        trace!("C: {command}");
    }

    loop {
        if let InternalEvent::ServerReply(reply) =
            internal_rx.recv().await.or(Err(SdkError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.split_ascii_whitespace().collect();
            match *args.first().unwrap_or(&"") {
                "ADC" => match list {
                    MsnpList::ForwardList => {
                        if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                            && *args.get(2).unwrap_or(&"") == "FL"
                            && args.get(3).unwrap_or(&"").replace("N=", "") == email
                            && let Some(guid) = args.get(5)
                        {
                            return Ok(Event::ContactInForwardList {
                                email: email.to_owned(),
                                display_name: display_name.to_owned(),
                                guid: guid.replace("C=", ""),
                                lists: vec![MsnpList::ForwardList],
                                groups: vec![],
                            });
                        }
                    }

                    _ => {
                        let list_str = match list {
                            MsnpList::ForwardList => "FL",
                            MsnpList::AllowList => "AL",
                            MsnpList::BlockList => "BL",
                            MsnpList::ReverseList => "RL",
                            MsnpList::PendingList => "PL",
                        };

                        if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                            && *args.get(2).unwrap_or(&"") == list_str
                            && args.get(3).unwrap_or(&"").replace("N=", "") == email
                        {
                            return Ok(Event::Contact {
                                email: email.to_owned(),
                                display_name: display_name.to_owned(),
                                lists: vec![list],
                            });
                        }
                    }
                },

                "201" | "215" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument);
                    }
                }

                "208" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::InvalidContact);
                    }
                }

                "603" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::ServerError);
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
) -> Result<(), SdkError> {
    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("ADC {tr_id} FL C={guid} {group_guid}\r\n");
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
                "ADC" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string()
                        && *args.get(2).unwrap_or(&"") == "FL"
                        && args.get(3).unwrap_or(&"").replace("C=", "") == guid
                        && *args.get(4).unwrap_or(&"") == group_guid
                    {
                        return Ok(());
                    }
                }

                "201" | "215" | "224" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::InvalidArgument);
                    }
                }

                "208" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::InvalidContact);
                    }
                }

                "603" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(SdkError::ServerError);
                    }
                }

                _ => (),
            }
        }
    }
}
