use std::error::Error;
use std::fmt;

/// Errors the SDK might return.
#[derive(Debug, Clone, uniffi::Error)]
pub enum SdkError {
    /// Could not resolve server name.
    ResolutionError,
    /// This MSNP version is not supported by the server.
    ProtocolNotSupported,
    /// Authentication error. Check your email and password.
    ServerIsBusy,
    /// Server returned an error while logging in.
    ServerError,
    /// Authentication error. Check your email and password.
    AuthenticationHeaderNotFound,
    /// Authentication error. Check your email and password.
    CouldNotGetAuthenticationString,
    /// An invalid argument was sent.
    InvalidArgument,
    /// Command refers to an invalid contact.
    InvalidContact,
    /// Message could not be delivered to all recipients.
    MessageNotDelivered,
    /// The contact you're trying to invite is offline.
    ContactIsOffline,
    /// Please log in before using this command.
    NotLoggedIn,
    /// Could not get session participants.
    CouldNotGetParticipants,
    /// Could not invite contact to session.
    CouldNotInviteContact,
    /// P2P session kind not supported.
    P2PInviteError,
    /// Could not get contact display picture.
    CouldNotGetDisplayPicture,
    /// Lost connection to the server.
    Disconnected,
    /// Error receiving data.
    ReceivingError,
    /// Error transmitting data.
    TransmittingError,
    /// Could not get session ID.
    CouldNotSetSessionId,
    /// Could not get session ID.
    CouldNotGetSessionId,
    /// Could not read binary header from P2P message.
    BinaryHeaderReadingError,
    /// Could not write user data.
    CouldNotSetUserData,
    /// Could not retrieve user data.
    CouldNotGetUserData,
    /// Could not connect to the server.
    CouldNotConnectToServer,
    /// Could not create runtime.
    CouldNotCreateRuntime,
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SdkError::ResolutionError => write!(f, "Could not resolve server name"),

            SdkError::ProtocolNotSupported => {
                write!(f, "This MSNP version is not supported by the server")
            }

            SdkError::ServerIsBusy
            | SdkError::AuthenticationHeaderNotFound
            | SdkError::CouldNotGetAuthenticationString => {
                write!(f, "Authentication error. Check your email and password")
            }

            SdkError::ServerError => write!(f, "Server returned an error while logging in"),

            SdkError::InvalidArgument => write!(f, "An invalid argument was sent"),

            SdkError::InvalidContact => write!(f, "Command refers to an invalid contact"),

            SdkError::MessageNotDelivered => {
                write!(f, "Message could not be delivered to all recipients")
            }

            SdkError::ContactIsOffline => {
                write!(f, "The contact you're trying to invite is offline")
            }

            SdkError::NotLoggedIn => write!(f, "Please log in before using this command"),

            SdkError::CouldNotGetParticipants => write!(f, "Could not get session participants"),

            SdkError::CouldNotInviteContact => write!(f, "Could not invite contact to session"),

            SdkError::P2PInviteError => write!(f, "P2P session kind not supported"),

            SdkError::CouldNotGetDisplayPicture => {
                write!(f, "Could not get contact display picture")
            }

            SdkError::Disconnected => write!(f, "Lost connection to the server"),

            SdkError::ReceivingError => write!(f, "Error receiving data"),

            SdkError::TransmittingError => write!(f, "Error transmitting data"),

            SdkError::CouldNotSetSessionId => {
                write!(f, "Could not set session ID from switchboard_server")
            }

            SdkError::CouldNotGetSessionId => {
                write!(f, "Could not get session ID")
            }

            SdkError::BinaryHeaderReadingError => {
                write!(f, "Could not read binary header from P2P message")
            }

            SdkError::CouldNotGetUserData => write!(f, "Could not retrieve user data"),

            SdkError::CouldNotSetUserData => write!(f, "Could not write user data"),

            SdkError::CouldNotConnectToServer => write!(f, "Could not connect to the server"),

            SdkError::CouldNotCreateRuntime => write!(f, "Could not create runtime"),
        }
    }
}

impl Error for SdkError {}
