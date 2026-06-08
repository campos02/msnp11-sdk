use crate::P2pError;
use crate::enums::internal_event::InternalEvent;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use crate::switchboard_server::p2p::p2p_session::P2pSession;
use deku::{DekuContainerRead, DekuContainerWrite};
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

pub async fn listen_for_file(
    socket: &mut TcpStream,
    nonce: guid_create::GUID,
    identifier: Arc<AtomicU32>,
    internal_tx: broadcast::Sender<InternalEvent>,
) {
    let mut length_buf = vec![0; 4];
    'receiving: loop {
        // Read foo\0
        let Ok(_) = read(socket, &mut length_buf).await else {
            break;
        };

        let Ok(message) = read(socket, &mut length_buf).await else {
            break;
        };

        if let Some(binary_header) = message.get(..48) {
            let mut cursor = Cursor::new(binary_header);
            if let Ok((_, binary_header)) = BinaryHeader::from_reader((&mut cursor, 0))
                && binary_header.flag == 0x100
            {
                let received_nonce = guid_create::GUID::build_from_components(
                    u32::from_le_bytes(
                        message
                            .get(36..40)
                            .unwrap_or_default()
                            .try_into()
                            .unwrap_or_default(),
                    ),
                    u16::from_le_bytes(
                        message
                            .get(40..42)
                            .unwrap_or_default()
                            .try_into()
                            .unwrap_or_default(),
                    ),
                    u16::from_le_bytes(
                        message
                            .get(42..44)
                            .unwrap_or_default()
                            .try_into()
                            .unwrap_or_default(),
                    ),
                    message
                        .get(44..)
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(&[0; 8]),
                );

                if received_nonce == nonce {
                    identifier.fetch_add(1, Ordering::SeqCst);
                    let Ok(header) = BinaryHeader {
                        session_id: 0,
                        identifier: identifier.load(Ordering::SeqCst),
                        data_offset: 0,
                        total_data_size: 0,
                        length: 0,
                        flag: 0x00,
                        ack_identifier: nonce.data1(),
                        ack_unique_id: u32::from_be_bytes(
                            [nonce.data3().to_be_bytes(), nonce.data2().to_be_bytes()]
                                .concat()
                                .try_into()
                                .unwrap_or_default(),
                        ),
                        ack_data_size: u64::from_ne_bytes(nonce.data4()),
                    }
                    .to_bytes() else {
                        break 'receiving;
                    };

                    let _ = socket.write_all(&header.len().to_le_bytes()).await;
                    let _ = socket.write_all(&header).await;
                } else {
                    break 'receiving;
                }

                loop {
                    let Ok(message) = read(socket, &mut length_buf).await else {
                        break 'receiving;
                    };

                    if let Some(binary_header) = message.get(..48) {
                        let mut cursor = Cursor::new(binary_header);
                        if let Ok((_, binary_header)) = BinaryHeader::from_reader((&mut cursor, 0))
                            && let Some(data) = message.get(48..)
                        {
                            match binary_header.flag {
                                0x1000030 => {
                                    let _ =
                                        internal_tx.send(InternalEvent::P2pDirectConnectionData {
                                            binary_header: binary_header.clone(),
                                            data: data.to_vec(),
                                        });

                                    if binary_header.data_offset + binary_header.length as u64
                                        == binary_header.total_data_size
                                        && let Ok(mut ack) = P2pSession::acknowledge(&message)
                                    {
                                        ack.truncate(ack.len() - 4);
                                        let _ = socket.write_all(&ack.len().to_le_bytes()).await;
                                        let _ = socket.write_all(&ack).await;
                                    }
                                }

                                0x00 => {
                                    let message_string = unsafe { str::from_utf8_unchecked(data) };
                                    if message_string.contains("BYE")
                                        && let Ok(mut ack) = P2pSession::acknowledge(&message)
                                    {
                                        ack.truncate(ack.len() - 4);
                                        let _ = socket.write_all(&ack.len().to_le_bytes()).await;
                                        let _ = socket.write_all(&ack).await;
                                        break 'receiving;
                                    }
                                }

                                _ => (),
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn read(socket: &mut TcpStream, length_buf: &mut Vec<u8>) -> Result<Vec<u8>, P2pError> {
    let received = socket.read_exact(length_buf).await.unwrap_or_default();
    if received == 0 {
        return Err(P2pError::ReceivingError);
    }

    let length = u32::from_le_bytes(length_buf.as_slice().try_into().unwrap_or_default()) as usize;

    if length == 0 {
        return Err(P2pError::ReceivingError);
    }

    let mut data = vec![0; length];
    let mut received = socket.read(&mut data).await.unwrap_or_default();

    if received == 0 {
        return Err(P2pError::ReceivingError);
    }

    while received < length {
        let mut buf = vec![0; length - received];
        received += socket.read_exact(&mut buf).await.unwrap_or_default();
        data.append(&mut buf);

        if received == 0 {
            return Err(P2pError::ReceivingError);
        }
    }

    Ok(data)
}
