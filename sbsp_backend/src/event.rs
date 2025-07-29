use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{executor::ExecutorEvent, model::cue::Cue};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "param", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UiEvent {
    // Cue Status Events
    CueStarted {
        cue_id: Uuid,
    },
    CuePaused {
        cue_id: Uuid,
    },
    CueResumed {
        cue_id: Uuid,
    },
    CueCompleted {
        cue_id: Uuid,
    },
    CueError {
        cue_id: Uuid,
        error: String,
    },

    // System Events
    PlaybackCursorMoved {
        cue_id: Uuid,
    },

    ShowModelLoaded {
        path: PathBuf
    },
    ShowModelSaved {
        path: PathBuf,
    },
    CueUpdated {
        cue: Cue,
    },
    CueAdded {
        cue: Cue,
        at_index: usize,
    },
    CueRemoved {
        cue_id: Uuid,
    },
    CueMoved {
        cue_id: Uuid,
        to_index: usize,
    },

    OperationFailed {
        error: UiError,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all="camelCase", rename_all_fields = "camelCase")]
pub enum UiError {
    FileSave {
        path: PathBuf,
        message: String,
    },
    FileLoad {
        path: PathBuf,
        message: String,
    },
    CueEdit {
        cue_id: Uuid,
        message: String,
    },
}

impl From<ExecutorEvent> for UiEvent {
    fn from(value: ExecutorEvent) -> Self {
        match value {
            ExecutorEvent::Started { cue_id } => UiEvent::CueStarted { cue_id },
            ExecutorEvent::Paused { cue_id, .. } => UiEvent::CuePaused { cue_id },
            ExecutorEvent::Resumed { cue_id } => UiEvent::CueResumed { cue_id },
            ExecutorEvent::Completed { cue_id } => UiEvent::CueCompleted { cue_id },
            ExecutorEvent::Progress { .. } => unreachable!(),
            ExecutorEvent::Error { cue_id, error } => UiEvent::CueError { cue_id, error },
        }
    }
}