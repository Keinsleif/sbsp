use std::path::PathBuf;

use uuid::Uuid;

use crate::{
    action::AudioAction,
    model::{
        cue::audio::{AudioFadeParam, SoundType},
        settings::AudioSettings,
    },
};

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Load { id: Uuid, data: AudioCommandData },
    Play { id: Uuid, data: AudioCommandData },
    Pause { id: Uuid },
    Resume { id: Uuid },
    Stop { id: Uuid },
    SeekTo { id: Uuid, position: f64 },
    SeekBy { id: Uuid, amount: f64 },
    PerformAction { id: Uuid, action: AudioAction },
    Reconfigure(AudioSettings),
}

impl AudioCommand {
    pub(super) fn id(&self) -> Uuid {
        match self {
            AudioCommand::Load { id, .. } => *id,
            AudioCommand::Play { id, .. } => *id,
            AudioCommand::Pause { id } => *id,
            AudioCommand::Resume { id } => *id,
            AudioCommand::Stop { id } => *id,
            AudioCommand::SeekTo { id, .. } => *id,
            AudioCommand::SeekBy { id, .. } => *id,
            AudioCommand::PerformAction { id, .. } => *id,
            _ => Uuid::nil(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioCommandData {
    pub sound_type: SoundType,
    pub filepath: PathBuf,
    pub volume: f32,
    pub pan: f32,
    pub start_time: Option<f64>,
    pub fade_in_param: Option<AudioFadeParam>,
    pub end_time: Option<f64>,
    pub fade_out_param: Option<AudioFadeParam>,
    pub repeat: bool,
}
