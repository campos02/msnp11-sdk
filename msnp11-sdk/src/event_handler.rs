use crate::event::Event;

#[uniffi::export]
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: Event);
}
