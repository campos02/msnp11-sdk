use crate::enums::internal_event::InternalEvent;
use crate::errors::p2p_error::P2pError;
use crate::models::user_data::UserData;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
#[cfg(feature = "file-transfers")]
use crate::switchboard_server::p2p::file_context::FileContext;
use crate::switchboard_server::p2p::{direct_connection, send_display_picture};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use core::str;
use deku::{DekuContainerRead, DekuContainerWrite};
#[cfg(feature = "file-transfers")]
use getifs::interface_addrs;
use rand::Rng;
use rand::rng;
use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use tokio::sync::RwLock;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;

pub struct P2pSession {
    session_id: u32,
    identifier: u32,
    branch: guid_create::GUID,
    call_id: guid_create::GUID,
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

    pub fn new_from_invite(invite: &Vec<u8>) -> Result<Self, Box<dyn Error>> {
        let mut invite = unsafe { str::from_utf8_unchecked(invite.as_slice()) }.split("\r\n");
        let Some(branch) =
            invite.find(|parameter| parameter.starts_with("Via: MSNSLP/1.0/TLP ;branch={"))
        else {
            return Err(P2pError::P2pInvite.into());
        };

        let Ok(branch) = guid_create::GUID::parse(
            &*branch
                .replace("Via: MSNSLP/1.0/TLP ;branch={", "")
                .replace("}", ""),
        ) else {
            return Err(P2pError::P2pInvite.into());
        };

        let Some(call_id) = invite.find(|parameter| parameter.starts_with("Call-ID: {")) else {
            return Err(P2pError::P2pInvite.into());
        };

        let Ok(call_id) =
            guid_create::GUID::parse(&*call_id.replace("Call-ID: {", "").replace("}", ""))
        else {
            return Err(P2pError::P2pInvite.into());
        };

        let Some(session_id) = invite.find(|parameter| parameter.starts_with("SessionID: ")) else {
            return Err(P2pError::P2pInvite.into());
        };

        let session_id = session_id.replace("SessionID: ", "").parse::<u32>()?;
        Ok(Self {
            session_id,
            identifier: rng().next_u32(),
            branch,
            call_id,
        })
    }

    pub async fn handle_display_picture_invite(
        destination: String,
        invite: Vec<u8>,
        user_data: Arc<RwLock<UserData>>,
        command_internal_rx: &mut Receiver<InternalEvent>,
        tr_id: Arc<AtomicU32>,
        sb_tx: Sender<Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        send_display_picture::handle_invite(
            destination,
            invite,
            user_data,
            command_internal_rx,
            tr_id,
            sb_tx,
        )
        .await
    }

    pub async fn handle_display_picture_bye(
        destination: String,
        bye: Vec<u8>,
        user_data: Arc<RwLock<UserData>>,
        command_internal_rx: &mut Receiver<InternalEvent>,
        tr_id: Arc<AtomicU32>,
        sb_tx: Sender<Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        send_display_picture::handle_bye(
            destination,
            bye,
            user_data,
            command_internal_rx,
            tr_id,
            sb_tx,
        )
        .await
    }

    pub fn picture_invite(
        &mut self,
        to: &str,
        from: &str,
        msn_object: &str,
    ) -> Result<Vec<u8>, P2pError> {
        let mut body = "EUF-GUID: {A4268EEC-FEC5-49E5-95C3-F126696BDBF6}\r\n".to_string();
        body.push_str(format!("SessionID: {}\r\n", rng().next_u32()).as_str());
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
        let context = FileContext {
            size: 574,
            second_field: 2,
            file_size,
            preview: 1,
            file_name: utf16_file_name,
        }
        .to_bytes()
        .or(Err(P2pError::InviteError))?;

        let mut body = "EUF-GUID: {5D3E02AB-6190-11D3-BBBB-00C04F795683}\r\n".to_string();
        body.push_str(format!("SessionID: {}\r\n", rng().next_u32()).as_str());
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

        let ips = interface_addrs().or(Err(P2pError::CouldNotGetIpAddress))?;
        let ip = ips.first().ok_or(P2pError::CouldNotGetIpAddress)?;

        if ip.addr().is_ipv6() {
            body.push_str(&format!("IPv6-global: {}\r\n", ip.addr()));
        }

        let nonce = guid_create::GUID::rand();
        body.push_str(&format!("Nonce: {{{}}}\r\n\r\n\0", &nonce));

        self.invite(
            to,
            from,
            &body,
            self.call_id.clone(),
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

    pub fn ok(&mut self, to: &str, from: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut body = "EUF-GUID: {A4268EEC-FEC5-49E5-95C3-F126696BDBF6}\r\n".to_string();
        body.push_str(format!("SessionID: {}\r\n\r\n\0", self.session_id).as_str());

        let mut headers = "MSNSLP/1.0 200 OK\r\n".to_string();
        headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
        headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
        headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{}}}\r\n", self.branch).as_str());
        headers.push_str("CSeq: 1\r\n");
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
        .to_bytes()?;

        ok.extend_from_slice(message.as_bytes());
        ok.extend_from_slice(&[0; 4]);
        Ok(ok)
    }

    pub fn decline(&mut self, to: &str, from: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let body = format!("SessionID: {}\r\n\r\n\0", self.session_id);

        let mut headers = "MSNSLP/1.0 603 Decline\r\n".to_string();
        headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
        headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
        headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{}}}\r\n", self.branch).as_str());
        headers.push_str("CSeq: 1\r\n");
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
        .to_bytes()?;

        decline.extend_from_slice(message.as_bytes());
        decline.extend_from_slice(&[0; 4]);
        Ok(decline)
    }

    pub fn data_preparation(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
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
        .to_bytes()?;

        data_preparation.extend_from_slice(message);
        data_preparation.extend_from_slice(&[0, 0, 0, 1]);
        Ok(data_preparation)
    }

    pub fn data(&mut self, data: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
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
                flag: 0x20,
                ack_identifier: rng().next_u32(),
                ack_unique_id: 0,
                ack_data_size: 0,
            }
            .to_bytes()?;

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
        headers.push_str("CSeq: 0\r\n");
        headers.push_str(format!("Call-ID: {{{}}}\r\n", self.call_id).as_str());
        headers.push_str("Max-Forwards: 0\r\n");
        headers.push_str("Content-Type: application/x-msnmsgr-sessionreqbody\r\n");
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

    pub async fn direct_connection_send_file(
        &mut self,
        ips: &[String],
        port: u16,
        nonce: &guid_create::GUID,
        file: &[u8],
    ) -> Result<(), P2pError> {
        direct_connection::send_file(
            ips,
            port,
            nonce,
            self.session_id,
            &mut self.identifier,
            file,
        )
        .await
    }
}
