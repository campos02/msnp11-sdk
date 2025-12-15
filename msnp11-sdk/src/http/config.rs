use crate::Tab;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, uniffi::Record)]
pub struct Config {
    pub tabs: Vec<Tab>,
    pub msn_today_url: String,
}
