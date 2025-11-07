use crate::enums::msnp_list::MsnpList;
use crate::models::personal_message::PersonalMessage;
use crate::models::plain_text::PlainText;
use crate::models::presence::Presence;
use crate::switchboard_server::switchboard::Switchboard;
use std::sync::Arc;

/// Contact and messaging events.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum Event {
    /// The Dispatch Server replied with a command asking to connect to a Notification Server.
    RedirectedTo { server: String, port: u16 },

    /// Authenticated successfully.
    Authenticated,

    /// GTC value stored in the server.
    Gtc(String),

    /// BLP value stored in the server.
    Blp(String),

    /// User display name stored in the server.
    DisplayName(String),

    /// A contact group
    Group { name: String, guid: String },

    /// A contact not in the forward list.
    Contact {
        email: String,
        display_name: String,
        lists: Vec<MsnpList>,
    },

    /// A contact in the forward list.
    ContactInForwardList {
        email: String,
        display_name: String,
        guid: String,
        lists: Vec<MsnpList>,
        groups: Vec<String>,
    },

    /// Contact presence update sent when setting a user's presence for the first time.
    InitialPresenceUpdate {
        email: String,
        display_name: String,
        presence: Presence,
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

    /// An invitation to a Switchboard session was accepted.
    SessionAnswered(Arc<Switchboard>),

    /// New text message.
    TextMessage { email: String, message: PlainText },

    /// New nudge.
    Nudge { email: String },

    /// Contact is writing...
    TypingNotification { email: String },

    /// New user joined the Switchboard.
    ParticipantInSwitchboard { email: String },

    /// A user left the Switchboard.
    ParticipantLeftSwitchboard { email: String },

    /// A contact's display picture was transferred.
    DisplayPicture { email: String, data: Vec<u8> },

    /// Disconnected because the user logged in on another device.
    LoggedInAnotherDevice,

    /// Lost connection to the server.
    Disconnected,
}
