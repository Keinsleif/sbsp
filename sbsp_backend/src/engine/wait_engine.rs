// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod command;
mod event;

pub use command::WaitCommand;
pub use event::WaitEvent;

use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use tokio::{
    sync::mpsc,
    time::{Instant, MissedTickBehavior},
};
use uuid::Uuid;

use super::EngineEvent;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum WaitType {
    PreWait,
    Wait,
    FadeWait,
}

pub struct WaitingInstance {
    status: WaitingStatus,
    total_duration: Duration,
    start_time: Instant,
    remaining_duration: Duration,
}

pub struct LoadedInstance {
    total_duration: Duration,
    remaining_duration: Duration,
}

impl LoadedInstance {
    fn start_waiting(&self) -> WaitingInstance {
        WaitingInstance {
            status: WaitingStatus::Waiting,
            total_duration: self.total_duration,
            start_time: Instant::now(),
            remaining_duration: self.remaining_duration,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum WaitingStatus {
    Waiting,
    Paused,
    Completed,
}

pub struct WaitEngine {
    command_rx: mpsc::Receiver<WaitCommand>,
    event_tx: mpsc::Sender<EngineEvent>,
    waiting_instances: HashMap<(WaitType, Uuid), WaitingInstance>,
    loaded_instances: HashMap<(WaitType, Uuid), LoadedInstance>,
}

impl WaitEngine {
    pub fn new(
        wait_command_rx: mpsc::Receiver<WaitCommand>,
        wait_event_tx: mpsc::Sender<EngineEvent>,
    ) -> Self {
        Self {
            command_rx: wait_command_rx,
            event_tx: wait_event_tx,
            waiting_instances: HashMap::new(),
            loaded_instances: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        let mut push_timer = tokio::time::interval(Duration::from_millis(50));
        push_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    let result: Result<()> = match command {
                        WaitCommand::Load { wait_type, instance_id, duration } => {
                            if wait_type.eq(&WaitType::Wait) {
                                self.loaded_instances.insert((wait_type, instance_id), LoadedInstance { total_duration: Duration::from_secs_f64(duration), remaining_duration: Duration::from_secs_f64(duration) });
                                if let Err(e) = self.event_tx.send(EngineEvent::Wait(WaitEvent::Loaded { instance_id, position: 0.0 , duration })).await {
                                    Err(anyhow::anyhow!("Error sending PreWait event: {:?}", e))
                                } else {
                                    Ok(())
                                }
                            } else {
                                Ok(())
                            }
                        }
                        WaitCommand::Start{wait_type, instance_id, duration} => {
                            if let Some(loaded_instance) = self.loaded_instances.remove(&(wait_type, instance_id)) {
                                self.waiting_instances.insert((wait_type, instance_id), loaded_instance.start_waiting());
                            } else {
                                self.waiting_instances.insert((wait_type, instance_id), WaitingInstance {
                                    status: WaitingStatus::Waiting,
                                    total_duration: Duration::from_secs_f64(duration),
                                    start_time: Instant::now(),
                                    remaining_duration: Duration::from_secs_f64(duration),
                                });
                            }
                            let wait_event = WaitEvent::Started { instance_id, position: 0.0, duration };
                            let event = Self::wrap_wait_event(wait_type, wait_event);
                            if let Err(e) = self.event_tx.send(event).await {
                                Err(anyhow::anyhow!("Error sending PreWait event: {:?}", e))
                            } else {
                                Ok(())
                            }
                        },
                        WaitCommand::Pause{wait_type, instance_id} => {
                            if let Some(waiting_instance) = self.waiting_instances.get_mut(&(wait_type, instance_id)) {
                                if !waiting_instance.status.eq(&WaitingStatus::Paused) {
                                    let elapsed = waiting_instance.start_time.elapsed();
                                    waiting_instance.status = WaitingStatus::Paused;
                                    waiting_instance.remaining_duration -= elapsed;
                                    let wait_event = WaitEvent::Paused { instance_id, position: (waiting_instance.total_duration - waiting_instance.remaining_duration).as_secs_f64(), duration: waiting_instance.total_duration.as_secs_f64() };
                                    let event = Self::wrap_wait_event(wait_type, wait_event);
                                    if let Err(e) = self.event_tx.send(event).await {
                                        Err(anyhow::anyhow!("Error polling Wait event: {:?}", e))
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
                        WaitCommand::Resume{wait_type, instance_id} => {
                            if let Some(waiting_instance) = self.waiting_instances.get_mut(&(wait_type, instance_id)) {
                                if waiting_instance.status.eq(&WaitingStatus::Paused) {
                                    waiting_instance.status = WaitingStatus::Waiting;
                                    waiting_instance.start_time = Instant::now();
                                    let wait_event = WaitEvent::Resumed { instance_id };
                                    let event = Self::wrap_wait_event(wait_type, wait_event);
                                    if let Err(e) = self.event_tx.send(event).await {
                                        Err(anyhow::anyhow!("Error sending Wait event: {:?}", e))
                                    } else {
                                        Ok(())
                                    }
                                } else {
                                    Err(anyhow::anyhow!("Instance with ID {} is playing.", instance_id))
                                }
                            } else {
                                Err(anyhow::anyhow!("Instance with ID {} not found for resume.", instance_id))
                            }
                        },
                        WaitCommand::SeekTo {wait_type, instance_id, position} => {
                            if let Some(waiting_instance) = self.waiting_instances.get_mut(&(wait_type, instance_id)) {
                                let new_duration = (waiting_instance.total_duration.as_secs_f64() - position).clamp(0.0, waiting_instance.total_duration.as_secs_f64());
                                waiting_instance.remaining_duration = Duration::from_secs_f64(new_duration);
                                waiting_instance.start_time = Instant::now();
                                let wait_event = WaitEvent::Seeked { instance_id, position };
                                let event = Self::wrap_wait_event(wait_type, wait_event);
                                if let Err(e) = self.event_tx.send(event).await {
                                    Err(anyhow::anyhow!("Error sending Wait event: {:?}", e))
                                } else {
                                    Ok(())
                                }
                            } else {
                                Err(anyhow::anyhow!("Instance with ID {} not found for seek to.", instance_id))
                            }
                        },
                        WaitCommand::SeekBy {wait_type, instance_id, amount} => {
                            if let Some(waiting_instance) = self.waiting_instances.get_mut(&(wait_type, instance_id)) {
                                let elapsed = if waiting_instance.status == WaitingStatus::Waiting {
                                    waiting_instance.start_time.elapsed()
                                } else {
                                    Duration::ZERO
                                };
                                let current_remaining = waiting_instance.remaining_duration.saturating_sub(elapsed);
                                let new_remaining = (current_remaining.as_secs_f64() - amount).clamp(0.0, waiting_instance.total_duration.as_secs_f64());
                                waiting_instance.remaining_duration = Duration::from_secs_f64(new_remaining);
                                waiting_instance.start_time = Instant::now();
                                let position = (waiting_instance.total_duration - waiting_instance.remaining_duration).as_secs_f64();
                                let wait_event = WaitEvent::Seeked { instance_id, position };
                                let event = Self::wrap_wait_event(wait_type, wait_event);
                                if let Err(e) = self.event_tx.send(event).await {
                                    Err(anyhow::anyhow!("Error sending Wait event: {:?}", e))
                                } else {
                                    Ok(())
                                }
                            } else {
                                Err(anyhow::anyhow!("Instance with ID {} not found for seek by.", instance_id))
                            }
                        },
                        WaitCommand::Stop{wait_type, instance_id} => {
                            let wait_event = WaitEvent::Stopped { instance_id };
                            let event = Self::wrap_wait_event(wait_type, wait_event);
                            if self.waiting_instances.remove(&(wait_type, instance_id)).is_some() || self.loaded_instances.remove(&(wait_type, instance_id)).is_some() {
                                if let Err(e) = self.event_tx.send(event).await {
                                    Err(anyhow::anyhow!("Error sending Wait event: {:?}", e))
                                } else {
                                    Ok(())
                                }
                            } else {
                                Err(anyhow::anyhow!("Instance with ID {} not found for stop.", instance_id))
                            }
                        }
                    };

                    if let Err(e) = result {
                        log::error!("{}",e);
                    }
                },
                _  = push_timer.tick() => {
                    for (instance_key, waiting_instance) in &mut self.waiting_instances {
                        let elapsed = waiting_instance.start_time.elapsed();
                        let wait_event;
                        if elapsed >= waiting_instance.remaining_duration {

                            if waiting_instance.status.eq(&WaitingStatus::Waiting) {
                                waiting_instance.status = WaitingStatus::Completed;
                                wait_event = WaitEvent::Completed { instance_id: instance_key.1 }
                            } else {
                                continue;
                            }
                        } else {
                            if waiting_instance.status.ne(&WaitingStatus::Waiting) {
                                continue;
                            }
                            wait_event = WaitEvent::Progress { instance_id: instance_key.1, position: (waiting_instance.total_duration - waiting_instance.remaining_duration + elapsed).as_secs_f64(), duration: waiting_instance.total_duration.as_secs_f64() };
                            let event = Self::wrap_wait_event(instance_key.0, wait_event);
                            if let Err(e) = self.event_tx.try_send(event) {
                                log::warn!("EngineEvent dropped: {:?}", e);
                            }
                            continue;
                        }
                        let event = Self::wrap_wait_event(instance_key.0, wait_event);
                        if let Err(e) = self.event_tx.send(event).await {
                            log::error!("Error sending Wait event: {:?}", e);
                        }
                    }
                    self.waiting_instances.retain(|_, waiting_instance| !waiting_instance.status.eq(&WaitingStatus::Completed));
                }
            }
        }
    }

    fn wrap_wait_event(wait_type: WaitType, wait_event: WaitEvent) -> EngineEvent {
        match wait_type {
            WaitType::PreWait => EngineEvent::PreWait(wait_event),
            WaitType::Wait => EngineEvent::Wait(wait_event),
            WaitType::FadeWait => EngineEvent::Fade(wait_event),
        }
    }
}
