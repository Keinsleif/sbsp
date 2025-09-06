use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::cue::{
    Cue, CueParam, CueSequence,
    audio::{AudioCueParam, SoundType},
};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct ShowSettings {
    pub general: GeneralSettings,
    pub hotkey: HotkeySettings,
    pub template: TemplateSettings,
    pub audio: AudioSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    pub advance_cursor_when_go: bool,
    pub lock_cursor_to_selection: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            advance_cursor_when_go: true,
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
    pub pause_and_resume: Option<String>,
    pub pause_all: Option<String>,
    pub resume_all: Option<String>,
    pub stop: Option<String>,
    pub stop_all: Option<String>,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            go: Some("Enter".to_string()),
            load: Some("L".to_string()),
            pause_and_resume: Some("Space".to_string()),
            pause_all: Some("[".to_string()),
            resume_all: Some("]".to_string()),
            stop: Some("Backspace".to_string()),
            stop_all: Some("Escape".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateSettings {
    pub audio: Cue,
    pub wait: Cue,
}

impl Default for TemplateSettings {
    fn default() -> Self {
        Self {
            audio: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Audio(AudioCueParam {
                    target: PathBuf::new(),
                    start_time: None,
                    fade_in_param: None,
                    end_time: None,
                    fade_out_param: None,
                    volume: 0.0,
                    pan: 0.0,
                    repeat: false,
                    sound_type: SoundType::Streaming,
                }),
            },
            wait: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Wait { duration: 5.0 },
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct AudioSettings {
    pub mono_output: bool,
}
