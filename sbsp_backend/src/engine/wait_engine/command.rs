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
        instance_id: Uuid,
    },
    Resume {
        instance_id: Uuid,
    },
    SeekTo {
        instance_id: Uuid,
        position: f64,
    },
    SeekBy {
        instance_id: Uuid,
        amount: f64,
    },
    Stop {
        instance_id: Uuid,
    },
}
