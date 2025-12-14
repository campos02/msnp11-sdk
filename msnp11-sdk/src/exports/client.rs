use crate::enums::msnp_status::MsnpStatus;
use crate::errors::contact_error::ContactError;
use crate::errors::sdk_error::SdkError;
use crate::event_handler::EventHandler;
use crate::{Event, MsnpList, PersonalMessage, Switchboard, Tab};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Defines the client itself, all Notification Server actions are done through an instance of this struct.
#[derive(uniffi::Object)]
pub struct Client {
    inner: crate::client::Client,
    rt: Runtime,
}

#[uniffi::export]
impl Client {
    /// Connects to the server, defines the channels and returns a new instance.
    #[uniffi::constructor]
    pub fn new(server: &str, port: u16) -> Result<Self, SdkError> {
        let rt = Runtime::new().or(Err(SdkError::CouldNotCreateRuntime))?;
        let client = rt.block_on(async { crate::client::Client::new(server, port).await })?;
        Ok(Self { inner: client, rt })
    }

    /// Adds a new handler that implements the [EventHandler] trait.
    pub fn add_event_handler(&self, handler: Arc<dyn EventHandler>) {
        self.rt
            .block_on(async { self.inner.add_event_handler(handler) })
    }

    /// Does the MSNP authentication process. Also starts regular pings and the handler for Switchboard invitations.
    ///
    /// # Events
    /// If the server you're connecting to implements a Dispatch Server, then this will return a [RedirectedTo][Event::RedirectedTo] event.
    /// What follows is [creating a new][Client::new] client instance with the server and port returned then logging in again, which
    /// will return an [Authenticated][Event::Authenticated] event.
    pub async fn login(
        &self,
        email: String,
        password: &str,
        nexus_url: &str,
        client_name: &str,
        version: &str,
    ) -> Result<Event, SdkError> {
        self.rt.block_on(async {
            self.inner
                .login(email, password, nexus_url, client_name, version)
                .await
        })
    }

    #[cfg(feature = "tabs")]
    /// Makes a request to get the tabs and returns them.
    pub async fn get_tabs(&self, config_url: &str) -> Result<Vec<Tab>, SdkError> {
        self.rt
            .block_on(async { self.inner.get_tabs(config_url).await })
    }

    /// Sets the user's presence status.
    pub async fn set_presence(&self, presence: MsnpStatus) -> Result<(), SdkError> {
        self.inner.set_presence(presence).await
    }

    /// Sets the user's personal message.
    pub async fn set_personal_message(
        &self,
        personal_message: &PersonalMessage,
    ) -> Result<(), SdkError> {
        self.inner.set_personal_message(personal_message).await
    }

    /// Sets the user's display name.
    pub async fn set_display_name(&self, display_name: &str) -> Result<(), SdkError> {
        self.inner.set_display_name(display_name).await
    }

    /// Sets a contact's display name.
    pub async fn set_contact_display_name(
        &self,
        guid: &str,
        display_name: &str,
    ) -> Result<(), ContactError> {
        self.inner
            .set_contact_display_name(guid, display_name)
            .await
    }

    /// Adds a contact to a specified list, also setting its display name if applicable.
    pub async fn add_contact(
        &self,
        email: &str,
        display_name: &str,
        list: MsnpList,
    ) -> Result<Event, ContactError> {
        self.inner.add_contact(email, display_name, list).await
    }

    /// Removes a contact from a specified list (except the forward list, which requires calling
    /// [remove_contact_from_forward_list][Client::remove_contact_from_forward_list]).
    pub async fn remove_contact(&self, email: &str, list: MsnpList) -> Result<(), ContactError> {
        self.inner.remove_contact(email, list).await
    }

    /// Removes a contact from the forward list.
    pub async fn remove_contact_from_forward_list(&self, guid: &str) -> Result<(), ContactError> {
        self.inner.remove_contact_from_forward_list(guid).await
    }

    /// Blocks a contact.
    pub async fn block_contact(&self, email: &str) -> Result<(), ContactError> {
        self.inner.block_contact(email).await
    }

    /// Unblocks a contact.
    pub async fn unblock_contact(&self, email: &str) -> Result<(), ContactError> {
        self.inner.unblock_contact(email).await
    }

    /// Creates a new contact group.
    pub async fn create_group(&self, name: &str) -> Result<(), ContactError> {
        self.inner.create_group(name).await
    }

    /// Deletes a contact group.
    pub async fn delete_group(&self, guid: &str) -> Result<(), ContactError> {
        self.inner.delete_group(guid).await
    }

    /// Renames a contact group.
    pub async fn rename_group(&self, guid: &str, new_name: &str) -> Result<(), ContactError> {
        self.inner.rename_group(guid, new_name).await
    }

    /// Adds a contact to a group.
    pub async fn add_contact_to_group(
        &self,
        guid: &str,
        group_guid: &str,
    ) -> Result<(), ContactError> {
        self.inner.add_contact_to_group(guid, group_guid).await
    }

    /// Removes a contact from a group.
    pub async fn remove_contact_from_group(
        &self,
        guid: &str,
        group_guid: &str,
    ) -> Result<(), ContactError> {
        self.inner.remove_contact_from_group(guid, group_guid).await
    }

    /// Sets the GTC value, which can be either `A` or `N`.
    pub async fn set_gtc(&self, gtc: &str) -> Result<(), SdkError> {
        self.inner.set_gtc(gtc).await
    }

    /// Sets the GTC value, which can be either `AL` or `BL`.
    pub async fn set_blp(&self, blp: &str) -> Result<(), SdkError> {
        self.inner.set_blp(blp).await
    }

    /// Creates and returns a new Switchboard session with the specified contact.
    pub async fn create_session(&self, email: &str) -> Result<Switchboard, SdkError> {
        self.rt
            .block_on(async { self.inner.create_session(email).await })
    }

    /// Sets the user's display picture, returning a standard base64 encoded hash of it.
    /// This method uses the picture's binary data, and scaling down beforehand to a size like 200x200 is recommended.
    pub async fn set_display_picture(&self, display_picture: Vec<u8>) -> Result<String, SdkError> {
        self.inner.set_display_picture(display_picture).await
    }

    /// Disconnects from the server.
    pub async fn disconnect(&self) -> Result<(), SdkError> {
        self.inner.disconnect().await
    }
}
