use tokio::net::TcpListener;

pub struct DirectConnectionOkResult {
    pub ok: Vec<u8>,
    pub nonce: guid_create::GUID,
    pub ipv4_listener: Option<TcpListener>,
    pub ipv6_listener: Option<TcpListener>,
}
