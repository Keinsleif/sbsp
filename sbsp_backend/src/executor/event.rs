use uuid::Uuid;

use crate::controller::state::StateParam;

#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    Loaded {
        cue_id: Uuid,
        position: f64,
        duration: f64,
    },
    PreWaitStarted {
        cue_id: Uuid,
    },
    PreWaitProgress {
        cue_id: Uuid,
        position: f64,
        duration: f64,
    },
    PreWaitPaused {
        cue_id: Uuid,
        position: f64,
        duration: f64,
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
        initial_params: StateParam,
    },
    Progress {
        cue_id: Uuid,
        position: f64,
        duration: f64,
    },
    Paused {
        cue_id: Uuid,
        position: f64,
        duration: f64,
    },
    Resumed {
        cue_id: Uuid,
    },
    Stopping {
        cue_id: Uuid,
        position: f64,
        duration: f64,
    },
    Stopped {
        cue_id: Uuid,
    },
    Completed {
        cue_id: Uuid,
    },
    StateParamUpdated {
        cue_id: Uuid,
        params: StateParam,
    },
    Error {
        cue_id: Uuid,
        error: String,
    },
}
