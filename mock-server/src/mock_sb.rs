use core::str;
use log::{error, trace};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct MockSB;

impl MockSB {
    pub async fn listen() {
        let listener = TcpListener::bind("127.0.0.1:1864")
            .await
            .expect("Could not bind mock Notification Server");

        tokio::spawn(async move {
            while let Ok(client) = listener.accept().await {
                let (mut socket, _) = client;
                let (mut rd, mut wr) = socket.split();

                let mut buf = vec![0; 1664];
                while let Ok(received) = rd.read(&mut buf).await {
                    if received == 0 {
                        break;
                    }

                    let mut messages_bytes = &buf[..received];
                    let mut messages: Vec<&[u8]> = Vec::new();

                    loop {
                        let messages_string = unsafe { str::from_utf8_unchecked(messages_bytes) };
                        let message_lines: Vec<String> = messages_string
                            .lines()
                            .map(|line| line.to_string() + "\r\n")
                            .collect();

                        if message_lines.is_empty() {
                            break;
                        }

                        let args: Vec<&str> = message_lines[0].split_ascii_whitespace().collect();
                        match args[0] {
                            "MSG" => {
                                let length = args[3].parse::<usize>().unwrap();
                                let length = message_lines[0].len() + length;

                                let new_bytes = &messages_bytes[..length];
                                messages_bytes = &messages_bytes[length..];
                                messages.push(new_bytes);
                            }

                            _ => {
                                let new_bytes = &messages_bytes[..message_lines[0].len()];
                                messages_bytes = &messages_bytes[message_lines[0].len()..];
                                messages.push(new_bytes);
                            }
                        }
                    }

                    for message in messages {
                        let message = unsafe { str::from_utf8_unchecked(message) };
                        trace!("C: {message}");

                        let replies: &[&str] = match message {
                            "USR 1 testing@example.com 123456\r\n" => {
                                &["USR 1 OK testing@example.com Testing\r\n"]
                            }

                            "ANS 1 testing@example.com 123456 11752013\r\n" => &[
                                "IRO 1 1 1 bob@passport.com Bob\r\n",
                                "ANS 1 OK\r\n",
                                "MSG bob@passport.com Bob 134\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nX-MMS-IM-Format: FN=Microsoft%20Sans%20Serif; EF=; CO=ff; CS=0; PF=22\r\n\r\nh",
                                "MSG bob@passport.com Bob 69\r\nMIME-Version: 1.0\r\nContent-Type: text/x-msnmsgr-datacast\r\n\r\nID: 1\r\n\r\n",
                                "BYE bob@passport.com\r\n",
                            ],

                            "CAL 2 bob@passport.com\r\n" => {
                                &["CAL 2 RINGING 11752013\r\n", "JOI bob@passport.com\r\n"]
                            }

                            "MSG 3 A 137\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nX-MMS-IM-Format: FN=Microsoft%20Sans%20Serif; EF=; CO=ff0000; CS=1; PF=0\r\n\r\nh" => {
                                &[
                                    "ACK 3\r\n",
                                    "MSG bob@passport.com Bob 134\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nX-MMS-IM-Format: FN=Microsoft%20Sans%20Serif; EF=; CO=ff; CS=0; PF=22\r\n\r\nh",
                                    "MSG bob@passport.com Bob 69\r\nMIME-Version: 1.0\r\nContent-Type: text/x-msnmsgr-datacast\r\n\r\nID: 1\r\n\r\n",
                                    "BYE bob@passport.com\r\n",
                                ]
                            }

                            "MSG 4 A 137\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nX-MMS-IM-Format: FN=Microsoft%20Sans%20Serif; EF=; CO=ff0000; CS=1; PF=0\r\n\r\nh" => {
                                &["ACK 4\r\n"]
                            }

                            _ => &[],
                        };

                        for reply in replies {
                            trace!("S: {reply}");
                            if wr.write_all(reply.as_bytes()).await.is_err() {
                                error!("Error sending to client");
                            }
                        }
                    }
                }
            }
        });
    }
}
