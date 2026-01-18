use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
pub enum LicenseEdition {
    Free,
    Pro,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
pub struct LicenseInformation {
    pub owner: String,
    pub edition: LicenseEdition,
    pub id: Uuid,
    pub issue_time: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
pub struct LicenseFile {
    pub payload: LicenseInformation,
    pub signature: String,
}
