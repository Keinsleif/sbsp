// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    action::{AudioAction, CueAction},
    model::cue::audio::Decibels,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(
    tag = "command",
    content = "params",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ControllerCommand {
    Execute(Uuid),
    Load(Uuid),
    Pause(Uuid),
    Resume(Uuid),
    Stop(Uuid),
    SoftStop(Uuid),
    HardStop(Uuid),
    SeekTo(Uuid, f64),
    SeekBy(Uuid, f64),
    PauseAll,
    ResumeAll,
    StopAll,
    SoftStopAll,
    HardStopAll,
    PerformAction(Uuid, CueAction),
}

impl ControllerCommand {
    pub fn toggle_repeat(cue_id: Uuid) -> Self {
        Self::PerformAction(cue_id, CueAction::Audio(AudioAction::ToggleRepeat))
    }
    pub fn set_volume(cue_id: Uuid, volume: Decibels) -> Self {
        Self::PerformAction(cue_id, CueAction::Audio(AudioAction::SetVolume(volume)))
    }
}
