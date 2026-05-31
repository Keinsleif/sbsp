// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use crate::engine::{audio_engine::AudioEngineEvent, wait_engine::WaitEvent};

pub mod audio_engine;
pub mod wait_engine;

#[derive(Debug)]
pub enum EngineEvent {
    Audio(AudioEngineEvent),
    Wait(WaitEvent),
    PreWait(WaitEvent),
    Fade(WaitEvent),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineType {
    PreWait,
    Audio,
    Wait,
    Fade,
    Playback,
    Group,
}
