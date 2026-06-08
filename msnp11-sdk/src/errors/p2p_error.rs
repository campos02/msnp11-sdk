/// Errors when using P2P features like display picture transfers.
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
pub enum P2pError {
    #[error("P2P session kind not supported")]
    P2pInvite,
    #[error("Request is for a different user")]
    OtherDestination,
    #[error("Could get session data")]
    CouldNotGetSessionData,
    #[error("Request has an invalid context")]
    OtherContext,
    #[error("Could not read binary header from P2P message")]
    BinaryHeaderReadingError,
    #[error("Could not create binary header")]
    BinaryHeaderWritingError,
    #[error("Message could not be delivered to all recipients")]
    MessageNotDelivered,
    #[error("Please log in before using this command")]
    NotLoggedIn,
    #[error("Error transmitting data")]
    TransmittingError,
    #[error("Error receiving data")]
    ReceivingError,
    #[error("P2P session kind not supported")]
    InviteError,
    #[error("Could not retrieve user data")]
    CouldNotGetUserData,
    #[error("Could not get contact display picture")]
    CouldNotGetDisplayPicture,
    #[error("Could not get device IP address")]
    CouldNotGetIpAddress,
    #[error("Could not send through a direct connection")]
    CouldNotSendThroughDirectConnection,
    #[error("Could not send file")]
    CouldNotSendFile,
    #[error("File transfer was cancelled")]
    FileTransferCancelled,
    #[error("File transfer was declined")]
    FileTransferDeclined,
}
