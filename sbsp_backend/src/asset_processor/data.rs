use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AssetData {
    pub metadata: AssetMetadata,
    pub waveform: Vec<f32>,
    pub integrated_lufs: Option<f64>,
    pub peak: f32,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AssetMetadata {
    pub path: PathBuf,
    pub duration: Option<f64>,
    pub channel_count: Option<u16>,
    pub sample_rate: u32,
}
