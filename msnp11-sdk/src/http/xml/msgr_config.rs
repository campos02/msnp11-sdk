use crate::http::xml::tab::Tab;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct MsgrConfig {
    #[serde(rename = "TabConfig")]
    pub tab_config: TabConfig,
}

#[derive(Deserialize)]
pub(crate) struct TabConfig {
    pub msntabdata: Msntabdata,
}

#[derive(Deserialize)]
pub(crate) struct Msntabdata {
    pub tab: Vec<Tab>,
}
