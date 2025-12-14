/// General errors the SDK might return.
#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum SdkError {
    #[error("Could not resolve server name")]
    ResolutionError,
    #[error("This MSNP version is not supported by the server.")]
    ProtocolNotSupported,
    #[error("Authentication error. Check your email and password.")]
    ServerIsBusy,
    #[error("Server returned an error while logging in")]
    ServerError,
    #[error("Authentication error. Check your email and password.")]
    AuthenticationHeaderNotFound,
    #[error("Authentication error. Check your email and password.")]
    CouldNotGetAuthenticationString,
    #[error("An invalid argument was sent")]
    InvalidArgument,
    #[error("Lost connection to the server")]
    Disconnected,
    #[error("Error receiving data")]
    ReceivingError,
    #[error("Error transmitting data")]
    TransmittingError,
    #[error("Could not connect to the server")]
    CouldNotConnectToServer,
    #[error("Could not create runtime")]
    CouldNotCreateRuntime,
    #[error("Could not write user data")]
    CouldNotSetUserData,
    #[error("Please log in before using this command")]
    NotLoggedIn,
    #[error("Command refers to an invalid contact")]
    InvalidContact,
    #[error("The contact you're trying to invite is offline")]
    ContactIsOffline,
    #[error("Error requesting tabs")]
    TabRequestError,
}
