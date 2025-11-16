pub mod hotkey;
pub mod manager;

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use sbsp_backend::{BackendSettings , model::cue::{Cue, CueParam, CueSequence, audio::{AudioCueParam, Easing, FadeParam, SoundType}}};
use hotkey::HotkeySettings;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct GlobalSettings {
    pub general: GeneralSettings,
    pub hotkey: HotkeySettings,
    pub template: TemplateSettings,
}

impl From<&GlobalSettings> for BackendSettings {
    fn from(from: &GlobalSettings) -> BackendSettings {
        BackendSettings {
            advance_cursor_when_go: from.general.advance_cursor_when_go,
            copy_assets_when_add: from.general.copy_assets_when_add,
        }
    }
}

impl PartialEq<BackendSettings> for GlobalSettings {
    fn eq(&self, other: &BackendSettings) -> bool {
        if self.general.advance_cursor_when_go == other.advance_cursor_when_go && self.general.copy_assets_when_add == other.copy_assets_when_add {
            return true;
        }
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    pub advance_cursor_when_go: bool,
    pub lock_cursor_to_selection: bool,
    pub copy_assets_when_add: bool,
    pub seek_amount: f64,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            advance_cursor_when_go: false,
            lock_cursor_to_selection: true,
            copy_assets_when_add: false,
            seek_amount: 5.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateSettings {
    pub audio: Cue,
    pub wait: Cue,
    pub fade: Cue,
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
            fade: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Fade { target: Uuid::nil(), volume: 0.0, fade_param: FadeParam { duration: 3.0, easing: Easing::InOutPowi(2) } }
            }
        }
    }
}
