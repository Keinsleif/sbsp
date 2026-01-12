use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueAction {
    Audio(AudioAction),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "action",
    content = "params",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AudioAction {
    ToggleRepeat,
    SetVolume(f32),
}
