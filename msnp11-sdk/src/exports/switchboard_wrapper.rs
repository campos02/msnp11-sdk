use crate::event_handler::EventHandler;
use crate::sdk_error::SdkError;
use crate::{PlainText, Switchboard};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Wraps an obtained [Switchboard][crate::switchboard_server::switchboard_server::Switchboard] object for use outside a tokio main.
/// [Switchboard][crate::switchboard_server::switchboard_server::Switchboard] represents a messaging session with one or more
/// contacts. The official MSNP clients usually create a new session every time a conversation
/// window is opened and leave it when it's closed.
#[derive(uniffi::Object)]
pub struct SwitchboardWrapper {
    inner: Arc<Switchboard>,
    rt: Runtime,
}

#[uniffi::export]
impl SwitchboardWrapper {
    /// Create new wrapper instance with an obtained [Switchboard][crate::switchboard_server::switchboard_server::Switchboard].
    #[uniffi::constructor]
    pub fn new(switchboard: Arc<Switchboard>) -> Result<Self, SdkError> {
        let rt = Runtime::new().or(Err(SdkError::CouldNotCreateRuntime))?;
        Ok(Self {
            inner: switchboard,
            rt,
        })
    }

    /// Adds a new handler that implements the [EventHandler] trait.
    pub fn add_event_handler(&self, handler: Arc<dyn EventHandler>) {
        self.rt
            .block_on(async { self.inner.add_event_handler(handler) })
    }

    /// Invites a new contact to this switchboard_server session. This makes them temporary chat rooms.
    pub async fn invite(&self, email: &str) -> Result<(), SdkError> {
        self.inner.invite(email).await
    }

    /// Returns the session ID, if it's defined.
    pub fn get_session_id(&self) -> Result<String, SdkError> {
        self.inner.get_session_id()
    }

    /// Sends a plain text message to the session.
    pub async fn send_text_message(&self, message: &PlainText) -> Result<(), SdkError> {
        self.inner.send_text_message(message).await
    }

    /// Sends a nudge to the session.
    pub async fn send_nudge(&self) -> Result<(), SdkError> {
        self.inner.send_nudge().await
    }

    /// Sends an "is writing..." notification to the session.
    pub async fn send_typing_user(&self, email: &str) -> Result<(), SdkError> {
        self.inner.send_typing_user(email).await
    }

    /// Requests the contact's display picture and handles the transfer process.
    pub async fn request_contact_display_picture(
        &self,
        email: &str,
        msn_object: &str,
    ) -> Result<(), SdkError> {
        self.inner
            .request_contact_display_picture(email, msn_object)
            .await
    }

    /// Disconnects from the server.
    pub async fn disconnect(&self) -> Result<(), SdkError> {
        self.inner.disconnect().await
    }
}
