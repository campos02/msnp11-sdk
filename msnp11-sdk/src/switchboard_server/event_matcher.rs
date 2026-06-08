use crate::enums::event::Event;
use crate::enums::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use crate::switchboard_server::p2p::file_context::FileContext;
#[cfg(feature = "file-transfers")]
use base64::{Engine as _, engine::general_purpose::STANDARD};
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

pub fn into_internal_event(message: &[u8]) -> InternalEvent {
    let reply = unsafe { str::from_utf8_unchecked(message) }.to_string();
    let command = reply.lines().next().unwrap_or_default().to_string() + "\r\n";

    let mut args = command.split_ascii_whitespace();
    match args.next().unwrap_or("") {
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

                if binary_header.total_data_size == 4 && binary_payload[48..52].eq(&[0; 4]) {
                    return InternalEvent::P2pShouldAck {
                        destination,
                        message: binary_payload,
                    };
                }

                if binary_header.flag == 0x20
                    || binary_header.flag == 0x1000020
                    || binary_header.flag == 0x1000030
                {
                    return InternalEvent::P2pData {
                        destination,
                        message: binary_payload[..(binary_payload.len() - 4)].to_vec(),
                    };
                }

                #[cfg(feature = "file-transfers")]
                if payload.contains("200 OK") {
                    if payload.contains("application/x-msnmsgr-transrespbody") {
                        let lines = payload.lines();

                        let mut bridge = None;
                        let mut listening = None;
                        let mut nonce = None;
                        let mut ips = None;
                        let mut port = None;

                        for line in lines {
                            if line.contains("Bridge: ") {
                                bridge = Some(line.replace("Bridge: ", ""));
                            } else if line.contains("Listening: ") {
                                listening = Some(line.contains("true"));
                            } else if line.contains("Nonce: ") {
                                nonce = Some(line.replace("Nonce: {", "").replace("}", ""));
                            } else if line.contains("IPv6-Addrs: ") {
                                ips = Some(
                                    line.replace("IPv6-Addrs: ", "")
                                        .split(" ")
                                        .map(|s| s.to_string())
                                        .collect(),
                                );
                            } else if line.contains("IPv6-Port: ") {
                                port = line.replace("IPv6-Port: ", "").parse::<u16>().ok();
                            } else if line.contains("IPv4Internal-Addrs: ") && ips.is_none() {
                                ips = Some(
                                    line.replace("IPv4Internal-Addrs: ", "")
                                        .split(" ")
                                        .map(|s| s.to_string())
                                        .collect(),
                                );
                            } else if line.contains("IPv4Internal-Port: ") && port.is_none() {
                                port = line.replace("IPv4Internal-Port: ", "").parse::<u16>().ok();
                            }
                        }

                        if let Some(bridge) = bridge
                            && let Some(listening) = listening
                            && let Some(nonce) = nonce
                            && let Ok(nonce) = guid_create::GUID::parse(&nonce)
                            && let Some(ips) = ips
                            && let Some(port) = port
                        {
                            return InternalEvent::P2pDirectConnectionOk {
                                destination,
                                message: binary_payload,
                                bridge,
                                listening,
                                nonce,
                                ips,
                                port,
                            };
                        }
                    }

                    return InternalEvent::P2pOk {
                        destination,
                        message: binary_payload,
                    };
                }

                #[cfg(feature = "file-transfers")]
                if payload.contains("603 Decline") {
                    return InternalEvent::P2pDecline {
                        destination,
                        message: binary_payload,
                    };
                }

                if payload.contains("INVITE") {
                    let lines = payload.lines();

                    let mut to = None;
                    let mut from = None;
                    let mut branch = None;
                    let mut call_id = None;
                    let mut content_type = None;
                    let mut euf_guid = None;
                    let mut session_id = None;
                    let mut context = None;

                    for line in lines {
                        if line.contains("To: ") {
                            to = Some(line.replace("To: <msnmsgr:", "").replace(">", ""));
                        } else if line.contains("From: ") {
                            from = Some(line.replace("From: <msnmsgr:", "").replace(">", ""));
                        } else if line.contains("Via: MSNSLP/1.0/TLP ;branch={") {
                            branch = guid_create::GUID::parse(
                                &line
                                    .replace("Via: MSNSLP/1.0/TLP ;branch={", "")
                                    .replace("}", ""),
                            )
                            .ok();
                        } else if line.contains("Call-ID: {") {
                            call_id = guid_create::GUID::parse(
                                &line.replace("Call-ID: {", "").replace("}", ""),
                            )
                            .ok();
                        } else if line.contains("Content-Type: ") {
                            content_type = Some(line.replace("Content-Type: ", ""));
                        } else if line.contains("EUF-GUID: ") {
                            euf_guid = Some(line.replace("EUF-GUID: ", ""));
                        } else if line.contains("SessionID: ") {
                            session_id = line.replace("SessionID: ", "").parse::<u32>().ok();
                        } else if line.contains("Context: ") {
                            context =
                                Some(line.replace("Context: ", "").trim_matches('\0').to_string());
                        }
                    }

                    if let Some(to) = to
                        && let Some(from) = from
                        && let Some(branch) = branch
                        && let Some(call_id) = call_id
                        && let Some(content_type) = content_type
                    {
                        match content_type.as_str() {
                            "application/x-msnmsgr-sessionreqbody" => {
                                if let Some(euf_guid) = euf_guid
                                    && let Some(session_id) = session_id
                                    && let Some(mut context) = context
                                {
                                    match euf_guid.as_str() {
                                        "{A4268EEC-FEC5-49E5-95C3-F126696BDBF6}" => {
                                            return InternalEvent::DisplayPictureInvite {
                                                to,
                                                from,
                                                branch,
                                                call_id,
                                                session_id,
                                                context,
                                                message: binary_payload,
                                            };
                                        }

                                        #[cfg(feature = "file-transfers")]
                                        "{5D3E02AB-6190-11D3-BBBB-00C04F795683}" => {
                                            // Padding
                                            while context.len() % 4 != 0 {
                                                context.push('=');
                                            }

                                            let Ok(context) = STANDARD.decode(context) else {
                                                return InternalEvent::ServerReply(reply);
                                            };

                                            let Some((file_info, file_name)) =
                                                context.split_at_checked(20)
                                            else {
                                                return InternalEvent::ServerReply(reply);
                                            };

                                            let mut cursor = Cursor::new(file_info);
                                            let Ok((_, context)) =
                                                FileContext::from_reader((&mut cursor, 0))
                                            else {
                                                return InternalEvent::ServerReply(reply);
                                            };

                                            let mut utf16_file_name =
                                                Vec::with_capacity(file_name.len() / 2);

                                            let mut even_byte = 0;
                                            for (i, byte) in file_name.iter().enumerate() {
                                                if i % 2 != 0 {
                                                    if even_byte == 0 && *byte == 0 {
                                                        break;
                                                    }

                                                    utf16_file_name.push(u16::from_le_bytes([
                                                        even_byte, *byte,
                                                    ]));
                                                } else {
                                                    even_byte = *byte;
                                                }
                                            }

                                            let file_name =
                                                String::from_utf16_lossy(&utf16_file_name);

                                            return InternalEvent::FileTransferInvite {
                                                to,
                                                from,
                                                branch,
                                                call_id,
                                                session_id,
                                                file_size: context.file_size,
                                                file_name,
                                                message: binary_payload,
                                            };
                                        }

                                        _ => (),
                                    }
                                }
                            }

                            #[cfg(feature = "file-transfers")]
                            "application/x-msnmsgr-transreqbody" => {
                                return InternalEvent::P2pDirectConnectionInvite {
                                    to,
                                    branch,
                                    call_id,
                                    message: binary_payload,
                                };
                            }

                            _ => (),
                        }
                    }
                }

                if payload.contains("BYE") {
                    let lines = payload.lines();

                    let mut to = None;
                    let mut from = None;

                    for line in lines {
                        if line.contains("To: ") {
                            to = Some(line.replace("To: <msnmsgr:", "").replace(">", ""));
                        } else if line.contains("From: ") {
                            from = Some(line.replace("From: <msnmsgr:", "").replace(">", ""));
                            break;
                        }
                    }

                    if let Some(to) = to
                        && let Some(from) = from
                    {
                        return InternalEvent::P2pBye {
                            to,
                            from,
                            message: binary_payload,
                        };
                    }
                }
            }

            InternalEvent::ServerReply(reply)
        }

        _ => InternalEvent::ServerReply(reply),
    }
}
