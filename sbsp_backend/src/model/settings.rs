use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::cue::{AudioCueLevels, Cue, CueParam, CueSequence};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowSettings {
    pub general: GeneralSettings,
    pub hotkey: HotkeySettings,
    pub template: TemplateSettings,
    // TODO Audio, Network, MIDI, OSC, Video settings, Remote
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    pub lock_cursor_to_selection: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            lock_cursor_to_selection: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct HotkeySettings {
    pub go: Option<String>,
    pub load: Option<String>,
    pub stop: Option<String>,
    pub stop_all: Option<String>,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            go: Some("Space".to_string()),
            load: Some("L".to_string()),
            stop: Some("Backspace".to_string()),
            stop_all: Some("Esc".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateSettings {
    pub audio: Option<Cue>,
    pub wait: Option<Cue>,
}

impl Default for TemplateSettings {
    fn default() -> Self {
        Self {
            audio: Some(Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: "".to_string(),
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Audio {
                    target: PathBuf::new(),
                    start_time: None,
                    fade_in_param: None,
                    end_time: None,
                    fade_out_param: None,
                    levels: AudioCueLevels::default(),
                    loop_region: None,
                },
            }),
            wait: Some(Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: "".to_string(),
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Wait { duration: 5.0 },
            }),
        }
    }
}
