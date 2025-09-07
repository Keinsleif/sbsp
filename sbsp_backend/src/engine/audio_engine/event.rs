use uuid::Uuid;

use crate::controller::state::AudioStateParam;

#[derive(Debug)]
pub enum AudioEngineEvent {
    Loaded {
        instance_id: Uuid,
    },
    Started {
        instance_id: Uuid,
        initial_params: AudioStateParam,
    },
    Progress {
        instance_id: Uuid,
        position: f64,
        duration: f64,
    },
    Paused {
        instance_id: Uuid,
        position: f64,
        duration: f64,
    },
    Resumed {
        instance_id: Uuid,
    },
    Stopped {
        instance_id: Uuid,
    },
    Completed {
        instance_id: Uuid,
    },
    StateParamUpdated {
        instance_id: Uuid,
        params: AudioStateParam,
    },
    Error {
        instance_id: Uuid,
        error: String,
    },
}

impl AudioEngineEvent {
    pub fn instance_id(&self) -> Uuid {
        match self {
            Self::Loaded { instance_id } => *instance_id,
            Self::Started { instance_id, .. } => *instance_id,
            Self::Progress { instance_id, .. } => *instance_id,
            Self::Paused { instance_id, .. } => *instance_id,
            Self::Resumed { instance_id } => *instance_id,
            Self::Stopped { instance_id } => *instance_id,
            Self::Completed { instance_id } => *instance_id,
            Self::StateParamUpdated { instance_id, .. } => *instance_id,
            Self::Error { instance_id, .. } => *instance_id,
        }
    }
}
