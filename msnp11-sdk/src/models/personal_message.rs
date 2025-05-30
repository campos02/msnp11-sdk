use serde::{Deserialize, Serialize};

/// A user's personal message. The text itself goes in [psm][PersonalMessage::psm], while [current_media][PersonalMessage::current_media]
/// is used for the song information feature.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, uniffi::Record)]
#[serde(rename = "Data")]
pub struct PersonalMessage {
    #[serde(rename = "PSM")]
    pub psm: String,
    #[serde(rename = "CurrentMedia")]
    pub current_media: String,
}
