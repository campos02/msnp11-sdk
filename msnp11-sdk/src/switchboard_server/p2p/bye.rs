use crate::P2pError;
use crate::switchboard_server::p2p::binary_header::BinaryHeader;
use deku::DekuContainerWrite;
use rand::{Rng, rng};

pub fn bye(
    to: &str,
    from: &str,
    branch: &guid_create::GUID,
    call_id: &guid_create::GUID,
    identifier: &mut u32,
) -> Result<Vec<u8>, P2pError> {
    let body = "\r\n\0";

    let mut headers = format!("BYE MSNMSGR:{to} MSNSLP/1.0\r\n");
    headers.push_str(format!("To: <msnmsgr:{to}>\r\n").as_str());
    headers.push_str(format!("From: <msnmsgr:{from}>\r\n").as_str());
    headers.push_str(format!("Via: MSNSLP/1.0/TLP ;branch={{{branch}}}\r\n").as_str());
    headers.push_str("CSeq: 0\r\n");
    headers.push_str(format!("Call-ID: {{{call_id}}}\r\n").as_str());
    headers.push_str("Max-Forwards: 0\r\n");
    headers.push_str("Content-Type: application/x-msnmsgr-sessionclosebody\r\n");
    headers.push_str(format!("Content-Length: {}\r\n\r\n", body.len()).as_str());

    let message = format!("{headers}{body}");
    *identifier += 1;

    let mut bye = BinaryHeader {
        session_id: 0,
        identifier: *identifier,
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
