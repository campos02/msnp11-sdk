use crate::enums::msnp_list::MsnpList;
use crate::models::personal_message::PersonalMessage;
use crate::models::plain_text::PlainText;
use crate::models::presence::Presence;
use crate::switchboard_server::switchboard::Switchboard;
use std::sync::Arc;

/// Contact and messaging events returned.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum Event {
    /// The Dispatch Server replied with a command asking to connect to another server.
    RedirectedTo { server: String, port: u16 },

    /// Authenticated successfully.
    Authenticated,
    /// GTC value stored in the server.
    Gtc(String),
    /// BLP value stored in the server.
    Blp(String),
    /// User display name stored in the server.
    DisplayName(String),

    /// Contact group
    Group { name: String, guid: String },

    /// Contact not in forward list.
    Contact {
        email: String,
        display_name: String,
        lists: Vec<MsnpList>,
    },

    /// Contact in forward list.
    ContactInForwardList {
        email: String,
        display_name: String,
        guid: String,
        lists: Vec<MsnpList>,
        groups: Vec<String>,
    },

    /// Contact presence information update.
    PresenceUpdate {
        email: String,
        display_name: String,
        presence: Presence,
    },

    /// Contact personal message update.
    PersonalMessageUpdate {
        email: String,
        personal_message: PersonalMessage,
    },

    /// A contact has gone offline.
    ContactOffline { email: String },

    /// Added to someone's forward list.
    AddedBy { email: String, display_name: String },

    /// Removed from someone's forward list.
    RemovedBy(String),

    /// An invitation to a switchboard_server session was accepted.
    SessionAnswered(Arc<Switchboard>),

    /// New text message.
    TextMessage { email: String, message: PlainText },

    /// New nudge.
    Nudge { email: String },

    /// Contact is writing...
    TypingNotification { email: String },

    /// New user joined the switchboard_server.
    ParticipantInSwitchboard { email: String },

    /// A user left the switchboard_server.
    ParticipantLeftSwitchboard { email: String },

    /// A contact's display picture.
    DisplayPicture { email: String, data: Vec<u8> },

    /// Disconnected because the user logged in on another device.
    LoggedInAnotherDevice,
    /// Lost connection to the server.
    Disconnected,
}
