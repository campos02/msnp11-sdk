#[derive(Debug, Clone)]
pub(crate) enum InternalEvent {
    ServerReply(String),
    GotAuthorizationString(String),
    RedirectedTo { server: String, port: String },
}
