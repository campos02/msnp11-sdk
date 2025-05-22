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
        port: String,
    },
}
