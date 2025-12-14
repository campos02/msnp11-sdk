use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, uniffi::Record)]
pub struct Tab {
    #[serde(rename = "type")]
    pub tab_type: String,
    #[serde(rename = "contenturl")]
    pub content_url: String,
    #[serde(rename = "hiturl")]
    pub hit_url: String,
    pub image: String,
    pub name: String,
    pub tooltip: String,
    #[serde(rename = "siteid")]
    pub site_id: String,
    #[serde(rename = "notificationid")]
    pub notification_id: String,
}
