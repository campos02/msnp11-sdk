use crate::connection_error::ConnectionError;
use crate::event::Event;
use crate::internal_event::InternalEvent;
use crate::list::List;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Adc;

impl Adc {
    pub async fn send(
        tr_id: &mut usize,
        ns_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        email: &String,
        display_name: &String,
        list: List,
    ) -> Result<Event, Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();
        let encoded_display_name = urlencoding::encode(display_name).to_string();

        if list == List::ForwardList {
            *tr_id += 1;
            let command = format!("ADC {tr_id} FL N={email} F={encoded_display_name}\r\n");

            ns_tx.send(command.as_bytes().to_vec()).await?;
            trace!("C: {command}");
        } else {
            let list = match list {
                List::ForwardList => "FL",
                List::AllowList => "AL",
                List::BlockList => "BL",
                List::ReverseList => "RL",
                List::PendingList => "PL",
            };

            *tr_id += 1;
            let command = format!("ADC {tr_id} {list} N={email}\r\n");

            ns_tx.send(command.as_bytes().to_vec()).await?;
            trace!("C: {command}");
        }

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "ADC" => match list {
                    List::ForwardList => {
                        if args[1] == tr_id.to_string()
                            && args[2] == "FL"
                            && args[3].replace("N=", "") == email.as_str()
                        {
                            return Ok(Event::ContactInForwardList {
                                email: email.to_owned(),
                                display_name: display_name.to_owned(),
                                guid: args[5].replace("C=", ""),
                                lists: vec![List::ForwardList],
                                groups: vec![],
                            });
                        }
                    }

                    _ => {
                        let list_str = match list {
                            List::ForwardList => "FL",
                            List::AllowList => "AL",
                            List::BlockList => "BL",
                            List::ReverseList => "RL",
                            List::PendingList => "PL",
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
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidContact.into());
                    }
                }

                "215" => {
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
        let command = format!("ADC {tr_id} FL C={guid} {group_guid}\r\n");

        ns_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
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
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "208" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidContact.into());
                    }
                }

                "215" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::InvalidArgument.into());
                    }
                }

                "224" => {
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
