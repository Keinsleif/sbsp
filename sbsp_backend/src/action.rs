// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use serde::{Deserialize, Serialize};

use crate::model::cue::audio::Decibels;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueAction {
    Audio(AudioAction),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(
    tag = "action",
    content = "params",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AudioAction {
    ToggleRepeat,
    SetVolume(Decibels),
}
