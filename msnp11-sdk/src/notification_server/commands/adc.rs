use crate::enums::event::Event;
use crate::enums::msnp_list::MsnpList;
use crate::internal_event::InternalEvent;
use crate::sdk_error::SdkError;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub struct Adc;

impl Adc {
    pub async fn send(
        tr_id: &AtomicU32,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_rx: &mut broadcast::Receiver<InternalEvent>,
        email: &String,
        display_name: &String,
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

                let args: Vec<&str> = reply.trim().split(' ').collect();
                match args[0] {
                    "ADC" => match list {
                        MsnpList::ForwardList => {
                            if args[1] == tr_id.to_string()
                                && args[2] == "FL"
                                && args[3].replace("N=", "") == email.as_str()
                            {
                                return Ok(Event::ContactInForwardList {
                                    email: email.to_owned(),
                                    display_name: display_name.to_owned(),
                                    guid: args[5].replace("C=", ""),
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

                            if args[1] == tr_id.to_string()
                                && args[2] == list_str
                                && args[3].replace("N=", "") == email.as_str()
                            {
                                return Ok(Event::Contact {
                                    email: email.to_owned(),
                                    display_name: display_name.to_owned(),
                                    lists: vec![list],
                                });
                            }
                        }
                    },

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

                    "215" => {
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
        }
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

                let args: Vec<&str> = reply.trim().split(' ').collect();
                match args[0] {
                    "ADC" => {
                        if args[1] == tr_id.to_string()
                            && args[2] == "FL"
                            && args[3].replace("C=", "") == guid.as_str()
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

                    "215" => {
                        if args[1] == tr_id.to_string() {
                            return Err(SdkError::InvalidArgument.into());
                        }
                    }

                    "224" => {
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
        }
    }
}
