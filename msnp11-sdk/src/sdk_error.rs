use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, uniffi::Error)]
pub enum SdkError {
    ResolutionError,
    ProtocolNotSupported,
    ServerIsBusy,
    ServerError,
    AuthenticationHeaderNotFound,
    CouldNotGetAuthenticationString,
    InvalidArgument,
    InvalidContact,
    MessageNotDelivered,
    ContactIsOffline,
    NotLoggedIn,
    CouldNotGetParticipants,
    CouldNotInviteContact,
    P2PInviteError,
    CouldNotGetDisplayPicture,
    Disconnected,
    ReceivingError,
    TransmittingError,
    CouldNotSetSessionId,
    CouldNotGetSessionId,
    BinaryHeaderReadingError,
    CouldNotSetUserData,
    CouldNotGetUserData,
    CouldNotConnectToServer,
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

            SdkError::ServerError => write!(f, "Server returned an error when logging in"),

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
                write!(f, "Could not set session ID from switchboard")
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
        }
    }
}

impl Error for SdkError {}
