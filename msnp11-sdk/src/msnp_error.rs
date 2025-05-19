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
        }
    }
}

impl Error for MsnpError {}
