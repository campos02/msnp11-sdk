use crate::event::Event;
use crate::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
use crate::switchboard::p2p::binary_header::BinaryHeader;
use core::str;
use deku::DekuContainerRead;
use std::io::Cursor;

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
        "MSG" => {
            if !args[1].contains("@") {
                return None;
            }

            let payload = reply.replace(command.as_str(), "");
            let Some(content_type) = payload.lines().nth(1) else {
                return None;
            };

            if content_type.contains("text/plain") {
                return Some(Event::TextMessage {
                    email: args[1].to_string(),
                    message: PlainText::new(payload),
                });
            }

            if content_type.contains("text/x-msnmsgr-datacast") {
                let text = payload.split("\r\n\r\n").nth(1).unwrap_or("");
                if text == "ID: 1" {
                    return Some(Event::Nudge {
                        email: args[1].to_string(),
                    });
                }
            }

            if content_type.contains("text/x-msmsgscontrol") {
                let Some(typing_user) = payload.lines().nth(2) else {
                    return None;
                };

                return Some(Event::TypingNotification {
                    email: typing_user.replace("TypingUser: ", ""),
                });
            }

            None
        }

        "JOI" => Some(Event::ParticipantInSwitchboard {
            email: args[1].to_string(),
        }),

        "IRO" => Some(Event::ParticipantInSwitchboard {
            email: args[4].to_string(),
        }),

        "BYE" => Some(Event::ParticipantLeftSwitchboard {
            email: args[1].to_string(),
        }),

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
                let msg_headers = payload.split("\r\n\r\n").collect::<Vec<&str>>()[0];
                let message =
                    message[(command.len() + msg_headers.len() + "\r\n\r\n".len())..].to_vec();

                let binary_header = &message[..48];
                let mut cursor = Cursor::new(binary_header);
                let Ok((_, binary_header)) = BinaryHeader::from_reader((&mut cursor, 0)) else {
                    return InternalEvent::ServerReply(reply);
                };

                if binary_header.flag == 0x20 {
                    return InternalEvent::P2PData {
                        destination,
                        message: message[..(message.len() - 4)].to_vec(),
                    };
                }

                if binary_header.total_data_size == 4 && message[48..52].eq(&[0; 4]) {
                    return InternalEvent::P2PDataPreparation {
                        destination,
                        message,
                    };
                }

                if payload.contains("INVITE") {
                    return InternalEvent::P2PInvite {
                        destination,
                        message,
                    };
                }

                if payload.contains("200 OK") {
                    return InternalEvent::P2POk {
                        destination,
                        message,
                    };
                }

                if payload.contains("BYE") {
                    return InternalEvent::P2PBye {
                        destination,
                        message,
                    };
                }
            }

            InternalEvent::ServerReply(reply)
        }
        _ => InternalEvent::ServerReply(reply),
    }
}
