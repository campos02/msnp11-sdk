/// Errors when sending messages.
#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum MessagingError {
    #[error("Error receiving data")]
    ReceivingError,
    #[error("Error transmitting data")]
    TransmittingError,
    #[error("Could not get session ID")]
    CouldNotGetSessionId,
    #[error("Message could not be delivered to all recipients")]
    MessageNotDelivered,
}
