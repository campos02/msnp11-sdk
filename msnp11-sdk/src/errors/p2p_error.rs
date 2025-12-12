/// Errors when using P2P features like display picture transfers.
#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum P2pError {
    #[error("P2P session kind not supported")]
    P2pInvite,
    #[error("Invite is for a different user")]
    OtherDestination,
    #[error("Could get session data")]
    CouldNotGetSessionData,
    #[error("Invite has an invalid context")]
    OtherContext,
    #[error("Could not read binary header from P2P message")]
    BinaryHeaderReadingError,
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
}
