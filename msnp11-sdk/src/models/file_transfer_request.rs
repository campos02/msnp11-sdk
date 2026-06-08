#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FileTransferRequest {
    pub(crate) to: String,
    pub(crate) from: String,
    pub(crate) branch: String,
    pub(crate) call_id: String,
    pub(crate) session_id: u32,
}
