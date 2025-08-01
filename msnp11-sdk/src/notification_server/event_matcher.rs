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
    let command = reply
        .lines()
        .next()
        .expect("Could not get command from server message")
        .to_string()
        + "\r\n";

    let args: Vec<&str> = command.trim().split(' ').collect();
    match args[0] {
        "GTC" => {
            if args.len() < 3 {
                Some(Event::Gtc(args[1].to_string()))
            } else {
                None
            }
        }

        "BLP" => {
            if args.len() < 3 {
                Some(Event::Blp(args[1].to_string()))
            } else {
                None
            }
        }

        "PRP" => {
            if args.len() < 4 && args[1] == "MFN" {
                Some(Event::DisplayName(
                    urlencoding::decode(args[2])
                        .expect("Could not url decode display name")
                        .to_string(),
                ))
            } else {
                None
            }
        }

        "LSG" => Some(Event::Group {
            name: urlencoding::decode(args[1])
                .expect("Could not url decode group name")
                .to_string(),
            guid: args[2].to_string(),
        }),

        "LST" => {
            let mut lists: Vec<MsnpList> = Vec::new();
            let lists_number_index = if args.len() > 4 { 4 } else { 3 };

            let lists_number = args[lists_number_index]
                .parse::<u32>()
                .expect("Found invalid list number");

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

            if lists_number & 1 == 1 {
                let mut groups: Vec<String> = Vec::new();
                if args.len() > 5 {
                    groups = args[5].split(",").map(|id| id.to_string()).collect();
                }

                Some(Event::ContactInForwardList {
                    email: args[1].replace("N=", ""),
                    display_name: urlencoding::decode(args[2].replace("F=", "").as_str())
                        .expect("Could not url decode contact name")
                        .to_string(),
                    guid: args[3].replace("C=", ""),
                    groups,
                    lists,
                })
            } else {
                Some(Event::Contact {
                    email: args[1].replace("N=", ""),
                    display_name: urlencoding::decode(args[2].replace("F=", "").as_str())
                        .expect("Could not url decode contact name")
                        .to_string(),
                    lists,
                })
            }
        }

        "NLN" | "ILN" => {
            let base_index = if args[0] == "ILN" { 1 } else { 0 };
            let msn_object = if args[0] == "ILN" && args.len() > 6 {
                Some(
                    urlencoding::decode(args[6])
                        .expect("Could not url decode MSN object")
                        .to_string(),
                )
            } else if args[0] == "NLN" && args.len() > 5 {
                Some(
                    urlencoding::decode(args[5])
                        .expect("Could not url decode MSN object")
                        .to_string(),
                )
            } else {
                None
            };

            let status = match args[base_index + 1] {
                "BSY" => MsnpStatus::Busy,
                "AWY" => MsnpStatus::Away,
                "IDL" => MsnpStatus::Idle,
                "LUN" => MsnpStatus::OutToLunch,
                "PHN" => MsnpStatus::OnThePhone,
                "BRB" => MsnpStatus::BeRightBack,
                _ => MsnpStatus::Online,
            };

            Some(Event::PresenceUpdate {
                email: args[base_index + 2].to_string(),
                display_name: urlencoding::decode(args[base_index + 3])
                    .unwrap_or(Cow::from(args[base_index + 2]))
                    .to_string(),
                presence: Presence {
                    status,
                    client_id: args[base_index + 4].to_string().parse().unwrap_or(0),
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
            let payload = reply.replace(command.as_str(), "");
            let personal_message =
                quick_xml::de::from_str(payload.as_str()).unwrap_or(PersonalMessage {
                    psm: "".to_string(),
                    current_media: "".to_string(),
                });

            Some(Event::PersonalMessageUpdate {
                email: args[1].to_string(),
                personal_message,
            })
        }

        "FLN" => Some(Event::ContactOffline {
            email: args[1].to_string(),
        }),

        "ADC" => {
            if args[1] == "0" && args[2] == "RL" {
                Some(Event::AddedBy {
                    email: args[3].replace("N=", ""),
                    display_name: urlencoding::decode(args[4].replace("F=", "").as_str())
                        .expect("Could not url decode contact display name")
                        .to_string(),
                })
            } else {
                None
            }
        }

        "REM" => {
            if args[1] == "0" && args[2] == "RL" {
                Some(Event::RemovedBy(args[3].replace("N=", "")))
            } else {
                None
            }
        }

        "OUT" => {
            if args.len() > 1 && args[1] == "OTH" {
                return Some(Event::LoggedInAnotherDevice);
            }

            Some(Event::Disconnected)
        }

        _ => None,
    }
}

pub fn into_internal_event(message: &Vec<u8>) -> InternalEvent {
    let reply = unsafe { str::from_utf8_unchecked(message.as_slice()) }.to_string();
    let command = reply
        .lines()
        .next()
        .expect("Could not get command from server message")
        .to_string()
        + "\r\n";

    let args: Vec<&str> = command.trim().split(' ').collect();
    match args[0] {
        "RNG" => {
            let server_and_port: Vec<&str> = args[2].trim().split(':').collect();

            InternalEvent::SwitchboardInvitation {
                server: server_and_port[0].to_string(),
                port: server_and_port[1].to_string(),
                session_id: args[1].to_string(),
                cki_string: args[4].to_string(),
            }
        }

        _ => InternalEvent::ServerReply(reply),
    }
}
