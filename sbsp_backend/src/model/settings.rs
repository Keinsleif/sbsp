use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowSettings {
    pub general: ShowGeneralSettings,
    pub audio: ShowAudioSettings,
    pub remote: ShowRemoteSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowGeneralSettings {
    pub copy_assets_destination: String,
}

impl Default for ShowGeneralSettings {
    fn default() -> Self {
        Self {
            copy_assets_destination: ".".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowAudioSettings {
    pub mono_output: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowRemoteSettings {
    pub lock_cursor_to_selection: bool,
}
