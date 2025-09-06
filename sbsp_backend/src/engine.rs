use crate::engine::{audio_engine::AudioEngineEvent, wait_engine::WaitEvent};

pub mod audio_engine;
pub mod wait_engine;

#[derive(Debug)]
pub enum EngineEvent {
    Audio(AudioEngineEvent),
    Wait(WaitEvent),
    PreWait(WaitEvent),
}

#[derive(Debug, Clone, Copy)]
pub enum EngineType {
    PreWait,
    Audio,
    Wait,
}
