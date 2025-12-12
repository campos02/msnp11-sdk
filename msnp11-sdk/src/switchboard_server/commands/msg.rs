use crate::enums::internal_event::InternalEvent;
use crate::errors::messaging_error::MessagingError;
use crate::errors::p2p_error::P2pError;
use crate::models::plain_text::PlainText;
use log::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{broadcast, mpsc};

pub async fn send_text_message(
    tr_id: &AtomicU32,
    sb_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    message: &PlainText,
) -> Result<(), MessagingError> {
    let payload = message.payload();

    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("MSG {tr_id} A {}\r\n{payload}", payload.len());
    sb_tx
        .send(command.as_bytes().to_vec())
        .await
        .or(Err(MessagingError::TransmittingError))?;

    trace!("C: {command}");

    loop {
        if let InternalEvent::ServerReply(reply) = internal_rx
            .recv()
            .await
            .or(Err(MessagingError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.split_ascii_whitespace().collect();
            match *args.first().unwrap_or(&"") {
                "ACK" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Ok(());
                    }
                }

                "NAK" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(MessagingError::MessageNotDelivered);
                    }
                }

                "282" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(MessagingError::MessageNotDelivered);
                    }
                }

                _ => (),
            }
        }
    }
}

pub async fn send_nudge(
    tr_id: &AtomicU32,
    sb_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
) -> Result<(), MessagingError> {
    let mut payload = String::from("MIME-Version: 1.0\r\n");
    payload.push_str("Content-Type: text/x-msnmsgr-datacast\r\n\r\n");
    payload.push_str("ID: 1\r\n\r\n");

    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("MSG {tr_id} A {}\r\n{payload}", payload.len());
    sb_tx
        .send(command.as_bytes().to_vec())
        .await
        .or(Err(MessagingError::TransmittingError))?;

    trace!("C: {command}");

    loop {
        if let InternalEvent::ServerReply(reply) = internal_rx
            .recv()
            .await
            .or(Err(MessagingError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.split_ascii_whitespace().collect();
            match *args.first().unwrap_or(&"") {
                "ACK" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Ok(());
                    }
                }

                "NAK" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(MessagingError::MessageNotDelivered);
                    }
                }

                "282" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(MessagingError::MessageNotDelivered);
                    }
                }

                _ => (),
            }
        }
    }
}

pub async fn send_typing_user(
    tr_id: &AtomicU32,
    sb_tx: &mpsc::Sender<Vec<u8>>,
    email: &str,
) -> Result<(), MessagingError> {
    let mut payload = String::from("MIME-Version: 1.0\r\n");
    payload.push_str("Content-Type: text/x-msmsgscontrol\r\n");
    payload.push_str(format!("TypingUser: {email}\r\n\r\n\r\n").as_str());

    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command = format!("MSG {tr_id} U {}\r\n{payload}", payload.len());
    sb_tx
        .send(command.as_bytes().to_vec())
        .await
        .or(Err(MessagingError::TransmittingError))?;

    trace!("C: {command}");

    Ok(())
}

pub async fn send_p2p(
    tr_id: &AtomicU32,
    sb_tx: &mpsc::Sender<Vec<u8>>,
    internal_rx: &mut broadcast::Receiver<InternalEvent>,
    message: Vec<u8>,
    destination: &str,
) -> Result<(), P2pError> {
    let mut payload = String::from("MIME-Version: 1.0\r\n");
    payload.push_str("Content-Type: application/x-msnmsgrp2p\r\n");
    payload.push_str(format!("P2P-Dest: {destination}\r\n\r\n").as_str());

    let mut payload = payload.as_bytes().to_vec();
    payload.extend_from_slice(message.as_slice());

    tr_id.fetch_add(1, Ordering::SeqCst);
    let tr_id = tr_id.load(Ordering::SeqCst);

    let command_string = format!("MSG {tr_id} D {}\r\n", payload.len());
    let mut command = command_string.as_bytes().to_vec();
    command.extend_from_slice(payload.as_slice());

    sb_tx
        .send(command)
        .await
        .or(Err(P2pError::TransmittingError))?;

    trace!("C: {command_string}");

    loop {
        if let InternalEvent::ServerReply(reply) =
            internal_rx.recv().await.or(Err(P2pError::ReceivingError))?
        {
            trace!("S: {reply}");

            let args: Vec<&str> = reply.split_ascii_whitespace().collect();
            match *args.first().unwrap_or(&"") {
                "ACK" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Ok(());
                    }
                }

                "NAK" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(P2pError::MessageNotDelivered);
                    }
                }

                "282" => {
                    if *args.get(1).unwrap_or(&"") == tr_id.to_string() {
                        return Err(P2pError::MessageNotDelivered);
                    }
                }

                _ => (),
            }
        }
    }
}
