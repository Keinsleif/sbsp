// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::action::CueAction;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "command",
    content = "params",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ControllerCommand {
    Go,
    Load(Uuid),
    Pause(Uuid),
    Resume(Uuid),
    Stop(Uuid),
    SeekTo(Uuid, f64),
    SeekBy(Uuid, f64),
    PauseAll,
    ResumeAll,
    StopAll,
    PerformAction(Uuid, CueAction),
    SetPlaybackCursor { cue_id: Option<Uuid> },
}
