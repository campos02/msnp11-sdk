use crate::event::Event;
use crate::internal_event::InternalEvent;
use crate::list::List;
use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use core::str;

pub fn match_event(base64_message: &String) -> Event {
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
            id: args[2].to_string(),
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
                    let _ = args[5].split(",").map(|id| groups.push(id.to_string()));
                }

                Event::ContactInForwardList {
                    email: args[1].replace("N=", ""),
                    display_name: urlencoding::decode(args[2].replace("F=", "").as_str())
                        .expect("Could not url decode contact name")
                        .to_string(),
                    id: args[3].replace("C=", ""),
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

        _ => Event::ServerReply,
    }
}

pub fn match_internal_event(base64_message: &String) -> InternalEvent {
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
        _ => InternalEvent::ServerReply(reply),
    }
}
