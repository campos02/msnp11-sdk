use crate::Tab;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Config {
    pub tabs: Vec<Tab>,
    pub msn_today_url: String,
}
