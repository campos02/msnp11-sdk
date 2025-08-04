//! An MSNP11 client SDK.
//! # Login
//! ```
//! use msnp11_sdk::client::Client;
//! use msnp11_sdk::enums::event::Event;
//! use msnp11_sdk::models::personal_message::PersonalMessage;
//! use msnp11_sdk::enums::msnp_status::MsnpStatus;
//!
//! let mut client = Client::new("127.0.0.1", 1863)
//!    .await
//!    .unwrap();
//!
//! client.add_event_handler_closure(|event| async { /* Handle events... */ });
//!
//! // Handle a redirection by creating a new connection
//! if let Ok(Event::RedirectedTo { server, port }) = client
//!     .login(
//!         "testing@example.com".to_string(),
//!         "123456",
//!         "http://localhost:3000/rdr/pprdr.asp",
//!         "msnp11-sdk",
//!         "0.7"
//!     )
//!     .await
//!  {
//!     client = Client::new(&*server, port).await.unwrap();
//!     client
//!         .login(
//!             "testing@example.com".to_string(),
//!             "123456",
//!             "http://localhost:3000/rdr/pprdr.asp",
//!             "msnp11-sdk",
//!             "0.7"
//!         )
//!         .await
//!         .unwrap();
//!  }
//!
//! client.set_presence(MsnpStatus::Online).await.unwrap();
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
pub mod enums;
pub mod event_handler;
mod exports;
mod internal_event;
pub mod models;
mod notification_server;
mod passport_auth;
mod receive_split;
pub mod sdk_error;
pub mod switchboard_server;
mod user_data;

uniffi::setup_scaffolding!();

pub use client::Client;
pub use enums::event::Event;
pub use enums::msnp_list::MsnpList;
pub use enums::msnp_status::MsnpStatus;
pub use models::msn_object::MsnObject;
pub use models::personal_message::PersonalMessage;
pub use models::plain_text::PlainText;
pub use models::presence::Presence;
pub use switchboard_server::switchboard::Switchboard;
