// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowSettings {
    pub general: ShowGeneralSettings,
    pub audio: ShowAudioSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowGeneralSettings {
    #[serde(default)]
    pub cursor_advance_trigger: CursorAdvanceTrigger,
    pub copy_assets_destination: String,
}

impl Default for ShowGeneralSettings {
    fn default() -> Self {
        Self {
            cursor_advance_trigger: CursorAdvanceTrigger::default(),
            copy_assets_destination: ".".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CursorAdvanceTrigger {
    OnTriggered,
    OnCompleted,
    #[default]
    Manual,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowAudioSettings {
    pub mono_output: bool,
    pub lufs_target: f64,
}

impl Default for ShowAudioSettings {
    fn default() -> Self {
        Self {
            mono_output: false,
            lufs_target: -14.0,
        }
    }
}
