use crate::enums::event::Event;
use crate::enums::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use core::str;
use deku::DekuContainerRead;
use std::io::Cursor;

pub fn into_event(message: &Vec<u8>) -> Option<Event> {
    let reply = unsafe { str::from_utf8_unchecked(message.as_slice()) };
    let command = reply.lines().next().unwrap_or_default().to_string() + "\r\n";

    let args: Vec<&str> = command.split_ascii_whitespace().collect();
    match *args.first().unwrap_or(&"") {
        "MSG" => {
            let payload = reply.replace(command.as_str(), "");
            let content_type = payload.lines().nth(1)?;

            if content_type.contains("text/plain")
                && let Some(email) = args.get(1)
            {
                return Some(Event::TextMessage {
                    email: email.to_string(),
                    message: PlainText::new(payload),
                });
            }

            if content_type.contains("text/x-msnmsgr-datacast") {
                let text = payload.split("\r\n\r\n").nth(1).unwrap_or_default();
                if text == "ID: 1"
                    && let Some(email) = args.get(1)
                {
                    return Some(Event::Nudge {
                        email: email.to_string(),
                    });
                }
            }

            if content_type.contains("text/x-msmsgscontrol") {
                let typing_user = payload.lines().nth(2)?;
                return Some(Event::TypingNotification {
                    email: typing_user.replace("TypingUser: ", ""),
                });
            }

            None
        }

        "JOI" => args.get(1).map(|email| Event::ParticipantInSwitchboard {
            email: email.to_string(),
        }),

        "IRO" => args.get(4).map(|email| Event::ParticipantInSwitchboard {
            email: email.to_string(),
        }),

        "BYE" => args.get(1).map(|email| Event::ParticipantLeftSwitchboard {
            email: email.to_string(),
        }),

        _ => None,
    }
}

pub fn into_internal_event(message: &Vec<u8>) -> InternalEvent {
    let reply = unsafe { str::from_utf8_unchecked(message.as_slice()) }.to_string();
    let command = reply.lines().next().unwrap_or_default().to_string() + "\r\n";

    let args: Vec<&str> = command.split_ascii_whitespace().collect();
    match *args.first().unwrap_or(&"") {
        "MSG" => {
            let payload = reply.replace(command.as_str(), "");
            let Some(content_type) = payload.lines().nth(1) else {
                return InternalEvent::ServerReply(reply);
            };

            if content_type == "Content-Type: application/x-msnmsgrp2p" {
                let Some(destination) = payload.lines().find(|line| line.contains("P2P-Dest: "))
                else {
                    return InternalEvent::ServerReply(reply);
                };

                let destination = destination.replace("P2P-Dest: ", "");
                let msg_headers = payload.split("\r\n\r\n").next().unwrap_or_default();

                let binary_payload = message
                    .get((command.len() + msg_headers.len() + "\r\n\r\n".len())..)
                    .unwrap_or_default()
                    .to_vec();

                if binary_payload.len() < 52 {
                    return InternalEvent::ServerReply(reply);
                }

                let binary_header = &binary_payload[..48];
                let mut cursor = Cursor::new(binary_header);
                let Ok((_, binary_header)) = BinaryHeader::from_reader((&mut cursor, 0)) else {
                    return InternalEvent::ServerReply(reply);
                };

                if binary_header.total_data_size == 4 && binary_payload[48..52].eq(&[0; 4])
                    || payload.contains("200 OK")
                {
                    return InternalEvent::P2PShouldAck {
                        destination,
                        message: binary_payload,
                    };
                }

                if binary_header.flag == 0x20 || binary_header.flag == 0x1000020 {
                    return InternalEvent::P2PData {
                        destination,
                        message: binary_payload[..(binary_payload.len() - 4)].to_vec(),
                    };
                }

                if payload.contains("INVITE") {
                    return InternalEvent::P2PInvite {
                        destination,
                        message: binary_payload,
                    };
                }

                if payload.contains("BYE") {
                    return InternalEvent::P2PBye {
                        destination,
                        message: binary_payload,
                    };
                }
            }

            InternalEvent::ServerReply(reply)
        }
        _ => InternalEvent::ServerReply(reply),
    }
}
