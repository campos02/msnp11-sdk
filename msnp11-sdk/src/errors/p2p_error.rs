use std::fmt;

#[derive(Debug)]
pub enum P2pError {
    /// P2P session kind not supported.
    P2pInvite,

    /// Invite is for a different user.
    OtherDestination,

    /// Could not get session data.
    CouldNotGetSessionData,

    /// Invite has an invalid context
    OtherContext,
}

impl fmt::Display for P2pError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            P2pError::P2pInvite => write!(f, "P2P session kind not supported"),
            P2pError::OtherDestination => write!(f, "Invite is for a different user"),
            P2pError::CouldNotGetSessionData => write!(f, "Could get session data"),
            P2pError::OtherContext => write!(f, "Invite has an invalid context"),
        }
    }
}

impl std::error::Error for P2pError {}
