use core::str;
use log::trace;
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

                    let message = unsafe { str::from_utf8_unchecked(&buf[..received]) };
                    trace!("C: {message}");

                    let replies: &[String] = match message {
                        "VER 1 MSNP11 CVR0\r\n" => &["VER 1 MSNP11\r\n".to_string()],
                        "CVR 2 0x0409 winnt 10 i386 msnp11-sdk 0.01 msmsgs testing@example.com\r\n" => &["CVR 2 1.0.0000 1.0.0000 7.0.0425 http://download.microsoft.com/download/D/F/B/DFB59A5D-92DF-4405-9767-43E3DF10D25B/fr/Install_MSN_Messenger.exe http://messenger.msn.com/fr\r\n".to_string()],
                        "USR 3 TWN I testing@example.com\r\n" => &["USR 3 TWN S ct=1,rver=1,wp=FS_40SEC_0_COMPACT,lc=1,id=1\r\n".to_string()],
                        "USR 4 TWN S aaa123aaa123\r\n" => &["USR 4 OK testing@example.com Testing 1 0\r\n".to_string()],
                        "SYN 5 0 0\r\n" =>
                            &["SYN 5 0 0 2 1\r\n".to_string(),
                            "GTC A\r\n".to_string(),
                            "BLP AL\r\n".to_string(),
                            "PRP MFN Testing\r\n".to_string(),
                            "LSG Mock%20Contacts 124153dc-a695-4f6c-93e8-8e07c9775251\r\n".to_string(),
                            "LST N=bob@passport.com F=Bob C=6bd736b8-dc18-44c6-ad61-8cd12d641e79 13 124153dc-a695-4f6c-93e8-8e07c9775251\r\n".to_string(),
                            "LST N=fred@passport.com F=Fred 2\r\n".to_string()],

                        "GCF 6 Shields.xml\r\n" => &[Self::gcf_reply()],
                        "CHG 7 NLN 1073741824\r\n" => 
                            &["CHG 7 NLN 1073741824\r\n".to_string(),
                            "ILN 7 NLN bob@passport.com Bob 1073741824 %3Cmsnobj%20Creator%3D%22\r\n".to_string(),
                            "NLN NLN bob@passport.com Bob 1073741824 %3Cmsnobj%20Creator%3D%22\r\n".to_string(),
                            "UBX bob@passport.com 70\r\n<Data><PSM>my msn all ducked</PSM><CurrentMedia></CurrentMedia></Data>".to_string()
                        ],
                        "UUX 8 43\r\n<Data><PSM>test</PSM><CurrentMedia/></Data>" => &["UUX 8 0\r\n".to_string()],
                        "PNG\r\n" => &["QNG 60\r\n".to_string()],
                        _ => &[]
                    };

                    for reply in replies {
                        trace!("S: {reply}");
                        wr.write_all(&reply.as_bytes())
                            .await
                            .expect("Could not write reply from Mock Notification Server");
                    }
                }
            }
        });
    }

    fn gcf_reply() -> String {
        let mut payload = r#"<?xml version= "1.0" encoding="utf-8" ?>"#.to_string();
        payload.push_str(
            r#"<config><shield><cli maj="7" min="0" minbld="0" maxbld="9999" deny=" " />"#,
        );
        payload.push_str("</shield><block></block></config>");

        let length = payload.len();
        format!("GCF 6 Shields.xml {length}\r\n{payload}")
    }
}
