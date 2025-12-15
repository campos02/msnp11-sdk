use crate::http::xml::tab::Tab;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct MsgrConfig {
    #[serde(rename = "TabConfig")]
    pub tab_config: TabConfig,
    #[serde(rename = "LocalizedConfig")]
    pub localized_config: LocalizedConfig,
}

#[derive(Deserialize)]
pub(crate) struct TabConfig {
    #[serde(rename = "msntabdata")]
    pub msn_tab_data: MsnTabData,
}

#[derive(Deserialize)]
pub(crate) struct LocalizedConfig {
    #[serde(rename = "MsnTodayConfig")]
    pub msn_today_config: MsnTodayConfig,
}

#[derive(Deserialize)]
pub(crate) struct MsnTodayConfig {
    #[serde(rename = "MsnTodayURL")]
    pub msn_today_url: String,
}

#[derive(Deserialize)]
pub(crate) struct MsnTabData {
    pub tab: Vec<Tab>,
}
