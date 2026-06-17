// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

pub mod audio;
pub mod group;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
pub use uuid::Uuid;

use crate::model::cue::{
    audio::{AudioCueParam, Decibels, FadeParam},
    group::GroupCueParamBase,
};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: Uuid,
    pub number: String,
    pub name: Option<String>,
    pub notes: String,
    #[serde(default)]
    pub color: CueColor,
    pub pre_wait: f64,
    #[serde(default)]
    pub chain: CueChain,
    pub params: CueParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, TS)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, TS)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueParam {
    Audio(AudioCueParam),
    Wait (WaitCueParam),
    Fade (FadeCueParam),
    Start (StartCueParam),
    Stop (StopCueParam),
    Pause (PauseCueParam),
    Load (LoadCueParam),
    Group {
        #[serde(flatten)]
        base: GroupCueParamBase,
        children: Box<Vec<Cue>>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct WaitCueParam {
    pub duration: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct FadeCueParam {
    pub target: Uuid,
    pub volume: Decibels,
    pub fade_param: FadeParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartCueParam {
    pub target: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct StopCueParam {
    pub target: Uuid,
    #[serde(default)]
    pub hard: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct PauseCueParam {
    pub target: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct LoadCueParam {
    pub target: Uuid,
}