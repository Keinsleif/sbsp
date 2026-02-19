use uuid::Uuid;

#[derive(Debug)]
pub enum WaitEvent {
    Loaded {
        instance_id: Uuid,
        position: f64,
        duration: f64,
    },
    Started {
        instance_id: Uuid,
        duration: f64,
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
}

impl WaitEvent {
    pub fn id(&self) -> Uuid {
        match self {
            WaitEvent::Loaded { instance_id, .. } => *instance_id,
            WaitEvent::Started { instance_id, .. } => *instance_id,
            WaitEvent::Progress { instance_id, .. } => *instance_id,
            WaitEvent::Paused { instance_id, .. } => *instance_id,
            WaitEvent::Resumed { instance_id } => *instance_id,
            WaitEvent::Stopped { instance_id } => *instance_id,
            WaitEvent::Completed { instance_id } => *instance_id,
        }
    }
}
