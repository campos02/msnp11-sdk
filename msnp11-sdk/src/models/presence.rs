use crate::enums::msnp_status::MsnpStatus;
use crate::models::msn_object::MsnObject;

/// Represents a contact's presence information.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Presence {
    pub status: MsnpStatus,
    pub client_id: u64,
    pub msn_object: Option<MsnObject>,
    pub msn_object_string: Option<String>,
}

impl Presence {
    pub(crate) fn new_without_object(status: MsnpStatus) -> Self {
        Self {
            status,
            client_id: 0x40000000,
            msn_object: None,
            msn_object_string: None,
        }
    }
}
