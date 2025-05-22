use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum MsnpError {
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
}

impl fmt::Display for MsnpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MsnpError::ProtocolNotSupported => {
                write!(f, "This MSNP version is not supported by the server")
            }

            MsnpError::ServerIsBusy
            | MsnpError::AuthenticationHeaderNotFound
            | MsnpError::CouldNotGetAuthenticationString => {
                write!(f, "Authentication error. Check your email and password")
            }

            MsnpError::ServerError => {
                write!(f, "Server returned an error when logging in")
            }

            MsnpError::InvalidArgument => {
                write!(f, "An invalid argument was sent")
            }

            MsnpError::InvalidContact => {
                write!(f, "Command refers to an invalid contact")
            }

            MsnpError::MessageNotDelivered => {
                write!(f, "Message could not be delivered to all recipients")
            }

            MsnpError::ContactIsOffline => {
                write!(f, "The contact you're trying to invite is offline")
            }

            MsnpError::NotLoggedIn => {
                write!(f, "Please log in before using this command")
            }

            MsnpError::CouldNotGetParticipants => {
                write!(f, "Could not get session participants")
            }
        }
    }
}

impl Error for MsnpError {}
