#[cfg(feature = "file-transfers")]
use crate::switchboard_server::p2p::binary_header::BinaryHeader;

#[derive(Debug, Clone)]
pub(crate) enum InternalEvent {
    ServerReply(String),
    SwitchboardInvitation {
        server: String,
        port: String,
        session_id: String,
        cki_string: String,
    },

    GotAuthorizationString(String),
    RedirectedTo {
        server: String,
        port: u16,
    },

    DisplayPictureInvite {
        to: String,
        from: String,
        branch: guid_create::GUID,
        call_id: guid_create::GUID,
        session_id: u32,
        context: String,
        message: Vec<u8>,
    },

    #[cfg(feature = "file-transfers")]
    FileTransferInvite {
        to: String,
        from: String,
        branch: guid_create::GUID,
        call_id: guid_create::GUID,
        session_id: u32,
        file_size: u64,
        file_name: String,
        message: Vec<u8>,
    },

    #[cfg(feature = "file-transfers")]
    P2pDirectConnectionInvite {
        to: String,
        branch: guid_create::GUID,
        call_id: guid_create::GUID,
        message: Vec<u8>,
    },

    P2pShouldAck {
        destination: String,
        message: Vec<u8>,
    },

    #[cfg(feature = "file-transfers")]
    P2pOk {
        destination: String,
        message: Vec<u8>,
    },

    #[cfg(feature = "file-transfers")]
    P2pDecline {
        destination: String,
        message: Vec<u8>,
    },

    #[cfg(feature = "file-transfers")]
    P2pDirectConnectionOk {
        destination: String,
        message: Vec<u8>,
        bridge: String,
        listening: bool,
        nonce: guid_create::GUID,
        ips: Vec<String>,
        port: u16,
    },

    P2pData {
        destination: String,
        message: Vec<u8>,
    },

    #[cfg(feature = "file-transfers")]
    P2pDirectConnectionData {
        binary_header: BinaryHeader,
        data: Vec<u8>,
    },

    P2pBye {
        to: String,
        from: String,
        message: Vec<u8>,
    },
}
