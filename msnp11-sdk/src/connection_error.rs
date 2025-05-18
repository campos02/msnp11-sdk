use std::{error::Error, fmt};

#[derive(Debug, Clone)]
pub enum ConnectionError {
    ResolutionError,
    Disconnected,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectionError::ResolutionError => write!(f, "Could not resolve server name"),
            ConnectionError::Disconnected => write!(f, "Lost connection to the server"),
        }
    }
}

impl Error for ConnectionError {}
