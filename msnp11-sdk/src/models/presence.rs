/// Represents a contact's presence information.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Presence {
    pub presence: String,
    pub client_id: u64,
    pub msn_object: Option<String>,
}

impl Presence {
    pub(crate) fn new(presence: String, msn_object: Option<String>) -> Self {
        Self {
            presence,
            client_id: 0x40000000,
            msn_object,
        }
    }
}
