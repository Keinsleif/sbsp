// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use uuid::Uuid;

use crate::{action::CueAction, model::settings::ShowSettings};

#[derive(Debug, Clone, Copy, PartialEq)]
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
