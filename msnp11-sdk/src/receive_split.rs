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
        let Some(command) = messages_string.lines().next() else {
            break;
        };

        let command = command.to_string() + "\r\n";
        let args: Vec<&str> = command.split_ascii_whitespace().collect();

        match *args.first().unwrap_or(&"") {
            "GCF" | "UBX" | "MSG" => {
                let length_index = match *args.first().unwrap_or(&"") {
                    "UBX" => 2,
                    _ => 3,
                };

                let Ok(length) = args.get(length_index).unwrap_or(&"").parse::<usize>() else {
                    continue;
                };

                let length = command.len() + length;
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
                let length = command.len();
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
