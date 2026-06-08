use crate::enums::internal_event::InternalEvent;
use crate::errors::p2p_error::P2pError;
use crate::models::user_data::UserData;
use crate::switchboard_server::commands::msg;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
#[cfg(feature = "file-transfers")]
use crate::switchboard_server::p2p::file_context::FileContext;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use core::str;
use deku::{DekuContainerRead, DekuContainerWrite};
use rand::Rng;
use rand::rng;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
#[cfg(feature = "file-transfers")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(feature = "file-transfers")]
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::{RwLock, broadcast};

pub struct P2pSession {
    session_id: u32,
    identifier: u32,
    branch: guid_create::GUID,
    call_id: guid_create::GUID,
}

impl Default for P2pSession {
    fn default() -> Self {
        Self::new()
    }
}

impl P2pSession {
    pub fn new() -> Self {
        Self {
            session_id: 0,
            identifier: rng().next_u32(),
            branch: guid_create::GUID::default(),
            call_id: guid_create::GUID::default(),
        }
    }

    pub fn new_from_existing_session(
        branch: guid_create::GUID,
        call_id: guid_create::GUID,
        session_id: u32,
    ) -> Self {
        Self {
            session_id,
            identifier: rng().next_u32(),
            branch,
            call_id,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn handle_display_picture_invite(
        &mut self,
        to: &str,
        from: &str,
        context: &str,
        invite: Vec<u8>,
        user_data: Arc<RwLock<UserData>>,
        command_internal_rx: &mut broadcast::Receiver<InternalEvent>,
        tr_id: Arc<AtomicU32>,
        sb_tx: Sender<Vec<u8>>,
    ) -> Result<(), P2pError> {
        {
            let user_data = user_data.read().await;
            let user_email = user_data.email.as_ref().ok_or(P2pError::NotLoggedIn)?;

            if to != *user_email {
                return Err(P2pError::OtherDestination);
            }
        }

        let ack_payload = P2pSession::acknowledge(&invite)?;
        msg::send_p2p(&tr_id, &sb_tx, command_internal_rx, ack_payload, from).await?;

        {
            let user_data = user_data.read().await;
            let msn_object = user_data
                .msn_object
                .as_ref()
                .ok_or(P2pError::CouldNotGetUserData)?;

            if context != STANDARD.encode((msn_object.to_owned() + "\0").as_bytes()) {
                return Err(P2pError::OtherContext);
            }
        }

        let ok_payload = self.ok(from, to)?;
        msg::send_p2p(&tr_id, &sb_tx, command_internal_rx, ok_payload, from).await?;

        let preparation_payload = self.data_preparation()?;
        msg::send_p2p(
            &tr_id,
            &sb_tx,
            command_internal_rx,
            preparation_payload,
            from,
        )
        .await?;

        let user_data = user_data.read().await;
        let display_picture = user_data
            .display_picture
            .as_ref()
            .ok_or(P2pError::CouldNotGetDisplayPicture)?;

        let data_payloads = self.data(display_picture, false)?;
        for data_payload in data_payloads {
            msg::send_p2p(&tr_id, &sb_tx, command_internal_rx, data_payload, from).await?;
        }

        Ok(())
    }

    pub(crate) async fn handle_bye(
        to: &str,
        from: &str,
        bye: Vec<u8>,
        user_data: Arc<RwLock<UserData>>,
        command_internal_rx: &mut broadcast::Receiver<InternalEvent>,
        tr_id: Arc<AtomicU32>,
        sb_tx: Sender<Vec<u8>>,
    ) -> Result<(), P2pError> {
        {
            let user_data = user_data.read().await;
            let user_email = user_data.email.as_ref().ok_or(P2pError::NotLoggedIn)?;

            if to != *user_email {
                return Err(P2pError::OtherDestination);
            }
        }

        let ack_payload = P2pSession::acknowledge(&bye)?;
        msg::send_p2p(&tr_id, &sb_tx, command_internal_rx, ack_payload, from).await
    }

    pub fn picture_invite(
        &mut self,
        to: &str,
        from: &str,
        msn_object: &str,
    ) -> Result<Vec<u8>, P2pError> {
        self.session_id = rng().next_u32();
        let mut body = "EUF-GUID: {A4268EEC-FEC5-49E5-95C3-F126696BDBF6}\r\n".to_string();
        body.push_str(format!("SessionID: {}\r\n", self.session_id).as_str());
        body.push_str("AppID: 1\r\n");
        body.push_str(
            format!(
                "Context: {}\r\n\r\n\0",
                STANDARD.encode((msn_object.to_owned() + "\0").as_bytes())
            )
            .as_str(),
        );

        self.invite(
            to,
            from,
            &body,
            guid_create::GUID::rand(),
            "application/x-msnmsgr-sessionreqbody",
        )
    }

    #[cfg(feature = "file-transfers")]
    pub fn file_invite(
        &mut self,
        to: &str,
        from: &str,
        file_name: &str,
        file_size: u64,
    ) -> Result<Vec<u8>, P2pError> {
        let mut utf16_file_name = Vec::with_capacity(554);
        let file_name = file_name.encode_utf16();

        for character in file_name {
            for byte in character.to_le_bytes() {
                utf16_file_name.push(byte);
            }
        }

        utf16_file_name.resize(utf16_file_name.capacity() - 4, 0);
        utf16_file_name.extend_from_slice(&[255; 4]);

        // 574 will work with the official clients, 1 means no preview
        let mut context = FileContext {
            size: 574,
            second_field: 2,
            file_size,
            preview: 1,
        }
        .to_bytes()
        .or(Err(P2pError::InviteError))?;

        context.extend_from_slice(&utf16_file_name);
        self.session_id = rng().next_u32();

        let mut body = "EUF-GUID: {5D3E02AB-6190-11D3-BBBB-00C04F795683}\r\n".to_string();
        body.push_str(format!("SessionID: {}\r\n", self.session_id).as_str());
        body.push_str("AppID: 2\r\n");
        body.push_str(format!("Context: {}\r\n\r\n\0", STANDARD.encode(context)).as_str());

        self.invite(
            to,
            from,
            &body,
            guid_create::GUID::rand(),
            "application/x-msnmsgr-sessionreqbody",
        )
    }

    #[cfg(feature = "file-transfers")]
    pub fn direct_connection_invite(&mut self, to: &str, from: &str) -> Result<Vec<u8>, P2pError> {
        let mut body = "Bridges: TCPv1\r\n".to_string();
        body.push_str("NetID: 0\r\n");
        body.push_str("Conn-Type: Firewall\r\n");
        body.push_str("UPnPNat: false\r\n");
        body.push_str("ICF: false\r\n");
        body.push_str(&format!(
            "Nonce: {{{}}}\r\n\r\n\0",
            guid_create::GUID::rand()
        ));

        self.invite(
            to,
            from,
            &body,
            self.call_id,
            "application/x-msnmsgr-transreqbody",
        )
    }

    fn invite(
        &mut self,
        to: &str,
        from: &str,
        body: &str,
        call_id: guid_create::GUID,
        content_type: &str,
    ) -> Result<Vec<u8>, P2pError> {
        let branch = guid_create::GUID::rand();
        let mut headers = format!("INVITE MSNMSGR:{to} MSNSLP/1.0\r\n");
        headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
        headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
        headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{branch}}}\r\n").as_str());
        headers.push_str("CSeq: 0 \r\n");
        headers.push_str(format!("Call-ID: {{{call_id}}}\r\n").as_str());
        headers.push_str("Max-Forwards: 0\r\n");
        headers.push_str(&format!("Content-Type: {content_type}\r\n"));
        headers.push_str(format!("Content-Length: {}\r\n\r\n", body.len()).as_str());

        let message = format!("{headers}{body}");
        self.identifier += 1;

        let mut invite = BinaryHeader {
            session_id: 0,
            identifier: self.identifier,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: rng().next_u32(),
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderReadingError))?;

        self.branch = branch;
        self.call_id = call_id;

        invite.extend_from_slice(message.as_bytes());
        invite.extend_from_slice(&[0; 4]);
        Ok(invite)
    }

    pub fn acknowledge(payload: &[u8]) -> Result<Vec<u8>, P2pError> {
        let binary_header = payload
            .get(..48)
            .ok_or(P2pError::BinaryHeaderReadingError)?;

        let mut cursor = Cursor::new(binary_header);
        let (_, binary_header) = BinaryHeader::from_reader((&mut cursor, 0))
            .or(Err(P2pError::BinaryHeaderReadingError))?;

        let mut ack_header = BinaryHeader {
            session_id: binary_header.session_id,
            identifier: !binary_header.identifier,
            data_offset: 0,
            total_data_size: binary_header.total_data_size,
            length: 0,
            flag: 0x02,
            ack_identifier: binary_header.identifier,
            ack_unique_id: binary_header.ack_identifier,
            ack_data_size: binary_header.total_data_size,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderReadingError))?;

        ack_header.extend_from_slice(&[0; 4]);
        Ok(ack_header)
    }

    pub fn ok(&mut self, to: &str, from: &str) -> Result<Vec<u8>, P2pError> {
        let body = format!("SessionID: {}\r\n\r\n\0", self.session_id);
        let mut headers = "MSNSLP/1.0 200 OK\r\n".to_string();
        headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
        headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
        headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{}}}\r\n", self.branch).as_str());
        headers.push_str("CSeq: 1 \r\n");
        headers.push_str(format!("Call-ID: {{{}}}\r\n", self.call_id).as_str());
        headers.push_str("Max-Forwards: 0\r\n");
        headers.push_str("Content-Type: application/x-msnmsgr-sessionreqbody\r\n");
        headers.push_str(format!("Content-Length: {}\r\n\r\n", body.len()).as_str());

        let message = format!("{headers}{body}");
        self.identifier += 1;

        let mut ok = BinaryHeader {
            session_id: 0,
            identifier: self.identifier,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: rng().next_u32(),
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderWritingError))?;

        ok.extend_from_slice(message.as_bytes());
        ok.extend_from_slice(&[0; 4]);
        Ok(ok)
    }

    #[cfg(feature = "file-transfers")]
    pub async fn direct_connection_ok(
        &mut self,
        to: &str,
        from: &str,
    ) -> Result<Vec<u8>, P2pError> {
        let mut body = "Bridge: TCPv1\r\n".to_string();
        body.push_str("Listening: false\r\n");
        body.push_str("Nonce: {00000000-0000-0000-0000-000000000000}\r\n");
        body.push_str("\r\n\0");

        let mut headers = "MSNSLP/1.0 200 OK\r\n".to_string();
        headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
        headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
        headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{}}}\r\n", self.branch).as_str());
        headers.push_str("CSeq: 1 \r\n");
        headers.push_str(format!("Call-ID: {{{}}}\r\n", self.call_id).as_str());
        headers.push_str("Max-Forwards: 0\r\n");
        headers.push_str("Content-Type: application/x-msnmsgr-transrespbody\r\n");
        headers.push_str(format!("Content-Length: {}\r\n\r\n", body.len()).as_str());

        let message = format!("{headers}{body}");
        self.identifier += 1;

        let mut ok = BinaryHeader {
            session_id: 0,
            identifier: self.identifier,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: rng().next_u32(),
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderWritingError))?;

        ok.extend_from_slice(message.as_bytes());
        ok.extend_from_slice(&[0; 4]);
        Ok(ok)
    }

    #[cfg(feature = "file-transfers")]
    pub fn decline(&mut self, to: &str, from: &str) -> Result<Vec<u8>, P2pError> {
        let body = format!("SessionID: {}\r\n\r\n\0", self.session_id);

        let mut headers = "MSNSLP/1.0 603 Decline\r\n".to_string();
        headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
        headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
        headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{}}}\r\n", self.branch).as_str());
        headers.push_str("CSeq: 1 \r\n");
        headers.push_str(format!("Call-ID: {{{}}}\r\n", self.call_id).as_str());
        headers.push_str("Max-Forwards: 0\r\n");
        headers.push_str("Content-Type: application/x-msnmsgr-sessionreqbody\r\n");
        headers.push_str(format!("Content-Length: {}\r\n\r\n", body.len()).as_str());

        let message = format!("{headers}{body}");
        self.identifier += 1;

        let mut decline = BinaryHeader {
            session_id: 0,
            identifier: self.identifier,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: rng().next_u32(),
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderWritingError))?;

        decline.extend_from_slice(message.as_bytes());
        decline.extend_from_slice(&[0; 4]);
        Ok(decline)
    }

    pub fn data_preparation(&mut self) -> Result<Vec<u8>, P2pError> {
        let message = &[0; 4];
        self.identifier += 1;

        let mut data_preparation = BinaryHeader {
            session_id: self.session_id,
            identifier: self.identifier,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: rng().next_u32(),
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderWritingError))?;

        data_preparation.extend_from_slice(message);
        data_preparation.extend_from_slice(&[0, 0, 0, 1]);
        Ok(data_preparation)
    }

    pub fn data(&mut self, data: &[u8], file_transfer: bool) -> Result<Vec<Vec<u8>>, P2pError> {
        let mut payloads: Vec<Vec<u8>> = Vec::new();
        let total_data_size = data.len() as u64;
        let mut data_offset = 0u64;
        self.identifier += 1;

        let chunks = data.chunks(1202);
        for chunk in chunks {
            let mut data_message = BinaryHeader {
                session_id: self.session_id,
                identifier: self.identifier,
                data_offset,
                total_data_size,
                length: chunk.len() as u32,
                flag: if file_transfer { 0x1000030 } else { 0x20 },
                ack_identifier: rng().next_u32(),
                ack_unique_id: 0,
                ack_data_size: 0,
            }
            .to_bytes()
            .or(Err(P2pError::BinaryHeaderWritingError))?;

            data_message.extend_from_slice(chunk);
            data_message.extend_from_slice(&[0, 0, 0, 1]);

            data_offset += chunk.len() as u64;
            payloads.push(data_message);
        }

        Ok(payloads)
    }

    pub fn bye(&mut self, to: &str, from: &str) -> Result<Vec<u8>, P2pError> {
        let body = "\r\n\0";

        let mut headers = format!("BYE MSNMSGR:{to} MSNSLP/1.0\r\n");
        headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
        headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
        headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{}}}\r\n", self.branch).as_str());
        headers.push_str("CSeq: 0 \r\n");
        headers.push_str(format!("Call-ID: {{{}}}\r\n", self.call_id).as_str());
        headers.push_str("Max-Forwards: 0\r\n");
        headers.push_str("Content-Type: application/x-msnmsgr-sessionclosebody\r\n");
        headers.push_str(format!("Content-Length: {}\r\n\r\n", body.len()).as_str());

        let message = format!("{headers}{body}");
        self.identifier += 1;

        let mut bye = BinaryHeader {
            session_id: 0,
            identifier: self.identifier,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: rng().next_u32(),
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderReadingError))?;

        bye.extend_from_slice(message.as_bytes());
        bye.extend_from_slice(&[0; 4]);
        Ok(bye)
    }

    #[cfg(feature = "file-transfers")]
    pub async fn direct_connection_send_file(
        &mut self,
        ips: &[String],
        port: u16,
        nonce: &guid_create::GUID,
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
                self.identifier += 1;

                let header = BinaryHeader {
                    session_id: 0,
                    identifier: self.identifier,
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
                .or(Err(P2pError::BinaryHeaderWritingError))?;

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
                        self.identifier += 1;

                        for chunk in file.chunks(1352) {
                            let mut message = BinaryHeader {
                                session_id: self.session_id,
                                identifier: self.identifier,
                                data_offset,
                                total_data_size,
                                length: chunk.len() as u32,
                                flag: 0x1000030,
                                ack_identifier: rng().next_u32(),
                                ack_unique_id: 0,
                                ack_data_size: 0,
                            }
                            .to_bytes()
                            .or(Err(P2pError::BinaryHeaderWritingError))?;

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

                        if let Ok(mut bye) = self.bye(to, from) {
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
}
