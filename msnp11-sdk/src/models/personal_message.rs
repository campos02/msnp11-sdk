use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, uniffi::Record)]
#[serde(rename = "Data")]
pub struct PersonalMessage {
    #[serde(rename = "PSM")]
    pub psm: String,
    #[serde(rename = "CurrentMedia")]
    pub current_media: String,
}
