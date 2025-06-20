use base64::{Engine as _, engine::general_purpose::STANDARD};
use core::str;
use log::{error, trace};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct MockNS;

impl MockNS {
    pub async fn listen() {
        let listener = TcpListener::bind("127.0.0.1:1863")
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
                            "UUX" => {
                                let length = args[2].parse::<usize>().unwrap();
                                let length = messages[0].len() + length;

                                let new_bytes = &messages_bytes[..length];
                                messages_bytes = &messages_bytes[length..];

                                let base64_message = STANDARD.encode(&new_bytes);
                                base64_messages.push(base64_message);
                            }

                            _ => {
                                let new_bytes = &messages_bytes[..messages[0].len()];
                                messages_bytes = &messages_bytes[messages[0].len()..];

                                let base64_message = STANDARD.encode(new_bytes);
                                base64_messages.push(base64_message);
                            }
                        }
                    }

                    for base64_message in base64_messages {
                        let message_bytes = STANDARD.decode(base64_message).unwrap();
                        let message = unsafe { str::from_utf8_unchecked(message_bytes.as_slice()) };
                        trace!("C: {message}");

                        let replies: &[&str] = match message {
                            "VER 1 MSNP11 CVR0\r\n" => &["VER 1 MSNP11\r\n"],
                            "CVR 2 0x0409 winnt 10 i386 msnp11-sdk 0.01 msmsgs testing@example.com\r\n" => {
                                &["CVR 2 1.0.0000 1.0.0000 7.0.0425\r\n"]
                            }

                            "USR 3 TWN I testing@example.com\r\n" => {
                                &["USR 3 TWN S ct=1,rver=1,wp=FS_40SEC_0_COMPACT,lc=1,id=1\r\n"]
                            }

                            "USR 4 TWN S aaa123aaa123\r\n" => {
                                &["USR 4 OK testing@example.com Testing 1 0\r\n"]
                            }

                            "SYN 5 0 0\r\n" => &[
                                "SYN 5 0 0 2 1\r\n",
                                "GTC A\r\n",
                                "BLP AL\r\n",
                                "PRP MFN Testing\r\n",
                                "LSG Mock%20Contacts 124153dc-a695-4f6c-93e8-8e07c9775251\r\n",
                                "LST N=bob@passport.com F=Bob C=6bd736b8-dc18-44c6-ad61-8cd12d641e79 13 124153dc-a695-4f6c-93e8-8e07c9775251\r\n",
                                "LST N=fred@passport.com F=Fred 2\r\n",
                            ],

                            "GCF 6 Shields.xml\r\n" => {
                                &["GCF 6 Shields.xml 33\r\n</shield><block></block></config>"]
                            }

                            "CHG 7 NLN 1073741824\r\n" => &[
                                "CHG 7 NLN 1073741824\r\n",
                                "ILN 7 NLN bob@passport.com Bob 1073741824 %3Cmsnobj%20Creator%3D%22\r\n",
                                "NLN NLN bob@passport.com Bob 1073741824 %3Cmsnobj%20Creator%3D%22\r\n",
                                "UBX bob@passport.com 70\r\n<Data><PSM>my msn all ducked</PSM><CurrentMedia></CurrentMedia></Data>",
                            ],

                            "UUX 8 43\r\n<Data><PSM>test</PSM><CurrentMedia/></Data>" => {
                                &["UUX 8 0\r\n"]
                            }

                            "PNG\r\n" => &["QNG 60\r\n"],
                            "ADC 7 FL N=bob@passport.com F=Bob\r\n" => &[
                                "ADC 7 FL N=bob@passport.com F=Bob C=6bd736b8-dc18-44c6-ad61-8cd12d641e79\r\n",
                                "ADC 0 RL N=fred@passport.com F=Fred\r\n",
                            ],

                            "ADC 8 AL N=fred@passport.com\r\n" => {
                                &["ADC 8 AL N=fred@passport.com\r\n"]
                            }

                            "XFR 7 SB\r\n" => &["XFR 7 SB 127.0.0.1:1864 CKI 123456\r\n"],

                            "GTC 7 ReceiveRNG\r\n" => &[
                                "GTC 7 ReceiveRNG\r\n",
                                "RNG 11752013 127.0.0.1:1864 CKI 123456 bob@passport.com Bob\r\n",
                            ],

                            _ => &[],
                        };

                        for reply in replies {
                            trace!("S: {reply}");
                            if wr.write_all(&reply.as_bytes()).await.is_err() {
                                error!("Error sending to client");
                            }
                        }
                    }
                }
            }
        });
    }
}
