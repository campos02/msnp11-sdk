use crate::sdk_error::SdkError;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use core::str;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;

pub(crate) async fn receive_split_into_base64(
    rd: &mut OwnedReadHalf,
) -> Result<Vec<String>, SdkError> {
    let mut buf = vec![0; 1664];
    let received = rd.read(&mut buf).await.unwrap_or(0);

    if received == 0 {
        return Err(SdkError::Disconnected);
    }

    let mut messages_bytes = buf[..received].to_vec();
    let mut base64_messages: Vec<String> = Vec::new();

    loop {
        let messages_string = unsafe { str::from_utf8_unchecked(&messages_bytes) };
        let messages: Vec<String> = messages_string
            .lines()
            .map(|line| line.to_string() + "\r\n")
            .collect();

        if messages.len() == 0 {
            break;
        }

        let args: Vec<&str> = messages[0].trim().split(' ').collect();
        match args[0] {
            "GCF" | "UBX" | "MSG" => {
                let length_index = match args[0] {
                    "UBX" => 2,
                    _ => 3,
                };

                let Ok(length) = args[length_index].parse::<usize>() else {
                    continue;
                };

                let length = messages[0].len() + length;
                if length > messages_bytes.len() {
                    let mut buf = vec![0; 1664];
                    let received = rd.read(&mut buf).await.unwrap_or(0);

                    if received == 0 {
                        return Err(SdkError::Disconnected);
                    }

                    let buf = &buf[..received];
                    messages_bytes.extend_from_slice(buf);
                    continue;
                }

                let new_bytes = messages_bytes.drain(..length);
                base64_messages.push(STANDARD.encode(new_bytes));
            }

            _ => {
                let new_bytes = messages_bytes.drain(..messages[0].len());
                base64_messages.push(STANDARD.encode(new_bytes));
            }
        }
    }

    Ok(base64_messages)
}
