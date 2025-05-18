#[derive(Debug, Clone, PartialEq)]
pub struct Presence {
    pub presence: String,
    pub client_id: u64,
    pub msn_object: Option<String>,
}

impl Presence {
    pub fn new(presence: String, msn_object: Option<String>) -> Self {
        Self {
            presence,
            client_id: 0x40000000,
            msn_object,
        }
    }
}
