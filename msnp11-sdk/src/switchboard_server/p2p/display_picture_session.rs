use crate::sdk_error::SdkError;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use core::str;
use deku::{DekuContainerRead, DekuContainerWrite};
use rand::RngCore;
use rand::rng;
use std::error::Error;
use std::io::Cursor;

pub struct DisplayPictureSession {
    session_id: u32,
    base_identifier: u32,
    branch: String,
    call_id: String,
}

impl DisplayPictureSession {
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
            return Err(SdkError::P2PInviteError.into());
        };
        let branch = branch
            .replace("Via: MSNSLP/1.0/TLP ;branch={", "")
            .replace("}", "");

        let Some(call_id) = invite.find(|parameter| parameter.starts_with("Call-ID: {")) else {
            return Err(SdkError::P2PInviteError.into());
        };
        let call_id = call_id.replace("Call-ID: {", "").replace("}", "");

        let Some(session_id) = invite.find(|parameter| parameter.starts_with("SessionID: ")) else {
            return Err(SdkError::P2PInviteError.into());
        };
        let session_id = session_id.replace("SessionID: ", "").parse::<u32>()?;

        Ok(Self {
            session_id,
            base_identifier: rng().next_u32(),
            branch,
            call_id,
        })
    }

    pub fn invite(&mut self, to: &str, from: &str, msn_object: &str) -> Result<Vec<u8>, SdkError> {
        let branch = guid_create::GUID::rand().to_string();
        let call_id = guid_create::GUID::rand().to_string();
        let session_id = rng().next_u32();

        let mut body = "EUF-GUID: {A4268EEC-FEC5-49E5-95C3-F126696BDBF6}\r\n".to_string();
        body.push_str(format!("SessionID: {session_id}\r\n").as_str());
        body.push_str("AppID: 1\r\n");
        body.push_str(
            format!(
                "Context: {}\r\n\r\n\0",
                STANDARD.encode((msn_object.to_owned() + "\0").as_bytes())
            )
            .as_str(),
        );

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
        .or(Err(SdkError::BinaryHeaderReadingError))?;

        self.branch = branch;
        self.call_id = call_id;

        invite.extend_from_slice(message.as_bytes());
        invite.extend_from_slice(&[0; 4]);
        Ok(invite)
    }

    pub fn acknowledge(payload: &[u8]) -> Result<Vec<u8>, SdkError> {
        let binary_header = &payload[..48];
        let mut cursor = Cursor::new(binary_header);

        let (_, binary_header) = BinaryHeader::from_reader((&mut cursor, 0))
            .or(Err(SdkError::BinaryHeaderReadingError))?;

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
        .or(Err(SdkError::BinaryHeaderReadingError))?;

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

    pub fn bye(&self, to: &str, from: &str) -> Result<Vec<u8>, SdkError> {
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
        .or(Err(SdkError::BinaryHeaderReadingError))?;

        bye.extend_from_slice(message.as_bytes());
        bye.extend_from_slice(&[0; 4]);
        Ok(bye)
    }
}
