use crate::connection_error::ConnectionError;
use crate::internal_event::InternalEvent;
use crate::models::plain_text::PlainText;
use crate::msnp_error::MsnpError;
use log::trace;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct Msg;

impl Msg {
    pub async fn send_text_message(
        tr_id: &mut usize,
        sb_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        message: &PlainText,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut internal_rx = internal_tx.subscribe();

        *tr_id += 1;
        let payload = message.payload();
        let command = format!("MSG {tr_id} A {}\r\n{payload}", payload.len());

        sb_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "ACK" => {
                    if args[1] == tr_id.to_string() {
                        return Ok(());
                    }
                }

                "NAK" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::MessageNotDelivered.into());
                    }
                }

                "282" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::MessageNotDelivered.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }

    pub async fn send_nudge(
        tr_id: &mut usize,
        sb_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut internal_rx = internal_tx.subscribe();

        let mut payload = String::from("MIME-Version: 1.0\r\n");
        payload.push_str("Content-Type: text/x-msnmsgr-datacast\r\n\r\n");
        payload.push_str("ID: 1\r\n\r\n");

        *tr_id += 1;
        let command = format!("MSG {tr_id} A {}\r\n{payload}", payload.len());

        sb_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "ACK" => {
                    if args[1] == tr_id.to_string() {
                        return Ok(());
                    }
                }

                "NAK" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::MessageNotDelivered.into());
                    }
                }

                "282" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::MessageNotDelivered.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }

    pub async fn send_typing_user(
        tr_id: &mut usize,
        sb_tx: &mpsc::Sender<Vec<u8>>,
        email: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut payload = String::from("MIME-Version: 1.0\r\n");
        payload.push_str("Content-Type: text/x-msmsgscontrol\r\n");
        payload.push_str(format!("TypingUser: {email}\r\n\r\n\r\n").as_str());

        *tr_id += 1;
        let command = format!("MSG {tr_id} U {}\r\n{payload}", payload.len());

        sb_tx.send(command.as_bytes().to_vec()).await?;
        trace!("C: {command}");
        Ok(())
    }

    pub async fn send_p2p(
        tr_id: &mut usize,
        sb_tx: &mpsc::Sender<Vec<u8>>,
        internal_tx: &broadcast::Sender<InternalEvent>,
        message: Vec<u8>,
        destination: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut internal_rx = internal_tx.subscribe();

        let mut payload = String::from("MIME-Version: 1.0\r\n");
        payload.push_str("Content-Type: application/x-msnmsgrp2p\r\n");
        payload.push_str(format!("P2P-Dest: {destination}\r\n\r\n").as_str());

        let mut payload = payload.as_bytes().to_vec();
        payload.extend_from_slice(message.as_slice());

        *tr_id += 1;
        let command_string = format!("MSG {tr_id} D {}\r\n", payload.len());

        let mut command = command_string.as_bytes().to_vec();
        command.extend_from_slice(payload.as_slice());

        sb_tx.send(command).await?;
        trace!("C: {command_string}");

        while let InternalEvent::ServerReply(reply) = internal_rx.recv().await? {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.trim().split(' ').collect();
            match args[0] {
                "ACK" => {
                    if args[1] == tr_id.to_string() {
                        return Ok(());
                    }
                }

                "NAK" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::MessageNotDelivered.into());
                    }
                }

                "282" => {
                    if args[1] == tr_id.to_string() {
                        return Err(MsnpError::MessageNotDelivered.into());
                    }
                }

                _ => (),
            }
        }

        Err(ConnectionError::Disconnected.into())
    }
}
