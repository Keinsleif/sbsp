use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AssetData {
    pub path: PathBuf,
    pub duration: Option<f64>,
    pub waveform: Vec<f32>,
}
