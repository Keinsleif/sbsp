// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use uuid::Uuid;

use crate::controller::state::StateParam;

#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    Loaded {
        cue_id: Uuid,
        position: f64,
        duration: f64,
    },
    Triggered {
        cue_id: Uuid,
    },
    PreWaitStarted {
        cue_id: Uuid,
        duration: f64,
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
    PreWaitCompleted {
        cue_id: Uuid,
    },
    Started {
        cue_id: Uuid,
        position: f64,
        duration: f64,
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
    Seeked {
        cue_id: Uuid,
        position: f64,
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
