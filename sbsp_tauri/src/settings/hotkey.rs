use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    playback: PlaybackHotkey,
    audio_action: AudioActionHotkey,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaybackHotkey {
    pub go: Option<String>,
    pub load: Option<String>,
    pub pause_and_resume: Option<String>,
    pub pause_all: Option<String>,
    pub resume_all: Option<String>,
    pub stop: Option<String>,
    pub stop_all: Option<String>,
    pub seek_forward: Option<String>,
    pub seek_backward: Option<String>,
}

impl Default for PlaybackHotkey {
    fn default() -> Self {
        Self {
            go: Some("Enter".to_string()),
            load: Some("L".to_string()),
            pause_and_resume: Some("Space".to_string()),
            pause_all: Some("[".to_string()),
            resume_all: Some("]".to_string()),
            stop: Some("Backspace".to_string()),
            stop_all: Some("Escape".to_string()),
            seek_forward: None,
            seek_backward: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioActionHotkey {
    pub toggle_repeat: Option<String>,
}

impl Default for AudioActionHotkey {
    fn default() -> Self {
        Self {
            toggle_repeat: Some("R".to_string()),
        }
    }
}
