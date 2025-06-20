//! An MSNP11 client SDK. Still a WIP, but messaging and some contact actions have tests that confirm they're working.
//! Other features like display picture transfers are present but still untested.
//! # Login
//! ```
//! use msnp11_sdk::client::Client;
//! use msnp11_sdk::event::Event;
//! use msnp11_sdk::models::personal_message::PersonalMessage;
//!
//! let mut client = Client::new("127.0.0.1".to_string(), "1863".to_string())
//!    .await
//!    .unwrap();
//!
//! client.add_event_handler_closure(|event| { /* Handle events... */ });
//!
//! // Handle a redirection by creating a new connection
//! if let Ok(Event::RedirectedTo { server, port }) = client
//!     .login(
//!         "testing@example.com".to_string(),
//!         "123456".to_string(),
//!         "http://localhost:3000/rdr/pprdr.asp".to_string(),
//!     )
//!     .await
//!  {
//!     client = Client::new(server, port).await.unwrap();
//!     client
//!         .login(
//!             "testing@example.com".to_string(),
//!             "123456".to_string(),
//!             "http://localhost:3000/rdr/pprdr.asp".to_string(),
//!         )
//!         .await
//!         .unwrap();
//!  }
//!
//! client.set_presence("NLN".to_string()).await.unwrap();
//! client
//!     .set_personal_message(&PersonalMessage {
//!         psm: "test".to_string(),
//!         current_media: "".to_string(),
//!     })
//!     .await
//!     .unwrap();
//! ```
//! # Bindings
//! Bindings for Kotlin and Swift can be generated with
//! [UniFFI](https://mozilla.github.io/uniffi-rs/latest/tutorial/foreign_language_bindings.html#multi-crate-workspaces).
//!

pub mod client;
pub mod event;
pub mod event_handler;
mod exports;
mod internal_event;
pub mod models;
pub mod msnp_list;
mod notification_server;
mod passport_auth;
mod receive_split;
pub mod sdk_error;
pub mod switchboard;

uniffi::setup_scaffolding!();

pub use client::Client;
pub use event::Event;
pub use models::personal_message::PersonalMessage;
pub use models::plain_text::PlainText;
pub use models::presence::Presence;
pub use msnp_list::MsnpList;
pub use switchboard::switchboard::Switchboard;
