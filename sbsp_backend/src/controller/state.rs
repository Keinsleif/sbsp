use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
pub enum PlaybackStatus {
    Loaded,
    PreWaiting,
    PreWaitPaused,
    Playing,
    Paused,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
pub struct ActiveCue {
    pub cue_id: Uuid,
    pub position: f64,
    pub duration: f64,
    pub status: PlaybackStatus,
}

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
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
