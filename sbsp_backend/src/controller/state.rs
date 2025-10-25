use indexmap::map::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
pub enum PlaybackStatus {
    Loaded,
    PreWaiting,
    PreWaitPaused,
    Playing,
    Paused,
    Stopped,
    Completed,
    Error,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, Copy)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StateParam {
    #[default]
    None,
    Audio (AudioStateParam),
    Wait,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Copy)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AudioStateParam {
    pub repeating: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ActiveCue {
    pub cue_id: Uuid,
    pub position: f64,
    pub duration: f64,
    pub status: PlaybackStatus,
    pub params: StateParam,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ShowState {
    pub playback_cursor: Option<Uuid>,
    pub active_cues: IndexMap<Uuid, ActiveCue>,
}

impl ShowState {
    pub fn new() -> Self {
        Self {
            playback_cursor: None,
            active_cues: IndexMap::new(),
        }
    }
}
