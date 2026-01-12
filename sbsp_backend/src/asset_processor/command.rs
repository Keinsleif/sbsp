use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "command",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssetProcessorCommand {
    RequestFileAssetData { path: PathBuf },
}