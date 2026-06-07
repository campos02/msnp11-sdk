use crate::P2pError;
use crate::P2pError::BinaryHeaderWritingError;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use crate::switchboard_server::p2p::bye;
use deku::DekuContainerWrite;
use rand::{Rng, rng};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[allow(clippy::too_many_arguments)]
pub async fn send_file(
    ips: &[String],
    port: u16,
    nonce: &guid_create::GUID,
    session_id: u32,
    identifier: &mut u32,
    branch: &guid_create::GUID,
    call_id: &guid_create::GUID,
    to: &str,
    from: &str,
    file: &[u8],
) -> Result<(), P2pError> {
    for ip in ips {
        if let Ok(mut socket) = TcpStream::connect((ip.as_str(), port)).await {
            let _ = socket.write_all(&u32::to_le_bytes(4)).await;
            let _ = socket.write_all("foo\0".as_bytes()).await;
            let _ = socket.write_all(&u32::to_le_bytes(48)).await;

            let mut message = Vec::with_capacity(48);
            *identifier += 1;

            let header = BinaryHeader {
                session_id: 0,
                identifier: *identifier,
                data_offset: 0,
                total_data_size: 0,
                length: 0,
                flag: 0x100,
                ack_identifier: nonce.data1(),
                ack_unique_id: u32::from_be_bytes(
                    [nonce.data3().to_be_bytes(), nonce.data2().to_be_bytes()]
                        .concat()
                        .try_into()
                        .unwrap_or_default(),
                ),
                ack_data_size: u64::from_ne_bytes(nonce.data4()),
            }
            .to_bytes()
            .or(Err(BinaryHeaderWritingError))?;

            message.extend_from_slice(&header);
            let _ = socket.write_all(&message).await;

            let mut message = Vec::with_capacity(52);
            while message.len() < 52 {
                let mut buf = vec![0; 52 - message.len()];
                let received = socket.read(&mut buf).await.unwrap_or_default();

                if received == 0 {
                    break;
                }

                message.extend_from_slice(&buf);
            }

            let flag = u32::from_le_bytes(
                message
                    .get(32..36)
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or_default(),
            );

            if flag == 0x100 {
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

                if received_nonce == *nonce {
                    let total_data_size = file.len() as u64;
                    let mut data_offset = 0u64;
                    *identifier += 1;

                    for chunk in file.chunks(1352) {
                        let mut message = BinaryHeader {
                            session_id,
                            identifier: *identifier,
                            data_offset,
                            total_data_size,
                            length: chunk.len() as u32,
                            flag: 0x1000030,
                            ack_identifier: rng().next_u32(),
                            ack_unique_id: 0,
                            ack_data_size: 0,
                        }
                        .to_bytes()
                        .or(Err(BinaryHeaderWritingError))?;

                        data_offset += chunk.len() as u64;
                        message.extend_from_slice(chunk);

                        let _ = socket
                            .write_all(&u32::to_le_bytes(message.len() as u32))
                            .await;

                        let _ = socket.write_all(&message).await;
                    }

                    // Receive acknowledgement
                    let mut message = Vec::with_capacity(52);
                    while message.len() < 52 {
                        let mut buf = vec![0; 52 - message.len()];
                        let received = socket.read(&mut buf).await.unwrap_or_default();

                        if received == 0 {
                            break;
                        }

                        message.extend_from_slice(&buf);
                    }

                    if let Ok(mut bye) = bye::bye(to, from, branch, call_id, identifier) {
                        bye.truncate(bye.len() - 4);
                        let _ = socket.write_all(&u32::to_le_bytes(bye.len() as u32)).await;
                        let _ = socket.write_all(&bye).await;
                        return Ok(());
                    }
                }
            }
        }
    }

    Err(P2pError::CouldNotSendThroughDirectConnection)
}
