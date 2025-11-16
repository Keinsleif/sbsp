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
}
