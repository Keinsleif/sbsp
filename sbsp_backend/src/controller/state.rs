use std::collections::HashMap;

use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct ActiveCue {
    pub cue_id: Uuid,
    pub position: f64,
    pub duration: f64,
    pub status: PlaybackStatus,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShowState {
    pub playback_cursor: Option<Uuid>,
    pub active_cues: HashMap<Uuid, ActiveCue>,
}

impl ShowState {
    pub fn new() -> Self {
        Self {
            playback_cursor: None,
            active_cues: HashMap::new(),
        }
    }
}
