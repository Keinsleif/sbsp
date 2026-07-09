// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use uuid::Uuid;

use crate::{action::CueAction, model::settings::ShowSettings};

#[derive(Debug)]
pub enum StopMode {
    Soft,
    Hard,
}

#[derive(Debug)]
pub enum ExecutorCommand {
    Load(Uuid),
    Execute(Uuid),
    Pause(Uuid),
    Resume(Uuid),
    Stop(Uuid, StopMode),
    SeekTo(Uuid, f64),
    SeekBy(Uuid, f64),
    PerformAction(Uuid, CueAction),
    ReconfigureEngines(Box<ShowSettings>),
}
