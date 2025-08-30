use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    executor::ExecutorEvent,
    model::{cue::Cue, settings::ShowSettings},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    content = "param",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UiEvent {
    // Cue Status Events
    CueLoaded { cue_id: Uuid },
    CuePreWaitStarted { cue_id: Uuid },
    CuePreWaitPaused { cue_id: Uuid },
    CuePreWaitResumed { cue_id: Uuid },
    CuePreWaitStopped { cue_id: Uuid },
    CuePreWaitCompleted { cue_id: Uuid },
    CueStarted { cue_id: Uuid },
    CuePaused { cue_id: Uuid },
    CueResumed { cue_id: Uuid },
    CueStopped { cue_id: Uuid },
    CueCompleted { cue_id: Uuid },
    CueError { cue_id: Uuid, error: String },

    // System Events
    PlaybackCursorMoved { cue_id: Option<Uuid> },

    // Model Events
    ShowModelLoaded { path: PathBuf },
    ShowModelSaved { path: PathBuf },
    CueUpdated { cue: Cue },
    CueAdded { cue: Cue, at_index: usize },
    CuesAdded { cues: Vec<Cue>, at_index: usize },
    CueRemoved { cue_id: Uuid },
    CueMoved { cue_id: Uuid, to_index: usize },
    SettingsUpdated { new_settings: Box<ShowSettings> },

    OperationFailed { error: UiError },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UiError {
    FileSave { path: PathBuf, message: String },
    FileLoad { path: PathBuf, message: String },
    CueEdit { message: String },
}

impl From<ExecutorEvent> for UiEvent {
    fn from(value: ExecutorEvent) -> Self {
        match value {
            ExecutorEvent::Loaded { cue_id } => UiEvent::CueLoaded { cue_id },
            ExecutorEvent::Started { cue_id } => UiEvent::CueStarted { cue_id },
            ExecutorEvent::Paused { cue_id, .. } => UiEvent::CuePaused { cue_id },
            ExecutorEvent::Resumed { cue_id } => UiEvent::CueResumed { cue_id },
            ExecutorEvent::Stopped { cue_id } => UiEvent::CueStopped { cue_id },
            ExecutorEvent::Completed { cue_id } => UiEvent::CueCompleted { cue_id },
            ExecutorEvent::Progress { .. } => unreachable!(),
            ExecutorEvent::Error { cue_id, error } => UiEvent::CueError { cue_id, error },
            ExecutorEvent::PreWaitStarted { cue_id } => UiEvent::CuePreWaitStarted { cue_id },
            ExecutorEvent::PreWaitProgress { .. } => unreachable!(),
            ExecutorEvent::PreWaitPaused { cue_id, .. } => UiEvent::CuePreWaitPaused { cue_id },
            ExecutorEvent::PreWaitResumed { cue_id } => UiEvent::CuePreWaitResumed { cue_id },
            ExecutorEvent::PreWaitStopped { cue_id } => UiEvent::CuePreWaitStopped { cue_id },
            ExecutorEvent::PreWaitCompleted { cue_id } => UiEvent::CuePreWaitCompleted { cue_id },
        }
    }
}
