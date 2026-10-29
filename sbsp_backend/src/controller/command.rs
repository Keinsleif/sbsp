// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::action::CueAction;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
