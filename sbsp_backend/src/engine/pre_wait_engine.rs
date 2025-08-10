use std::{collections::HashMap, time::Duration};

use tokio::{sync::mpsc, time::Instant};
use uuid::Uuid;

use crate::executor::EngineEvent;

#[derive(Debug)]
pub enum PreWaitCommand {
    Start {
        instance_id: Uuid,
        duration: f64,
    },
    Pause {
        instance_id: Uuid,
    },
    Resume {
        instance_id: Uuid,
    },
    Stop {
        instance_id: Uuid,
    },
}

#[derive(Debug)]
pub enum PreWaitEvent {
    Started {
        instance_id: Uuid,
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
    Completed {
        instance_id: Uuid,
    },
}

impl PreWaitEvent {
    pub fn instance_id(&self) -> Uuid {
        match self {
            PreWaitEvent::Started { instance_id } => *instance_id,
            PreWaitEvent::Progress { instance_id, .. } => *instance_id,
            PreWaitEvent::Paused { instance_id, .. } => *instance_id,
            PreWaitEvent::Resumed { instance_id } => *instance_id,
            PreWaitEvent::Completed { instance_id } => *instance_id,
        }
    }
}

pub struct WaitingInstance {
    is_paused: bool,
    total_duration: Duration,
    start_time: Instant,
    remaining_duration: Duration,
}

pub struct PreWaitEngine {
    command_rx: mpsc::Receiver<PreWaitCommand>,
    event_tx: mpsc::Sender<EngineEvent>,
    waiting_instances: HashMap<Uuid, WaitingInstance>,
}

impl PreWaitEngine {
    pub async fn run(mut self) {
        let mut push_timer = tokio::time::interval(Duration::from_millis(50));

        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        PreWaitCommand::Start{instance_id, duration} => {
                            self.waiting_instances.insert(instance_id, WaitingInstance {
                                is_paused: false,
                                total_duration: Duration::from_secs_f64(duration),
                                start_time: Instant::now(),
                                remaining_duration: Duration::from_secs_f64(duration),
                            });
                            self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::PreWaitStarted { instance_id })).await;
                        },
                        PreWaitCommand::Pause{instance_id} => {
                            if let Some(waiting_instance) = self.waiting_instances.get_mut(&instance_id) {
                                if !waiting_instance.is_paused {
                                    let elapsed = waiting_instance.start_time.elapsed();
                                    waiting_instance.is_paused = true;
                                    waiting_instance.remaining_duration -= elapsed;
                                    self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::PreWaitPaused { instance_id, position: (waiting_instance.total_duration - waiting_instance.remaining_duration + elapsed).as_secs_f64(), duration: waiting_instance.total_duration.as_secs_f64() })).await;

                                }
                            }
                        },
                        PreWaitCommand::Resume{instance_id} => {
                            if let Some(waiting_instance) = self.waiting_instances.get_mut(&instance_id) {
                                if waiting_instance.is_paused {
                                    waiting_instance.is_paused = false;
                                    waiting_instance.start_time = Instant::now();
                                    self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::PreWaitResumed { instance_id })).await;
                                }
                            }
                        },
                        PreWaitCommand::Stop{instance_id} => {
                            self.waiting_instances.remove(&instance_id);
                        }
                    }
                },
                _  = push_timer.tick() => {
                    for (instance_id, waiting_instance) in &self.waiting_instances {
                        let elapsed = waiting_instance.start_time.elapsed();
                        if elapsed >= waiting_instance.remaining_duration {
                            self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::PreWaitCompleted { instance_id: *instance_id })).await;
                            break;
                        } else {
                            self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::PreWaitProgress { instance_id: *instance_id, position: (waiting_instance.total_duration - waiting_instance.remaining_duration + elapsed).as_secs_f64(), duration: waiting_instance.total_duration.as_secs_f64() })).await;
                        }
                    }
                }
            }
        }
    }
}