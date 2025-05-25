use crate::list::List;
use crate::models::personal_message::PersonalMessage;
use crate::models::plain_text::PlainText;
use crate::models::presence::Presence;
use crate::switchboard::switchboard::Switchboard;

#[derive(Debug)]
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
    SessionAnswered(Switchboard),

    TextMessage {
        email: String,
        message: PlainText,
    },

    Nudge {
        email: String,
    },

    TypingNotification {
        email: String,
    },

    ParticipantInSwitchboard {
        email: String,
    },

    ParticipantLeftSwitchboard {
        email: String,
    },

    DisplayPicture {
        email: String,
        data: Vec<u8>,
    },

    LoggedInAnotherDevice,
    Disconnected,
}
