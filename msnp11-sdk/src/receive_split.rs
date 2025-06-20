use crate::sdk_error::SdkError;
use core::str;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;

pub(crate) async fn receive_split(rd: &mut OwnedReadHalf) -> Result<Vec<Vec<u8>>, SdkError> {
    let mut buf = vec![0; 1664];
    let received = rd.read(&mut buf).await.unwrap_or(0);

    if received == 0 {
        return Err(SdkError::Disconnected);
    }

    let mut messages_bytes = buf[..received].to_vec();
    let mut messages: Vec<Vec<u8>> = Vec::new();

    loop {
        let messages_string = unsafe { str::from_utf8_unchecked(&messages_bytes) };
        let message_lines: Vec<String> = messages_string
            .lines()
            .map(|line| line.to_string() + "\r\n")
            .collect();

        if message_lines.len() == 0 {
            break;
        }

        let args: Vec<&str> = message_lines[0].trim().split(' ').collect();
        match args[0] {
            "GCF" | "UBX" | "MSG" => {
                let length_index = match args[0] {
                    "UBX" => 2,
                    _ => 3,
                };

                let Ok(length) = args[length_index].parse::<usize>() else {
                    continue;
                };

                let length = message_lines[0].len() + length;
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
                messages.push(new_bytes.as_ref().to_vec());
            }

            _ => {
                let new_bytes = messages_bytes.drain(..message_lines[0].len());
                messages.push(new_bytes.as_ref().to_vec());
            }
        }
    }

    Ok(messages)
}
