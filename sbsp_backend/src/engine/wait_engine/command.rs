// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use uuid::Uuid;

use super::WaitType;

#[derive(Debug)]
pub enum WaitCommand {
    Load {
        wait_type: WaitType,
        instance_id: Uuid,
        duration: f64,
    },
    Start {
        wait_type: WaitType,
        instance_id: Uuid,
        duration: f64,
    },
    Pause {
        wait_type: WaitType,
        instance_id: Uuid,
    },
    Resume {
        wait_type: WaitType,
        instance_id: Uuid,
    },
    SeekTo {
        wait_type: WaitType,
        instance_id: Uuid,
        position: f64,
    },
    SeekBy {
        wait_type: WaitType,
        instance_id: Uuid,
        amount: f64,
    },
    Stop {
        wait_type: WaitType,
        instance_id: Uuid,
    },
}
