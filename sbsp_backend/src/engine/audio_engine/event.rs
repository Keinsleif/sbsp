// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use uuid::Uuid;

use crate::controller::state::AudioStateParam;

#[derive(Debug)]
pub enum AudioEngineEvent {
    Loaded {
        instance_id: Uuid,
        position: f64,
        duration: f64,
    },
    Started {
        instance_id: Uuid,
        position: f64,
        duration: f64,
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
    Seeked {
        instance_id: Uuid,
        position: f64,
    },
    Stopping {
        instance_id: Uuid,
        position: f64,
        duration: f64,
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
    AudioOutputFallback {
        device: bool,
        config: bool,
    },
}
