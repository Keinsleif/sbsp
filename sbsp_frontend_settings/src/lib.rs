// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

pub mod hotkey;
#[cfg(feature = "manager")]
pub mod manager;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use hotkey::HotkeySettings;
use sbsp_backend::{
    BackendAudioSettings, BackendSettings,
    model::cue::{
        Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, FadeCueParam,
        LoadCueParam, PauseCueParam, StartCueParam, StopCueParam, Uuid, WaitCueParam,
        audio::{AudioCueParam, Decibels, Easing, FadeParam, SoundType},
        group::{GroupCueParamBase, GroupMode},
    },
};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct GlobalHostSettings {
    pub general: GeneralSettings,
    #[serde(default)]
    pub audio: AudioHardwareSettings,
    pub appearance: AppearanceSettings,
    pub hotkey: HotkeySettings,
    pub template: TemplateSettings,
    pub name_format: NameFormatSettings,
}

impl From<&GlobalHostSettings> for BackendSettings {
    fn from(from: &GlobalHostSettings) -> BackendSettings {
        BackendSettings {
            copy_assets_when_add: from.general.copy_assets_when_add,
            audio: BackendAudioSettings {
                device_id: from.audio.device_id.clone(),
                channel_count: from.audio.channel_count,
                sample_rate: from.audio.sample_rate,
                buffer_size: from.audio.buffer_size,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct GlobalRemoteSettings {
    pub general: GeneralSettings,
    pub appearance: AppearanceSettings,
    pub hotkey: HotkeySettings,
    pub template: TemplateSettings,
    pub name_format: NameFormatSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    #[serde(default)]
    pub cursor_advance_trigger: CursorAdvanceTrigger,
    pub lock_cursor_to_selection: bool,
    pub copy_assets_when_add: bool,
    pub seek_amount: f64,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            cursor_advance_trigger: CursorAdvanceTrigger::default(),
            lock_cursor_to_selection: true,
            copy_assets_when_add: false,
            seek_amount: 5.0,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct AudioHardwareSettings {
    pub device_id: Option<String>,
    pub channel_count: Option<u16>,
    pub sample_rate: Option<u32>,
    pub buffer_size: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", default)]
pub struct AppearanceSettings {
    pub language: Option<String>,
    pub dark_mode: DarkMode,
    pub hide_controls: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum DarkMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
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
                color: CueColor::None,
                pre_wait: 0.0,
                chain: CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: CueParam::Audio(AudioCueParam {
                    target: PathBuf::new(),
                    start_time: None,
                    fade_in_param: None,
                    end_time: None,
                    fade_out_param: None,
                    volume: Decibels::IDENTITY,
                    pan: 0.0,
                    repeat: false,
                    sound_type: SoundType::Streaming,
                    envelope: Vec::new(),
                }),
            },
            wait: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                color: CueColor::None,
                pre_wait: 0.0,
                chain: CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: CueParam::Wait(WaitCueParam { duration: 5.0 }),
            },
            fade: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                color: CueColor::None,
                pre_wait: 0.0,
                chain: CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: CueParam::Fade(FadeCueParam {
                    target: Uuid::nil(),
                    volume: Decibels::IDENTITY,
                    fade_param: FadeParam {
                        duration: 3.0,
                        easing: Easing::InOutPow(2.0),
                    },
                }),
            },
            start: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                color: CueColor::None,
                pre_wait: 0.0,
                chain: CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: CueParam::Start(StartCueParam {
                    target: Uuid::nil(),
                }),
            },
            stop: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                color: CueColor::None,
                pre_wait: 0.0,
                chain: CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: CueParam::Stop(StopCueParam {
                    target: Uuid::nil(),
                    hard: false,
                }),
            },
            pause: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                color: CueColor::None,
                pre_wait: 0.0,
                chain: CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: CueParam::Pause(PauseCueParam {
                    target: Uuid::nil(),
                }),
            },
            load: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                color: CueColor::None,
                pre_wait: 0.0,
                chain: CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: CueParam::Load(LoadCueParam {
                    target: Uuid::nil(),
                }),
            },
            group: Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: None,
                notes: "".to_string(),
                color: CueColor::None,
                pre_wait: 0.0,
                chain: CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: CueParam::Group {
                    base: GroupCueParamBase {
                        mode: GroupMode::Playlist { repeat: true },
                    },
                    children: Vec::new(),
                },
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
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
