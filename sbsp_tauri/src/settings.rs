pub mod hotkey;
pub mod manager;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;
use uuid::Uuid;

use hotkey::HotkeySettings;
use sbsp_backend::{
    BackendSettings,
    model::cue::{
        Cue, CueParam, CueSequence,
        audio::{AudioCueParam, Easing, FadeParam, SoundType}, group::GroupMode,
    },
};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct GlobalSettings {
    pub general: GeneralSettings,
    pub appearance: AppearanceSettings,
    pub hotkey: HotkeySettings,
    pub template: TemplateSettings,
    pub name_format: NameFormatSettings,
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
        if self.general.advance_cursor_when_go == other.advance_cursor_when_go
            && self.general.copy_assets_when_add == other.copy_assets_when_add
        {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct AppearanceSettings {
    pub language: Option<String>,
    pub dark_mode: DarkMode,
    pub hide_controls: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, TS)]
#[serde(rename_all = "camelCase")]
pub enum DarkMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateSettings {
    pub audio: Cue,
    pub wait: Cue,
    pub fade: Cue,
    pub start: Cue,
    pub stop: Cue,
    pub pause: Cue,
    pub load: Cue,
    pub group: Cue,
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
                params: CueParam::Fade {
                    target: Uuid::nil(),
                    volume: 0.0,
                    fade_param: FadeParam {
                        duration: 3.0,
                        easing: Easing::InOutPowi(2),
                    },
                },
            },
            start: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Start {
                    target: Uuid::nil(),
                },
            },
            stop: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Stop {
                    target: Uuid::nil(),
                },
            },
            pause: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Pause {
                    target: Uuid::nil(),
                },
            },
            load: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Load {
                    target: Uuid::nil(),
                },
            },
            group: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Group {
                    mode: GroupMode::Playlist { repeat: true },
                    children: Box::new(Vec::new()),
                },
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct NameFormatSettings {
    pub audio: String,
    pub wait: String,
    pub fade: String,
    pub start: String,
    pub stop: String,
    pub pause: String,
    pub load: String,
    pub group: String,
}

impl Default for NameFormatSettings {
    fn default() -> Self {
        Self {
            audio: "{filename}".into(),
            wait: "Wait {duration}".into(),
            fade: "Fade {targetName}".into(),
            start: "Start {targetName}".into(),
            stop: "Stop {targetName}".into(),
            pause: "Pause {targetName}".into(),
            load: "Load {targetName}".into(),
            group: "Group".into(),
        }
    }
}
