use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(not(feature = "type_export"))]
use crate::executor::ExecutorEvent;
use crate::{
    asset_processor::AssetData, controller::state::StateParam, model::{ProjectType, ShowModel, cue::Cue, settings::ShowSettings}
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
    CueStatus(CueStatusEventParam),

    // System Events
    PlaybackCursorMoved {
        cue_id: Option<Uuid>,
    },
    SyncState(SyncData),

    // Model Events
    ShowModelLoaded {
        model: ShowModel,
        project_type: ProjectType,
        path: PathBuf,
    },
    ShowModelSaved {
        project_type: ProjectType,
        path: PathBuf,
    },
    ShowModelReset {
        model: ShowModel,
    },
    CueRemoved {
        cue_id: Uuid,
    },
    CueListUpdated {
        cues: Vec<Cue>,
    },
    SettingsUpdated {
        new_settings: Box<ShowSettings>,
    },
    ModelNameUpdated {
        new_name: String,
    },

    // AssetProcessor Events
    AssetResult {
        path: PathBuf,
        data: Result<AssetData, String>,
    },

    OperationFailed {
        error: UiError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueStatusEventParam {
    Loaded {
        cue_id: Uuid,
        position: f64,
        duration: f64,
    },
    PreWaitStarted {
        cue_id: Uuid,
        duration: f64,
    },
    PreWaitPaused {
        cue_id: Uuid,
        position: f64,
    },
    PreWaitResumed {
        cue_id: Uuid,
    },
    PreWaitStopped {
        cue_id: Uuid,
    },
    PreWaitCompleted {
        cue_id: Uuid,
    },
    Started {
        cue_id: Uuid,
        duration: f64,
        params: StateParam,
    },
    Paused {
        cue_id: Uuid,
        position: f64,
    },
    Resumed {
        cue_id: Uuid,
    },
    Stopping {
        cue_id: Uuid,
    },
    Stopped {
        cue_id: Uuid,
    },
    Seeked {
        cue_id: Uuid,
        position: f64,
    },
    Completed {
        cue_id: Uuid,
    },
    Error {
        cue_id: Uuid,
        error: String,
    },
    StateParamUpdated {
        cue_id: Uuid,
        params: StateParam,
    },
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

#[cfg(not(feature = "type_export"))]
impl TryFrom<ExecutorEvent> for UiEvent {
    type Error = ();

    fn try_from(value: ExecutorEvent) -> Result<Self, Self::Error> {
        use crate::executor::ExecutorEvent;

        let status_param = match value {
            ExecutorEvent::Loaded { cue_id, position, duration } => Some(CueStatusEventParam::Loaded { cue_id, position, duration }),
            ExecutorEvent::Started { cue_id, duration, initial_params } => Some(CueStatusEventParam::Started { cue_id, duration, params: initial_params }),
            ExecutorEvent::Paused { cue_id, position, .. } => Some(CueStatusEventParam::Paused { cue_id, position }),
            ExecutorEvent::Resumed { cue_id } => Some(CueStatusEventParam::Resumed { cue_id }),
            ExecutorEvent::Stopped { cue_id } => Some(CueStatusEventParam::Stopped { cue_id }),
            ExecutorEvent::Completed { cue_id } => Some(CueStatusEventParam::Completed { cue_id }),
            ExecutorEvent::Progress { .. } => None,
            ExecutorEvent::Stopping { cue_id, .. } => Some(CueStatusEventParam::Stopping { cue_id }),
            ExecutorEvent::StateParamUpdated { cue_id, params } => Some(CueStatusEventParam::StateParamUpdated { cue_id, params }),
            ExecutorEvent::Error { cue_id, error } => Some(CueStatusEventParam::Error { cue_id, error }),
            ExecutorEvent::PreWaitStarted { cue_id, duration } => Some(CueStatusEventParam::PreWaitStarted { cue_id, duration }),
            ExecutorEvent::PreWaitProgress { .. } => None,
            ExecutorEvent::PreWaitPaused { cue_id, position, .. } => Some(CueStatusEventParam::PreWaitPaused { cue_id, position }),
            ExecutorEvent::PreWaitResumed { cue_id } => Some(CueStatusEventParam::PreWaitResumed { cue_id }),
            ExecutorEvent::PreWaitStopped { cue_id } => Some(CueStatusEventParam::PreWaitStopped { cue_id }),
            ExecutorEvent::PreWaitCompleted { cue_id } => Some(CueStatusEventParam::PreWaitCompleted { cue_id }),
        };
        if let Some(param) = status_param {
            Ok(UiEvent::CueStatus(param))
        } else {
            Err(())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SyncData {
    pub latency: f64,
    pub cues: Vec<CueState>
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CueState {
    pub id: Uuid,
    pub position: f64,
}