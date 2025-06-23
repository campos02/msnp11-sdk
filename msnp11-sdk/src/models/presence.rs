use crate::msnp_status::MsnpStatus;

/// Represents a contact's presence information.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Presence {
    pub status: MsnpStatus,
    pub client_id: u64,
    pub msn_object: Option<String>,
}

impl Presence {
    pub(crate) fn new(status: MsnpStatus, msn_object: Option<String>) -> Self {
        Self {
            status,
            client_id: 0x40000000,
            msn_object,
        }
    }
}
