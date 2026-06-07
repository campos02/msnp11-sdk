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

    P2pInvite {
        destination: String,
        message: Vec<u8>,
    },

    P2pShouldAck {
        destination: String,
        message: Vec<u8>,
    },

    P2pOk {
        destination: String,
        message: Vec<u8>,
    },

    P2pDecline {
        destination: String,
        message: Vec<u8>,
    },

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

    P2pBye {
        destination: String,
        message: Vec<u8>,
    },
}
