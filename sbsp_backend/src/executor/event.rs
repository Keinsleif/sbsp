use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    Loaded {
        cue_id: Uuid,
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
    Stopped {
        cue_id: Uuid,
    },
    Completed {
        cue_id: Uuid,
    },
    Error {
        cue_id: Uuid,
        error: String,
    },
}
