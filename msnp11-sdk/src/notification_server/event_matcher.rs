use crate::enums::event::Event;
use crate::enums::msnp_list::MsnpList;
use crate::enums::msnp_status::MsnpStatus;
use crate::internal_event::InternalEvent;
use crate::models::personal_message::PersonalMessage;
use crate::models::presence::Presence;
use core::str;
use std::borrow::Cow;

pub fn into_event(message: &Vec<u8>) -> Option<Event> {
    let reply = unsafe { str::from_utf8_unchecked(message.as_slice()) };
    let command = reply.lines().next().unwrap_or_default().to_string() + "\r\n";

    let args: Vec<&str> = command.trim().split(' ').collect();
    match *args.first().unwrap_or(&"") {
        "GTC" => {
            if args.len() < 3
                && let Some(gtc) = args.get(1)
            {
                Some(Event::Gtc(gtc.to_string()))
            } else {
                None
            }
        }

        "BLP" => {
            if args.len() < 3
                && let Some(blp) = args.get(1)
            {
                Some(Event::Blp(blp.to_string()))
            } else {
                None
            }
        }

        "PRP" => {
            if args.len() < 4
                && *args.get(1).unwrap_or(&"") == "MFN"
                && let Some(display_name) = args.get(2)
            {
                Some(Event::DisplayName(
                    urlencoding::decode(display_name)
                        .unwrap_or(Cow::from(*display_name))
                        .to_string(),
                ))
            } else {
                None
            }
        }

        "LSG" => {
            if let Some(email) = args.get(1)
                && let Some(guid) = args.get(2)
            {
                Some(Event::Group {
                    name: urlencoding::decode(email)
                        .unwrap_or(Cow::from(*email))
                        .to_string(),
                    guid: guid.to_string(),
                })
            } else {
                None
            }
        }

        "LST" => {
            let mut lists: Vec<MsnpList> = Vec::new();
            let lists_number_index = if args.len() > 4 { 4 } else { 3 };

            let lists_number = args
                .get(lists_number_index)
                .unwrap_or(&"")
                .parse::<u32>()
                .ok()?;
            let email = args.get(1)?;

            if lists_number & 1 == 1 {
                lists.push(MsnpList::ForwardList);
            }

            if lists_number & 2 == 2 {
                lists.push(MsnpList::AllowList);
            }

            if lists_number & 4 == 4 {
                lists.push(MsnpList::BlockList);
            }

            if lists_number & 8 == 8 {
                lists.push(MsnpList::ReverseList);
            }

            if lists_number & 16 == 16 {
                lists.push(MsnpList::PendingList);
            }

            if lists_number & 1 == 1
                && let Some(guid) = args.get(3)
            {
                let mut groups: Vec<String> = Vec::new();
                if args.len() > 5 {
                    groups = args[5].split(",").map(|id| id.to_string()).collect();
                }

                Some(Event::ContactInForwardList {
                    email: email.replace("N=", ""),
                    display_name: urlencoding::decode(
                        &args.get(2).unwrap_or(email).replace("F=", ""),
                    )
                    .unwrap_or(Cow::from(*email))
                    .to_string(),
                    guid: guid.replace("C=", ""),
                    groups,
                    lists,
                })
            } else {
                Some(Event::Contact {
                    email: email.replace("N=", ""),
                    display_name: urlencoding::decode(
                        args.get(2).unwrap_or(email).replace("F=", "").as_str(),
                    )
                    .unwrap_or(Cow::from(*email))
                    .to_string(),
                    lists,
                })
            }
        }

        "ILN" => {
            let email = args.get(3)?;
            let msn_object = if args.len() > 6 {
                urlencoding::decode(args[6]).ok().map(String::from)
            } else {
                None
            };

            let status = match *args.get(2)? {
                "BSY" => MsnpStatus::Busy,
                "AWY" => MsnpStatus::Away,
                "IDL" => MsnpStatus::Idle,
                "LUN" => MsnpStatus::OutToLunch,
                "PHN" => MsnpStatus::OnThePhone,
                "BRB" => MsnpStatus::BeRightBack,
                _ => MsnpStatus::Online,
            };

            Some(Event::InitialPresenceUpdate {
                email: email.to_string(),
                display_name: urlencoding::decode(args.get(4).unwrap_or(email))
                    .unwrap_or(Cow::from(*email))
                    .to_string(),
                presence: Presence {
                    status,
                    client_id: args.get(5).unwrap_or(&"").parse().unwrap_or(0),
                    msn_object: if let Some(msn_object) = &msn_object {
                        quick_xml::de::from_str(msn_object).ok()
                    } else {
                        None
                    },
                    msn_object_string: msn_object,
                },
            })
        }

        "NLN" => {
            let email = args.get(2)?;
            let msn_object = if args.len() > 5 {
                urlencoding::decode(args[5]).ok().map(String::from)
            } else {
                None
            };

            let status = match *args.get(1)? {
                "BSY" => MsnpStatus::Busy,
                "AWY" => MsnpStatus::Away,
                "IDL" => MsnpStatus::Idle,
                "LUN" => MsnpStatus::OutToLunch,
                "PHN" => MsnpStatus::OnThePhone,
                "BRB" => MsnpStatus::BeRightBack,
                _ => MsnpStatus::Online,
            };

            Some(Event::PresenceUpdate {
                email: email.to_string(),
                display_name: urlencoding::decode(args.get(3).unwrap_or(email))
                    .unwrap_or(Cow::from(*email))
                    .to_string(),
                presence: Presence {
                    status,
                    client_id: args.get(4).unwrap_or(&"").parse().unwrap_or(0),
                    msn_object: if let Some(msn_object) = &msn_object {
                        quick_xml::de::from_str(msn_object).ok()
                    } else {
                        None
                    },
                    msn_object_string: msn_object,
                },
            })
        }

        "UBX" => {
            let email = args.get(1)?;

            let payload = reply.replace(command.as_str(), "");
            let personal_message =
                quick_xml::de::from_str(payload.as_str()).unwrap_or(PersonalMessage {
                    psm: "".to_string(),
                    current_media: "".to_string(),
                });

            Some(Event::PersonalMessageUpdate {
                email: email.to_string(),
                personal_message,
            })
        }

        "FLN" => args.get(1).map(|email| Event::ContactOffline {
            email: email.to_string(),
        }),

        "ADC" => {
            if *args.get(1).unwrap_or(&"") == "0"
                && *args.get(2).unwrap_or(&"") == "RL"
                && let Some(email) = args.get(3)
                && let Some(display_name) = args.get(4)
            {
                Some(Event::AddedBy {
                    email: email.replace("N=", ""),
                    display_name: urlencoding::decode(display_name.replace("F=", "").as_str())
                        .unwrap_or(Cow::from(*email))
                        .to_string(),
                })
            } else {
                None
            }
        }

        "REM" => {
            if *args.get(1).unwrap_or(&"") == "0"
                && *args.get(2).unwrap_or(&"") == "RL"
                && let Some(email) = args.get(3)
            {
                Some(Event::RemovedBy(email.replace("N=", "")))
            } else {
                None
            }
        }

        "MSG" => {
            let payload = reply.replace(&command, "");
            let mut payload_lines = payload.lines();
            let content_type = payload_lines.nth(1)?;

            if content_type.contains("application/x-msmsgssystemmessage") {
                let message_type = payload_lines.nth(1)?;
                let (_, message_type) = message_type.split_once(":")?;
                let message_type = message_type.trim();

                if message_type == "1" {
                    let time_remaining = payload_lines.next()?;
                    let (_, time_remaining) = time_remaining.split_once(":")?;

                    let time_remaining = time_remaining.trim().parse::<u32>().ok()?;
                    return Some(Event::ServerMaintenance { time_remaining });
                }
            }

            None
        }

        "OUT" => {
            if args.len() > 1 && *args.get(1).unwrap_or(&"") == "OTH" {
                return Some(Event::LoggedInAnotherDevice);
            }

            Some(Event::Disconnected)
        }

        _ => None,
    }
}

pub fn into_internal_event(message: &Vec<u8>) -> InternalEvent {
    let reply = unsafe { str::from_utf8_unchecked(message.as_slice()) }.to_string();
    let command = reply.lines().next().unwrap_or_default().to_string() + "\r\n";

    let args: Vec<&str> = command.trim().split(' ').collect();
    match *args.first().unwrap_or(&"") {
        "RNG" => {
            let mut server_and_port = args.get(2).unwrap_or(&"").split(":");
            if let Some(server) = server_and_port.next()
                && let Some(port) = server_and_port.next()
                && let Some(session_id) = args.get(1)
                && let Some(cki_string) = args.get(4)
            {
                InternalEvent::SwitchboardInvitation {
                    server: server.to_string(),
                    port: port.to_string(),
                    session_id: session_id.to_string(),
                    cki_string: cki_string.to_string(),
                }
            } else {
                InternalEvent::ServerReply(reply)
            }
        }

        _ => InternalEvent::ServerReply(reply),
    }
}
