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

    P2PInvite {
        destination: String,
        message: Vec<u8>,
    },

    P2POk {
        destination: String,
        message: Vec<u8>,
    },

    P2PDataPreparation {
        destination: String,
        message: Vec<u8>,
    },

    P2PData {
        destination: String,
        message: Vec<u8>,
    },

    P2PBye {
        destination: String,
        message: Vec<u8>,
    },
}
