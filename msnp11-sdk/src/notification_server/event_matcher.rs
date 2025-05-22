use crate::event::Event;
use crate::internal_event::InternalEvent;
use crate::list::List;
use crate::models::personal_message::PersonalMessage;
use crate::models::presence::Presence;
use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use core::str;

pub fn into_event(message: &String) -> Event {
    let command = message
        .lines()
        .next()
        .expect("Could not get command from server message")
        .to_string()
        + "\r\n";

    let args: Vec<&str> = command.trim().split(' ').collect();
    match args[0] {
        "GTC" => {
            let gtc = if args.len() > 2 { args[2] } else { args[1] };
            Event::Gtc(gtc.to_string())
        }

        "BLP" => {
            let blp = if args.len() > 2 { args[2] } else { args[1] };
            Event::Blp(blp.to_string())
        }

        "PRP" => {
            let display_name = if args.len() > 2 { args[2] } else { args[1] };
            Event::DisplayName(
                urlencoding::decode(display_name)
                    .expect("Could not url decode display name")
                    .to_string(),
            )
        }

        "LSG" => Event::Group {
            name: urlencoding::decode(args[1])
                .expect("Could not url decode group name")
                .to_string(),
            guid: args[2].to_string(),
        },

        "LST" => {
            let mut lists: Vec<List> = Vec::new();
            let lists_number_index = if args.len() > 4 { 4 } else { 3 };

            let lists_number = args[lists_number_index]
                .parse::<u32>()
                .expect("Found invalid list number");

            if lists_number & 1 == 1 {
                lists.push(List::ForwardList);
            }

            if lists_number & 2 == 2 {
                lists.push(List::AllowList);
            }

            if lists_number & 4 == 4 {
                lists.push(List::BlockList);
            }

            if lists_number & 8 == 8 {
                lists.push(List::ReverseList);
            }

            if lists_number & 16 == 16 {
                lists.push(List::PendingList);
            }

            if lists_number & 1 == 1 {
                let mut groups: Vec<String> = Vec::new();
                if args.len() > 5 {
                    groups = args[5].split(",").map(|id| id.to_string()).collect();
                }

                Event::ContactInForwardList {
                    email: args[1].replace("N=", ""),
                    display_name: urlencoding::decode(args[2].replace("F=", "").as_str())
                        .expect("Could not url decode contact name")
                        .to_string(),
                    guid: args[3].replace("C=", ""),
                    groups,
                    lists,
                }
            } else {
                Event::Contact {
                    email: args[1].replace("N=", ""),
                    display_name: urlencoding::decode(args[2].replace("F=", "").as_str())
                        .expect("Could not url decode contact name")
                        .to_string(),
                    lists,
                }
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

            Event::PresenceUpdate {
                email: args[base_index + 2].to_string(),
                display_name: urlencoding::decode(args[base_index + 3])
                    .expect("Could not url decode contact name")
                    .to_string(),
                presence: Presence {
                    presence: args[base_index + 1].to_string(),
                    client_id: args[base_index + 4].to_string().parse().unwrap_or(0),
                    msn_object,
                },
            }
        }

        "UBX" => {
            let payload = message.replace(command.as_str(), "");
            let personal_message =
                quick_xml::de::from_str(payload.as_str()).unwrap_or(PersonalMessage {
                    psm: "".to_string(),
                    current_media: "".to_string(),
                });

            Event::PersonalMessageUpdate {
                email: args[1].to_string(),
                personal_message,
            }
        }

        "FLN" => Event::ContactOffline {
            email: args[1].to_string(),
        },

        "ADC" => {
            if args[1] == "0" && args[2] == "RL" {
                Event::AddedBy {
                    email: args[3].replace("N=", ""),
                    display_name: urlencoding::decode(args[4].replace("F=", "").as_str())
                        .expect("Could not url decode contact display name")
                        .to_string(),
                }
            } else {
                Event::ServerReply
            }
        }

        "REM" => {
            if args[1] == "0" && args[2] == "RL" {
                Event::RemovedBy(args[3].replace("N=", ""))
            } else {
                Event::ServerReply
            }
        }

        "OUT" => {
            if args.len() > 1 {
                if args[1] == "OTH" {
                    return Event::LoggedInAnotherDevice;
                }
            }

            Event::Disconnected
        }

        _ => Event::ServerReply,
    }
}

pub fn into_internal_event(base64_message: &String) -> InternalEvent {
    let message_bytes = URL_SAFE
        .decode(base64_message)
        .expect("Could not decode socket message from base64");

    let reply = unsafe { str::from_utf8_unchecked(message_bytes.as_slice()) }.to_string();
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
