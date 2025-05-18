use crate::list::List;
use crate::models::personal_message::PersonalMessage;
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
        id: String,
    },

    Contact {
        email: String,
        display_name: String,
        lists: Vec<List>,
    },

    ContactInForwardList {
        email: String,
        display_name: String,
        id: String,
        lists: Vec<List>,
        groups: Vec<String>,
    },
    
    PresenceUpdate {
        email: String,
        presence: Presence
    },
    
    PersonalMessageUpdate {
        email: String,
        personal_message: PersonalMessage
    },

    Disconnected,
}
