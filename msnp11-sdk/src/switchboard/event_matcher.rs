use crate::event::Event;
use crate::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use core::str;

pub fn into_event(message: &String, session_id: &String) -> Event {
    let command = message
        .lines()
        .next()
        .expect("Could not get command from server message")
        .to_string()
        + "\r\n";

    let args: Vec<&str> = command.trim().split(' ').collect();
    match args[0] {
        "MSG" => {
            let payload = message.replace(command.as_str(), "");
            let Some(content_type) = payload.lines().nth(1) else {
                return Event::ServerReply;
            };

            if content_type.contains("text/plain") {
                return Event::TextMessage {
                    session_id: session_id.to_owned(),
                    email: args[1].to_string(),
                    message: PlainText::new(payload),
                };
            }

            if content_type.contains("text/x-msnmsgr-datacast") {
                let text = payload.split("\r\n\r\n").nth(1).unwrap_or("");

                if text == "ID: 1" {
                    return Event::Nudge {
                        session_id: session_id.to_owned(),
                        email: args[1].to_string(),
                    };
                }
            }

            if content_type.contains("text/x-msmsgscontrol") {
                let Some(typing_user) = payload.lines().nth(2) else {
                    return Event::ServerReply;
                };

                return Event::TypingNotification {
                    session_id: session_id.to_owned(),
                    email: typing_user.replace("TypingUser: ", ""),
                };
            }

            Event::ServerReply
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
        "MSG" => InternalEvent::ServerReply(reply),
        _ => InternalEvent::ServerReply(reply),
    }
}
