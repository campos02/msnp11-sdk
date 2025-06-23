use crate::enums::event::Event;

/// This trait is used to define an async event handler when using this SDK through foreign language bindings. If using it with Rust
/// the preferred handling method is closures.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: Event);
}
