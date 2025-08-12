use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use tokio::{sync::mpsc, time::Instant};
use uuid::Uuid;

use crate::executor::EngineEvent;

#[derive(Debug)]
pub enum PreWaitCommand {
    Start { instance_id: Uuid, duration: f64 },
    Pause { instance_id: Uuid },
    Resume { instance_id: Uuid },
    Stop { instance_id: Uuid },
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
    status: WaitingStatus,
    total_duration: Duration,
    start_time: Instant,
    remaining_duration: Duration,
}

#[derive(Debug, PartialEq)]
pub enum WaitingStatus {
    Waiting,
    Paused,
    Completed,
}

pub struct PreWaitEngine {
    command_rx: mpsc::Receiver<PreWaitCommand>,
    event_tx: mpsc::Sender<EngineEvent>,
    waiting_instances: HashMap<Uuid, WaitingInstance>,
}

impl PreWaitEngine {
    pub fn new(
        pre_wait_command_rx: mpsc::Receiver<PreWaitCommand>,
        pre_wait_event_tx: mpsc::Sender<EngineEvent>,
    ) -> Self {
        Self {
            command_rx: pre_wait_command_rx,
            event_tx: pre_wait_event_tx,
            waiting_instances: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        let mut push_timer = tokio::time::interval(Duration::from_millis(50));

        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    let result: Result<()> = match command {
                        PreWaitCommand::Start{instance_id, duration} => {
                            self.waiting_instances.insert(instance_id, WaitingInstance {
                                status: WaitingStatus::Waiting,
                                total_duration: Duration::from_secs_f64(duration),
                                start_time: Instant::now(),
                                remaining_duration: Duration::from_secs_f64(duration),
                            });
                            if let Err(e) = self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::Started { instance_id })).await {
                                Err(anyhow::anyhow!("Error sending PreWait event: {:?}", e))
                            } else {
                                Ok(())
                            }
                        },
                        PreWaitCommand::Pause{instance_id} => {
                            if let Some(waiting_instance) = self.waiting_instances.get_mut(&instance_id) {
                                if !waiting_instance.status.eq(&WaitingStatus::Paused) {
                                    let elapsed = waiting_instance.start_time.elapsed();
                                    waiting_instance.status = WaitingStatus::Paused;
                                    waiting_instance.remaining_duration -= elapsed;
                                    if let Err(e) = self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::Paused { instance_id, position: (waiting_instance.total_duration - waiting_instance.remaining_duration + elapsed).as_secs_f64(), duration: waiting_instance.total_duration.as_secs_f64() })).await {
                                        Err(anyhow::anyhow!("Error polling PreWait event: {:?}", e))
                                    } else {
                                        Ok(())
                                    }
                                } else {
                                    Err(anyhow::anyhow!("Instance with ID {} has already paused.", instance_id))
                                }
                            } else {
                                Err(anyhow::anyhow!("Instance with ID {} not found for pause.", instance_id))
                            }
                        },
                        PreWaitCommand::Resume{instance_id} => {
                            if let Some(waiting_instance) = self.waiting_instances.get_mut(&instance_id) {
                                if waiting_instance.status.eq(&WaitingStatus::Paused) {
                                    waiting_instance.status = WaitingStatus::Waiting;
                                    waiting_instance.start_time = Instant::now();
                                    if let Err(e) = self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::Resumed { instance_id })).await {
                                        Err(anyhow::anyhow!("Error sending PreWait event: {:?}", e))
                                    } else {
                                        Ok(())
                                    }
                                } else {
                                    Err(anyhow::anyhow!("Instance with ID {} is playing.", instance_id))
                                }
                            } else {
                                Err(anyhow::anyhow!("Instance with ID {} not found for pause.", instance_id))
                            }
                        },
                        PreWaitCommand::Stop{instance_id} => {
                            self.waiting_instances.remove(&instance_id);
                            Ok(())
                        }
                    };

                    if let Err(e) = result {
                        log::error!("{}",e);
                    }
                },
                _  = push_timer.tick() => {
                    for (instance_id, waiting_instance) in &mut self.waiting_instances {
                        let elapsed = waiting_instance.start_time.elapsed();
                        if elapsed >= waiting_instance.remaining_duration {
                            if !waiting_instance.status.eq(&WaitingStatus::Completed) {
                                waiting_instance.status = WaitingStatus::Completed;
                                if let Err(e) = self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::Completed { instance_id: *instance_id })).await {
                                    log::error!("Error sending PreWait event: {:?}", e);
                                }
                            }
                        } else if let Err(e) = self.event_tx.send(EngineEvent::PreWait(PreWaitEvent::Progress { instance_id: *instance_id, position: (waiting_instance.total_duration - waiting_instance.remaining_duration + elapsed).as_secs_f64(), duration: waiting_instance.total_duration.as_secs_f64() })).await {
                           log::error!("Error sending PreWait event: {:?}", e);
                        }
                    }

                    self.waiting_instances.retain(|_, waiting_instance| !waiting_instance.status.eq(&WaitingStatus::Completed));
                }
            }
        }
    }
}
