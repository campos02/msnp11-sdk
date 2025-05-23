use crate::list::List;
use crate::models::personal_message::PersonalMessage;
use crate::models::plain_text::PlainText;
use crate::models::presence::Presence;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ServerReply,

    RedirectedTo {
        server: String,
        port: String,
    },

    Authenticated,
    Gtc(String),
    Blp(String),
    DisplayName(String),

    Group {
        name: String,
        guid: String,
    },

    Contact {
        email: String,
        display_name: String,
        lists: Vec<List>,
    },

    ContactInForwardList {
        email: String,
        display_name: String,
        guid: String,
        lists: Vec<List>,
        groups: Vec<String>,
    },

    PresenceUpdate {
        email: String,
        display_name: String,
        presence: Presence,
    },

    PersonalMessageUpdate {
        email: String,
        personal_message: PersonalMessage,
    },

    ContactOffline {
        email: String,
    },

    AddedBy {
        email: String,
        display_name: String,
    },

    RemovedBy(String),

    TextMessage {
        session_id: String,
        email: String,
        message: PlainText,
    },

    Nudge {
        session_id: String,
        email: String,
    },

    TypingNotification {
        session_id: String,
        email: String,
    },

    ParticipantInSwitchboard {
        session_id: String,
        email: String,
    },

    ParticipantLeftSwitchboard {
        session_id: String,
        email: String,
    },

    LoggedInAnotherDevice,
    Disconnected,
}
