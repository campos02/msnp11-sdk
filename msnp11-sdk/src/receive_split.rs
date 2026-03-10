use crate::errors::sdk_error::SdkError;
use core::str;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio_util::sync::CancellationToken;

pub(crate) async fn receive_split(
    rd: &mut OwnedReadHalf,
    cancellation_token: CancellationToken,
) -> Result<Vec<Vec<u8>>, SdkError> {
    let mut buf = vec![0; 1664];
    let received = tokio::select! {
        received = rd.read(&mut buf) => {
            received.unwrap_or(0)
        }

        _ = cancellation_token.cancelled() => {
            return Err(SdkError::Disconnected);
        }
    };

    if received == 0 {
        return Err(SdkError::Disconnected);
    }

    let mut messages_bytes = buf.get(..received).unwrap_or_default().to_vec();
    let mut messages = Vec::new();

    loop {
        let messages_string = unsafe { str::from_utf8_unchecked(&messages_bytes) };
        let Some(message) = messages_string.lines().next() else {
            break;
        };

        let message = message.to_string() + "\r\n";
        let mut args = message.split_ascii_whitespace();
        let command = args.next().unwrap_or("");

        match command {
            "GCF" | "UBX" | "MSG" => {
                let length_index = match command {
                    "UBX" => 1,
                    _ => 2,
                };

                let Ok(length) = args.nth(length_index).unwrap_or("").parse::<usize>() else {
                    continue;
                };

                let length = message.len() + length;
                if length > messages_bytes.len() {
                    let mut buf = vec![0; 1664];
                    let received = tokio::select! {
                        received = rd.read(&mut buf) => {
                            received.unwrap_or(0)
                        }

                        _ = cancellation_token.cancelled() => {
                            return Err(SdkError::Disconnected);
                        }
                    };

                    if received == 0 {
                        return Err(SdkError::Disconnected);
                    }

                    let buf = buf.get(..received).unwrap_or_default();
                    messages_bytes.extend_from_slice(buf);
                    continue;
                }

                let new_bytes = messages_bytes.drain(..length);
                messages.push(new_bytes.collect());
            }

            _ => {
                let length = message.len();
                if length > messages_bytes.len() {
                    break;
                }

                let new_bytes = messages_bytes.drain(..length);
                messages.push(new_bytes.collect());
            }
        }
    }

    Ok(messages)
}
