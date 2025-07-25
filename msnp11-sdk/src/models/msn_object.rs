use serde::{Deserialize, Serialize};

/// Serializable msnobject representing data like display pictures.
#[derive(Serialize, Deserialize, Clone, Debug, uniffi::Record)]
#[serde(rename = "msnobject")]
pub struct MsnObject {
    #[serde(rename = "@Creator")]
    pub creator: String,
    #[serde(rename = "@Size")]
    pub size: u32,
    #[serde(rename = "@Type")]
    pub object_type: u16,
    #[serde(rename = "@Location")]
    pub location: String,
    #[serde(rename = "@Friendly")]
    pub friendly: String,
    #[serde(rename = "@SHA1D")]
    pub sha1d: String,
    #[serde(rename = "@SHA1C")]
    pub sha1c: String,
}
