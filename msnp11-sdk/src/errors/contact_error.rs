/// Errors when performing contact actions.
#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum ContactError {
    #[error("Server returned an error while logging in")]
    ServerError,
    #[error("An invalid argument was sent")]
    InvalidArgument,
    #[error("Command refers to an invalid contact")]
    InvalidContact,
    #[error("Error receiving data")]
    ReceivingError,
    #[error("Error transmitting data")]
    TransmittingError,
    #[error("The contact you're trying to invite is offline")]
    ContactIsOffline,
}
