use crate::Client;
use crate::sdk_error::SdkError;

/// Builds a new Client instance.
#[uniffi::export]
pub async fn build_client(server: String, port: String) -> Result<Client, SdkError> {
    Client::new(server, port).await
}
