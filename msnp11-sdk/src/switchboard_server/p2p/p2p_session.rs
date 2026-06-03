use crate::errors::p2p_error::P2pError;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use crate::switchboard_server::p2p::file_context::FileContext;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use core::str;
use deku::{DekuContainerRead, DekuContainerWrite};
use rand::Rng;
use rand::rng;
use std::error::Error;
use std::io::Cursor;

pub struct P2pSession {
    session_id: u32,
    base_identifier: u32,
    branch: String,
    call_id: String,
}

impl P2pSession {
    pub fn new() -> Self {
        Self {
            session_id: 0,
            base_identifier: rng().next_u32(),
            branch: "".to_string(),
            call_id: "".to_string(),
        }
    }

    pub fn new_from_invite(invite: &Vec<u8>) -> Result<Self, Box<dyn Error>> {
        let mut invite = unsafe { str::from_utf8_unchecked(invite.as_slice()) }.split("\r\n");
        let Some(branch) =
            invite.find(|parameter| parameter.starts_with("Via: MSNSLP/1.0/TLP ;branch={"))
        else {
            return Err(P2pError::P2pInvite.into());
        };

        let branch = branch
            .replace("Via: MSNSLP/1.0/TLP ;branch={", "")
            .replace("}", "");

        let Some(call_id) = invite.find(|parameter| parameter.starts_with("Call-ID: {")) else {
            return Err(P2pError::P2pInvite.into());
        };

        let call_id = call_id.replace("Call-ID: {", "").replace("}", "");
        let Some(session_id) = invite.find(|parameter| parameter.starts_with("SessionID: ")) else {
            return Err(P2pError::P2pInvite.into());
        };

        let session_id = session_id.replace("SessionID: ", "").parse::<u32>()?;
        Ok(Self {
            session_id,
            base_identifier: rng().next_u32(),
            branch,
            call_id,
        })
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

        self.invite(to, from, &body)
    }

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

        self.invite(to, from, &body)
    }

    fn invite(&mut self, to: &str, from: &str, body: &str) -> Result<Vec<u8>, P2pError> {
        let branch = guid_create::GUID::rand().to_string();
        let call_id = guid_create::GUID::rand().to_string();

        let mut headers = format!("INVITE MSNMSGR:{to} MSNSLP/1.0\r\n");
        headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
        headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
        headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{branch}}}\r\n").as_str());
        headers.push_str("CSeq: 0\r\n");
        headers.push_str(format!("Call-ID: {{{call_id}}}\r\n").as_str());
        headers.push_str("Max-Forwards: 0\r\n");
        headers.push_str("Content-Type: application/x-msnmsgr-sessionreqbody\r\n");
        headers.push_str(format!("Content-Length: {}\r\n\r\n", body.len()).as_str());

        let message = format!("{headers}{body}");
        let mut invite = BinaryHeader {
            session_id: 0,
            identifier: self.base_identifier,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: self.base_identifier + 1,
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
            ack_identifier: binary_header.identifier + 1,
            ack_unique_id: binary_header.ack_unique_id,
            ack_data_size: binary_header.ack_data_size,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderReadingError))?;

        ack_header.extend_from_slice(&[0; 4]);
        Ok(ack_header)
    }

    pub fn ok(&self, to: &str, from: &str) -> Result<Vec<u8>, Box<dyn Error>> {
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
        let mut ok = BinaryHeader {
            session_id: 0,
            identifier: self.base_identifier + 1,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: self.base_identifier + 1,
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()?;

        ok.extend_from_slice(message.as_bytes());
        ok.extend_from_slice(&[0; 4]);
        Ok(ok)
    }

    pub fn decline(&self, to: &str, from: &str) -> Result<Vec<u8>, Box<dyn Error>> {
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
        let mut decline = BinaryHeader {
            session_id: 0,
            identifier: self.base_identifier + 1,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: self.base_identifier + 1,
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()?;

        decline.extend_from_slice(message.as_bytes());
        decline.extend_from_slice(&[0; 4]);
        Ok(decline)
    }

    pub fn data_preparation(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let message = &[0; 4];
        let mut data_preparation = BinaryHeader {
            session_id: self.session_id,
            identifier: self.base_identifier + 2,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: self.base_identifier + 2,
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()?;

        data_preparation.extend_from_slice(message);
        data_preparation.extend_from_slice(&[0, 0, 0, 1]);
        Ok(data_preparation)
    }

    pub fn data(&self, data: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let mut payloads: Vec<Vec<u8>> = Vec::new();
        let total_data_size = data.len() as u64;
        let mut data_offset = 0u64;

        let chunks = data.chunks(1202);
        for chunk in chunks {
            let mut data_message = BinaryHeader {
                session_id: self.session_id,
                identifier: self.base_identifier + 3,
                data_offset,
                total_data_size,
                length: chunk.len() as u32,
                flag: 0x20,
                ack_identifier: self.base_identifier + 3,
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

    pub fn bye(&self, to: &str, from: &str) -> Result<Vec<u8>, P2pError> {
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
        let mut bye = BinaryHeader {
            session_id: 0,
            identifier: self.base_identifier + 4,
            data_offset: 0,
            total_data_size: message.len() as u64,
            length: message.len() as u32,
            flag: 0x00,
            ack_identifier: self.base_identifier + 4,
            ack_unique_id: 0,
            ack_data_size: 0,
        }
        .to_bytes()
        .or(Err(P2pError::BinaryHeaderReadingError))?;

        bye.extend_from_slice(message.as_bytes());
        bye.extend_from_slice(&[0; 4]);
        Ok(bye)
    }
}
