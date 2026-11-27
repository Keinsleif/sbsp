// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

pub mod audio;
pub mod group;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
pub use uuid::Uuid;

use crate::model::{
    cue::{
        audio::{AudioCueParam, Decibels, FadeParam},
        group::GroupCueParamBase,
    },
    settings::CursorAdvanceTrigger,
};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CueList {
    pub cues: HashMap<Uuid, Cue>,
    pub root_ids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: Uuid,
    #[serde(default)]
    pub number: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub color: CueColor,
    #[serde(default)]
    pub pre_wait: f64,
    #[serde(default)]
    pub chain: CueChain,
    #[serde(default)]
    pub treat_stop_as_completed: bool,
    #[serde(default)]
    pub cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    pub params: CueParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CueColor {
    #[default]
    None,
    Red,
    Purple,
    Blue,
    Cyan,
    Green,
    Yellow,
    Orange,
    Grey,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueChain {
    #[default]
    DoNotChain,
    AfterStart {
        target_id: Option<Uuid>,
    },
    AfterComplete {
        target_id: Option<Uuid>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum CueCursorAdvanceTriggerOverride {
    #[default]
    None,
    #[serde(untagged)]
    Override(CursorAdvanceTrigger),
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueParam {
    Audio(AudioCueParam),
    Wait(WaitCueParam),
    Fade(FadeCueParam),
    Start(StartCueParam),
    Stop(StopCueParam),
    Pause(PauseCueParam),
    Load(LoadCueParam),
    Group {
        #[serde(flatten)]
        base: GroupCueParamBase,
        children: Vec<Uuid>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WaitCueParam {
    pub duration: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FadeCueParam {
    pub target: Uuid,
    pub volume: Decibels,
    pub fade_param: FadeParam,
    #[serde(default)]
    pub stop_target_on_complete: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartCueParam {
    pub target: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StopCueParam {
    pub target: Uuid,
    #[serde(default)]
    pub hard: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PauseCueParam {
    pub target: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoadCueParam {
    pub target: Uuid,
}
