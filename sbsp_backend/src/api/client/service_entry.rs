use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ServiceEntry {
    pub fullname: String,
    pub server_name: String,
    pub host: String,
    pub port: u16,
}
