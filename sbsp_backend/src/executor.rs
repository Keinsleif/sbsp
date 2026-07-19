// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod command;
mod event;

#[cfg(test)]
mod tests;

pub use command::ExecutorCommand;
pub use command::StopMode;
pub use event::ExecutorEvent;

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::model::cue::CueChain;
use crate::{
    action::CueAction,
    controller::state::StateParam,
    engine::{
        EngineEvent, EngineType,
        audio_engine::{AudioCommand, AudioCommandData, AudioEngineEvent},
        wait_engine::{WaitCommand, WaitEvent, WaitType},
    },
    manager::ShowModelHandle,
    model::cue::{Cue, CueParam, audio::AudioCueParam, group::GroupMode},
};

#[derive(Debug)]
struct ActiveInstance {
    engine_type: EngineType,
    is_triggered: bool,  // specify loaded or triggered
    is_prewaiting: bool, // specify prewaiting or playing
    is_paused: bool,     // specify paused or playing
}

enum ChainType {
    Start,
    Complete,
}

enum Task {
    Dispatch(ExecutorCommand),
    SettleStart(Uuid),
    SettleStop {
        cue_id: Uuid,
        is_completed: bool,
    },
    SendCompleted {
        cue_id: Uuid,
    },
    BeginScope {
        cue_id: Uuid,
        context: ScopeContext,
    },
    EndScope {
        cue_id: Uuid,
        context: ScopeContext,
        watermark: usize,
    },
}

#[derive(Debug, Clone, Copy)]
enum ScopeContext {
    GroupLoad { child_count: usize },
    GroupExecute { child_count: usize },
    GroupPause,
    GroupResume,
    GroupStop,
    Playback,
}

pub struct Executor {
    model_handle: ShowModelHandle,
    command_rx: mpsc::Receiver<ExecutorCommand>,
    audio_tx: mpsc::Sender<AudioCommand>,
    wait_tx: mpsc::Sender<WaitCommand>,
    executor_event_tx: mpsc::Sender<ExecutorEvent>,
    engine_event_rx: mpsc::Receiver<EngineEvent>,

    active_instances: HashMap<Uuid, ActiveInstance>,
    task_stack: Vec<Task>,
    error_stack: Vec<String>,
    in_flight: HashSet<Uuid>,
    chain_trigger_history: HashMap<Uuid, VecDeque<Instant>>,
}

impl Executor {
    pub fn new(
        model_handle: ShowModelHandle,
        command_rx: mpsc::Receiver<ExecutorCommand>,
        audio_tx: mpsc::Sender<AudioCommand>,
        wait_tx: mpsc::Sender<WaitCommand>,
        playback_event_tx: mpsc::Sender<ExecutorEvent>,
        engine_event_rx: mpsc::Receiver<EngineEvent>,
    ) -> Self {
        Self {
            model_handle,
            command_rx,
            audio_tx,
            wait_tx,
            executor_event_tx: playback_event_tx,
            engine_event_rx,
            active_instances: HashMap::new(),
            task_stack: Vec::new(),
            error_stack: Vec::new(),
            in_flight: HashSet::new(),
            chain_trigger_history: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        log::info!("Executor run loop started.");
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    log::debug!("Executor received command: {:?}", command);
                    self.task_stack.push(Task::Dispatch(command));
                },
                Some(event) = self.engine_event_rx.recv() => {
                    if let Err(e) = self.handle_engine_event(event).await {
                        log::error!("Error handling engine event: {:?}", e);
                    }
                }
                else => break,
            }
            while let Some(task) = self.task_stack.pop() {
                self.apply(task).await;
            }
            if !self.error_stack.is_empty() {
                log::debug!("Unclaimed cue errors: {}", self.error_stack.join("; "));
                self.error_stack.clear();
            }
        }
        log::info!("Executor run loop finished.");
    }

    async fn apply(&mut self, task: Task) {
        match task {
            Task::Dispatch(executor_command) => {
                if let Err(e) = self.process_command(executor_command).await {
                    log::error!("Failed to execute command: e={}", e);
                    self.error_stack.push(e.to_string());
                }
            }
            Task::SettleStart(cue_id) => {
                if let Some(parent) = self.model_handle.get_parent_by_id(&cue_id).await {
                    let mut need_notify_event = false;
                    self.active_instances
                        .entry(parent.id)
                        .and_modify(|instance| {
                            if instance.is_prewaiting || !instance.is_triggered {
                                need_notify_event = true;
                            }
                            instance.is_prewaiting = false;
                            instance.is_triggered = true;
                        })
                        .or_insert_with(|| {
                            need_notify_event = true;
                            ActiveInstance {
                                engine_type: EngineType::Group,
                                is_prewaiting: false,
                                is_triggered: true,
                                is_paused: false,
                            }
                        });

                    if need_notify_event {
                        self.task_stack.push(Task::SettleStart(parent.id));
                        self.resolve_after_start_chain(parent.id).await;
                        self.executor_event_tx
                            .send(ExecutorEvent::Triggered { cue_id: parent.id })
                            .await
                            .ok();
                        self.executor_event_tx
                            .send(ExecutorEvent::Started {
                                cue_id: parent.id,
                                position: 0.0,
                                duration: 0.0,
                                initial_params: StateParam::None,
                            })
                            .await
                            .ok();
                    }
                }
            }
            Task::SettleStop {
                cue_id,
                is_completed,
            } => {
                let Some(parent) = self.model_handle.get_parent_by_id(&cue_id).await else {
                    return;
                };
                let CueParam::Group { children, .. } = &parent.params else {
                    return;
                };

                if children
                    .iter()
                    .any(|id| self.active_instances.contains_key(id))
                    || !self.active_instances.contains_key(&parent.id)
                {
                    return;
                }

                self.active_instances.remove(&parent.id);

                if is_completed {
                    self.task_stack
                        .push(Task::SendCompleted { cue_id: parent.id });
                    self.resolve_after_complete_chain(parent.id).await;
                } else {
                    self.executor_event_tx
                        .send(ExecutorEvent::Stopped { cue_id: parent.id })
                        .await
                        .ok();
                    self.task_stack.push(Task::SettleStop {
                        cue_id: parent.id,
                        is_completed: false,
                    });
                }
            }
            Task::SendCompleted { cue_id } => {
                if self.active_instances.contains_key(&cue_id) {
                    return;
                }
                self.executor_event_tx
                    .send(ExecutorEvent::Completed { cue_id })
                    .await
                    .ok();
                self.task_stack.push(Task::SettleStop {
                    cue_id,
                    is_completed: true,
                });
            }
            Task::BeginScope { cue_id, context } => {
                self.in_flight.insert(cue_id);
                match context {
                    ScopeContext::GroupLoad { .. } => {}
                    ScopeContext::GroupExecute { .. } => {
                        self.active_instances.insert(
                            cue_id,
                            ActiveInstance {
                                engine_type: EngineType::Group,
                                is_prewaiting: false,
                                is_triggered: true,
                                is_paused: false,
                            },
                        );
                        self.executor_event_tx
                            .send(ExecutorEvent::Started {
                                cue_id,
                                position: 0.0,
                                duration: 0.0,
                                initial_params: StateParam::None,
                            })
                            .await
                            .ok();
                    }
                    ScopeContext::GroupPause => {}
                    ScopeContext::GroupResume => {}
                    ScopeContext::GroupStop => {}
                    ScopeContext::Playback => {
                        self.active_instances.insert(
                            cue_id,
                            ActiveInstance {
                                engine_type: EngineType::Playback,
                                is_prewaiting: false,
                                is_triggered: true,
                                is_paused: false,
                            },
                        );
                        self.executor_event_tx
                            .send(ExecutorEvent::Started {
                                cue_id,
                                position: 0.0,
                                duration: 0.0,
                                initial_params: StateParam::None,
                            })
                            .await
                            .ok();
                    }
                }
            }
            Task::EndScope {
                cue_id,
                context,
                watermark,
            } => {
                self.in_flight.remove(&cue_id);
                let failures = self.error_stack.split_off(watermark);
                match context {
                    ScopeContext::GroupLoad { child_count } => {
                        match (failures.len(), child_count) {
                            (0, _) => {
                                self.executor_event_tx
                                    .send(ExecutorEvent::Loaded {
                                        cue_id,
                                        position: 0.0,
                                        duration: 0.0,
                                    })
                                    .await
                                    .ok();
                            }
                            (n, total) if n == total => {
                                self.active_instances.remove(&cue_id);
                                self.emit_error(
                                    cue_id,
                                    format!("Failed to execute group children. e={:?}", failures),
                                )
                                .await
                                .ok();
                            }
                            _ => {
                                log::error!("Failed to load some group children.");
                                self.executor_event_tx
                                    .send(ExecutorEvent::Loaded {
                                        cue_id,
                                        position: 0.0,
                                        duration: 0.0,
                                    })
                                    .await
                                    .ok();
                            }
                        }
                    }
                    ScopeContext::GroupExecute { child_count } => {
                        match (failures.len(), child_count) {
                            (0, _) => {} // successfully executed.
                            (n, total) if n == total => {
                                self.active_instances.remove(&cue_id);
                                self.emit_error(
                                    cue_id,
                                    format!("Failed to execute group children. e={:?}", failures),
                                )
                                .await
                                .ok();
                            }
                            _ => {
                                log::error!(
                                    "Failed to execute some group children. e={:?}",
                                    failures
                                );
                            } // continue with error log
                        }
                    }
                    ScopeContext::GroupPause => {
                        self.executor_event_tx
                            .send(ExecutorEvent::Paused {
                                cue_id,
                                position: 0.0,
                                duration: 0.0,
                            })
                            .await
                            .ok();
                    }
                    ScopeContext::GroupResume => {
                        self.executor_event_tx
                            .send(ExecutorEvent::Resumed { cue_id })
                            .await
                            .ok();
                    }
                    ScopeContext::GroupStop => {}
                    ScopeContext::Playback => {
                        if let Some(e) = failures.first() {
                            self.active_instances.remove(&cue_id);
                            self.emit_error(
                                cue_id,
                                format!("Failed to dispatch playback control. e={}", e),
                            )
                            .await
                            .ok();
                        } else {
                            self.active_instances.remove(&cue_id);
                            self.emit_completed(cue_id).await.ok();
                        }
                    }
                }
            }
        }
    }

    async fn process_command(&mut self, command: ExecutorCommand) -> Result<(), anyhow::Error> {
        match command {
            ExecutorCommand::Load(cue_id) => {
                if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                    self.load_cue(&cue).await?;
                }
            }
            ExecutorCommand::Execute(cue_id) => {
                if let Some(active_instance) = self.active_instances.get(&cue_id)
                    && active_instance.is_triggered
                {
                    log::warn!("Cue already executed. cue_id={}", cue_id);
                } else if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                    self.task_stack.push(Task::SettleStart(cue_id));
                    self.executor_event_tx
                        .send(ExecutorEvent::Triggered { cue_id })
                        .await?;
                    if cue.pre_wait > 0.0 {
                        if !self.active_instances.contains_key(&cue_id) {
                            self.load_cue(&cue).await?;
                        }
                        if let Some(instance) = self.active_instances.get_mut(&cue_id) {
                            instance.is_prewaiting = true;
                            instance.is_triggered = true;
                        }
                        self.wait_tx
                            .send(WaitCommand::Start {
                                wait_type: WaitType::PreWait,
                                instance_id: cue_id,
                                duration: cue.pre_wait,
                            })
                            .await?;
                    } else {
                        self.execute_cue(&cue).await?;
                    }
                    self.resolve_after_start_chain(cue_id).await;
                } else {
                    anyhow::bail!("EXECUTE: cue not found. cue_id={}", cue_id);
                }
            }
            ExecutorCommand::Pause(cue_id) => self.pause_cue(cue_id).await?,
            ExecutorCommand::Resume(cue_id) => self.resume_cue(cue_id).await?,
            ExecutorCommand::Stop(cue_id, stop_mode) => self.stop_cue(cue_id, stop_mode).await?,
            ExecutorCommand::SeekTo(cue_id, position) => self.seek_to_cue(cue_id, position).await?,
            ExecutorCommand::SeekBy(cue_id, amount) => self.seek_by_cue(cue_id, amount).await?,
            ExecutorCommand::PerformAction(cue_id, action) => {
                if let Some(active_instance) = self.active_instances.get(&cue_id) {
                    match (action, active_instance.engine_type) {
                        (CueAction::Audio(audio_action), EngineType::Audio) => {
                            self.audio_tx
                                .send(AudioCommand::PerformAction {
                                    id: cue_id,
                                    action: audio_action,
                                })
                                .await?;
                        }
                        _ => {
                            log::warn!("Action type isn't match active cue's type. ignoring...");
                        }
                    }
                }
            }
            ExecutorCommand::ReconfigureEngines(settings) => {
                self.audio_tx
                    .send(AudioCommand::Reconfigure(settings.audio))
                    .await?;
            }
        }
        Ok(())
    }

    async fn load_cue(&mut self, cue: &Cue) -> Result<(), anyhow::Error> {
        if self.active_instances.contains_key(&cue.id) {
            anyhow::bail!("Cue already loaded or executed. cue_id={}", cue.id);
        }
        match &cue.params {
            CueParam::Audio(AudioCueParam {
                target,
                start_time,
                fade_in_param,
                end_time,
                fade_out_param,
                volume,
                pan,
                repeat,
                sound_type,
                envelope,
            }) => {
                let filepath = self.model_handle.get_asset_standard_path(target).await?;

                self.audio_tx
                    .send(AudioCommand::Load {
                        id: cue.id,
                        data: AudioCommandData {
                            filepath,
                            volume: *volume,
                            pan: *pan,
                            start_time: *start_time,
                            fade_in_param: *fade_in_param,
                            end_time: *end_time,
                            fade_out_param: *fade_out_param,
                            repeat: *repeat,
                            sound_type: *sound_type,
                            envelope: envelope.clone(),
                        },
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Audio,
                        is_prewaiting: false,
                        is_triggered: false,
                        is_paused: false,
                    },
                );
            }
            CueParam::Wait(params) => {
                self.wait_tx
                    .send(WaitCommand::Load {
                        wait_type: WaitType::Wait,
                        instance_id: cue.id,
                        duration: params.duration,
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Wait,
                        is_prewaiting: false,
                        is_triggered: false,
                        is_paused: false,
                    },
                );
            }
            CueParam::Fade(params) => {
                self.wait_tx
                    .send(WaitCommand::Load {
                        wait_type: WaitType::FadeWait,
                        instance_id: cue.id,
                        duration: params.fade_param.duration,
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Fade,
                        is_prewaiting: false,
                        is_triggered: false,
                        is_paused: false,
                    },
                );
            }
            CueParam::Start(_) | CueParam::Stop(_) | CueParam::Pause(_) | CueParam::Load(_) => {
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Playback,
                        is_prewaiting: false,
                        is_triggered: false,
                        is_paused: false,
                    },
                );
            }
            CueParam::Group { base, children } => {
                if self.in_flight.contains(&cue.id) {
                    log::error!("cyclic group containment; skipping. cue_id={}", cue.id);
                    return Ok(());
                }
                match base.mode {
                    GroupMode::Playlist { .. } | GroupMode::StartFirst { .. } => {
                        if let Some(first_id) = children.first() {
                            self.active_instances.insert(
                                cue.id,
                                ActiveInstance {
                                    engine_type: EngineType::Group,
                                    is_prewaiting: false,
                                    is_triggered: false,
                                    is_paused: false,
                                },
                            );
                            let context = ScopeContext::GroupLoad { child_count: 1 };
                            self.task_stack.push(Task::EndScope {
                                cue_id: cue.id,
                                context,
                                watermark: self.error_stack.len(),
                            });
                            self.task_stack
                                .push(Task::Dispatch(ExecutorCommand::Load(*first_id)));
                            self.task_stack.push(Task::BeginScope {
                                cue_id: cue.id,
                                context,
                            });
                        }
                    }
                    GroupMode::Concurrency => {
                        if !children.is_empty() {
                            self.active_instances.insert(
                                cue.id,
                                ActiveInstance {
                                    engine_type: EngineType::Group,
                                    is_prewaiting: false,
                                    is_triggered: false,
                                    is_paused: false,
                                },
                            );
                            let context = ScopeContext::GroupLoad {
                                child_count: children.len(),
                            };
                            self.task_stack.push(Task::EndScope {
                                cue_id: cue.id,
                                context,
                                watermark: self.error_stack.len(),
                            });
                            for cue_id in children.iter().rev() {
                                self.task_stack
                                    .push(Task::Dispatch(ExecutorCommand::Load(*cue_id)));
                            }
                            self.task_stack.push(Task::BeginScope {
                                cue_id: cue.id,
                                context,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn execute_cue(&mut self, cue: &Cue) -> Result<(), anyhow::Error> {
        match &cue.params {
            CueParam::Audio(AudioCueParam {
                target,
                start_time,
                fade_in_param,
                end_time,
                fade_out_param,
                volume,
                pan,
                repeat,
                sound_type,
                envelope,
            }) => {
                let filepath = self.model_handle.get_asset_standard_path(target).await?;

                let audio_command = AudioCommand::Play {
                    id: cue.id,
                    data: AudioCommandData {
                        filepath,
                        volume: *volume,
                        pan: *pan,
                        start_time: *start_time,
                        fade_in_param: *fade_in_param,
                        end_time: *end_time,
                        fade_out_param: *fade_out_param,
                        repeat: *repeat,
                        sound_type: *sound_type,
                        envelope: envelope.clone(),
                    },
                };
                self.audio_tx.send(audio_command).await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Audio,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
            }
            CueParam::Wait(params) => {
                self.wait_tx
                    .send(WaitCommand::Start {
                        wait_type: WaitType::Wait,
                        instance_id: cue.id,
                        duration: params.duration,
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Wait,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
            }
            CueParam::Fade(params) => {
                if let Some(cue) = self.model_handle.get_cue_by_id(&params.target).await
                    && self.active_instances.contains_key(&params.target)
                {
                    match cue.params {
                        CueParam::Audio(_) => {
                            if self
                                .audio_tx
                                .send(AudioCommand::FadeVolume {
                                    id: params.target,
                                    volume: params.volume,
                                    fade_param: params.fade_param,
                                })
                                .await
                                .is_err()
                            {
                                anyhow::bail!("cannot send AudioCommand");
                            }
                        }
                        CueParam::Group { .. } => {
                            // TODO: check and fade decendants?
                            let children = self
                                .model_handle
                                .get_all_children_by_id(&params.target)
                                .await;
                            for child in children {
                                if self.active_instances.contains_key(&child.id)
                                    && let CueParam::Audio(_) = child.params
                                    && let Err(e) = self
                                        .audio_tx
                                        .send(AudioCommand::FadeVolume {
                                            id: child.id,
                                            volume: params.volume,
                                            fade_param: params.fade_param,
                                        })
                                        .await
                                {
                                    log::error!("Failed to fade group child. e={}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                self.wait_tx
                    .send(WaitCommand::Start {
                        wait_type: WaitType::FadeWait,
                        instance_id: cue.id,
                        duration: params.fade_param.duration,
                    })
                    .await?;
                self.active_instances.insert(
                    cue.id,
                    ActiveInstance {
                        engine_type: EngineType::Fade,
                        is_prewaiting: false,
                        is_triggered: true,
                        is_paused: false,
                    },
                );
            }
            CueParam::Start(params) => {
                if self.in_flight.contains(&cue.id) {
                    log::error!("cyclic playback target; skipping. cue_id={}", cue.id);
                    return Ok(());
                }
                self.task_stack.push(Task::EndScope {
                    cue_id: cue.id,
                    context: ScopeContext::Playback,
                    watermark: self.error_stack.len(),
                });
                if let Some(instance) = self.active_instances.get(&params.target)
                    && instance.is_triggered
                {
                    if instance.is_paused {
                        self.task_stack
                            .push(Task::Dispatch(ExecutorCommand::Resume(params.target)));
                    }
                } else {
                    self.task_stack
                        .push(Task::Dispatch(ExecutorCommand::Execute(params.target)));
                }
                self.task_stack.push(Task::BeginScope {
                    cue_id: cue.id,
                    context: ScopeContext::Playback,
                });
            }
            CueParam::Stop(params) => {
                if self.in_flight.contains(&cue.id) {
                    log::error!("cyclic playback target; skipping. cue_id={}", cue.id);
                    return Ok(());
                }
                self.task_stack.push(Task::EndScope {
                    cue_id: cue.id,
                    context: ScopeContext::Playback,
                    watermark: self.error_stack.len(),
                });
                if self.active_instances.contains_key(&params.target) {
                    let stop_mode = if params.hard {
                        StopMode::Hard
                    } else {
                        StopMode::Soft
                    };
                    self.task_stack.push(Task::Dispatch(ExecutorCommand::Stop(
                        params.target,
                        stop_mode,
                    )));
                }
                self.task_stack.push(Task::BeginScope {
                    cue_id: cue.id,
                    context: ScopeContext::Playback,
                });
            }
            CueParam::Pause(params) => {
                if self.in_flight.contains(&cue.id) {
                    log::error!("cyclic playback target; skipping. cue_id={}", cue.id);
                    return Ok(());
                }
                self.task_stack.push(Task::EndScope {
                    cue_id: cue.id,
                    context: ScopeContext::Playback,
                    watermark: self.error_stack.len(),
                });
                if let Some(instance) = self.active_instances.get(&params.target)
                    && instance.is_triggered
                    && !instance.is_paused
                {
                    self.task_stack
                        .push(Task::Dispatch(ExecutorCommand::Pause(params.target)));
                }
                self.task_stack.push(Task::BeginScope {
                    cue_id: cue.id,
                    context: ScopeContext::Playback,
                });
            }
            CueParam::Load(params) => {
                if self.in_flight.contains(&cue.id) {
                    log::error!("cyclic playback target; skipping. cue_id={}", cue.id);
                    return Ok(());
                }
                self.task_stack.push(Task::EndScope {
                    cue_id: cue.id,
                    context: ScopeContext::Playback,
                    watermark: self.error_stack.len(),
                });
                if !self.active_instances.contains_key(&params.target) {
                    self.task_stack
                        .push(Task::Dispatch(ExecutorCommand::Load(params.target)));
                }
                self.task_stack.push(Task::BeginScope {
                    cue_id: cue.id,
                    context: ScopeContext::Playback,
                });
            }
            CueParam::Group { base, children } => {
                if self.in_flight.contains(&cue.id) {
                    log::error!("cyclic group containment; skipping. cue_id={}", cue.id);
                    return Ok(());
                }
                match base.mode {
                    GroupMode::Playlist { .. } | GroupMode::StartFirst { .. } => {
                        if let Some(first_id) = children.first() {
                            let context = ScopeContext::GroupExecute { child_count: 1 };
                            self.task_stack.push(Task::EndScope {
                                cue_id: cue.id,
                                context,
                                watermark: self.error_stack.len(),
                            });
                            self.task_stack
                                .push(Task::Dispatch(ExecutorCommand::Execute(*first_id)));
                            self.task_stack.push(Task::BeginScope {
                                cue_id: cue.id,
                                context,
                            });
                        }
                    }
                    GroupMode::Concurrency => {
                        if !children.is_empty() {
                            let context = ScopeContext::GroupExecute {
                                child_count: children.len(),
                            };
                            self.task_stack.push(Task::EndScope {
                                cue_id: cue.id,
                                context,
                                watermark: self.error_stack.len(),
                            });
                            for cue_id in children.iter().rev() {
                                self.task_stack
                                    .push(Task::Dispatch(ExecutorCommand::Execute(*cue_id)));
                            }
                            self.task_stack.push(Task::BeginScope {
                                cue_id: cue.id,
                                context,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn pause_cue(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            if active_instance.is_prewaiting {
                self.wait_tx
                    .send(WaitCommand::Pause {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                    })
                    .await?;
                return Ok(());
            }
            match active_instance.engine_type {
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::Pause { id: cue_id })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::Pause {
                            wait_type: WaitType::Wait,
                            instance_id: cue_id,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("Pause command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("Pause command is not available for Transport cues. ignoring...");
                }
                EngineType::Group => {
                    if self.in_flight.contains(&cue_id) {
                        log::error!("cyclic group containment; skipping. cue_id={}", cue_id);
                        return Ok(());
                    }
                    if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await
                        && let CueParam::Group { children, .. } = cue.params
                    {
                        let active_children: Vec<_> = children
                            .iter()
                            .filter(|c| self.active_instances.contains_key(c))
                            .rev()
                            .collect();
                        if !active_children.is_empty() {
                            let context = ScopeContext::GroupPause;
                            self.task_stack.push(Task::EndScope {
                                cue_id: cue.id,
                                context,
                                watermark: self.error_stack.len(),
                            });
                            for child_id in active_children {
                                self.task_stack
                                    .push(Task::Dispatch(ExecutorCommand::Pause(*child_id)));
                            }
                            self.task_stack.push(Task::BeginScope {
                                cue_id: cue.id,
                                context,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn resume_cue(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            if active_instance.is_prewaiting {
                self.wait_tx
                    .send(WaitCommand::Resume {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                    })
                    .await?;
                return Ok(());
            }
            match active_instance.engine_type {
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::Resume { id: cue_id })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::Resume {
                            wait_type: WaitType::Wait,
                            instance_id: cue_id,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("Resume command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("Resume command is not available for Transport cues. ignoring...");
                }
                EngineType::Group => {
                    if self.in_flight.contains(&cue_id) {
                        log::error!("cyclic group containment; skipping. cue_id={}", cue_id);
                        return Ok(());
                    }
                    if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await
                        && let CueParam::Group { children, .. } = cue.params
                    {
                        let active_children: Vec<_> = children
                            .iter()
                            .filter(|c| self.active_instances.contains_key(c))
                            .rev()
                            .collect();
                        if !active_children.is_empty() {
                            let context = ScopeContext::GroupResume;
                            self.task_stack.push(Task::EndScope {
                                cue_id: cue.id,
                                context,
                                watermark: self.error_stack.len(),
                            });
                            for child_id in active_children {
                                self.task_stack
                                    .push(Task::Dispatch(ExecutorCommand::Resume(*child_id)));
                            }
                            self.task_stack.push(Task::BeginScope {
                                cue_id: cue.id,
                                context,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn stop_cue(&mut self, cue_id: Uuid, stop_mode: StopMode) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            let is_prewaiting = active_instance.is_prewaiting;
            if is_prewaiting {
                if let Err(e) = self
                    .wait_tx
                    .send(WaitCommand::Stop {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                    })
                    .await
                {
                    log::error!("Failed to send wait command to stop prewait timer. {}", e);
                }
                match active_instance.engine_type {
                    EngineType::Audio => {
                        self.audio_tx
                            .send(AudioCommand::HardStop { id: cue_id })
                            .await?;
                    }
                    EngineType::Wait => {
                        self.wait_tx
                            .send(WaitCommand::Stop {
                                wait_type: WaitType::Wait,
                                instance_id: cue_id,
                            })
                            .await?;
                    }
                    EngineType::Fade => {
                        self.wait_tx
                            .send(WaitCommand::Stop {
                                wait_type: WaitType::FadeWait,
                                instance_id: cue_id,
                            })
                            .await?;
                    }
                    EngineType::Playback => {}
                    EngineType::Group => {
                        if self.in_flight.contains(&cue_id) {
                            log::error!("cyclic group containment; skipping. cue_id={}", cue_id);
                            return Ok(());
                        }
                        if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await
                            && let CueParam::Group { children, .. } = cue.params
                        {
                            let active_children: Vec<_> = children
                                .iter()
                                .filter(|c| self.active_instances.contains_key(c))
                                .rev()
                                .collect();
                            if !active_children.is_empty() {
                                let context = ScopeContext::GroupStop;
                                self.task_stack.push(Task::EndScope {
                                    cue_id: cue.id,
                                    context,
                                    watermark: self.error_stack.len(),
                                });
                                for child_id in active_children {
                                    self.task_stack.push(Task::Dispatch(ExecutorCommand::Stop(
                                        *child_id,
                                        StopMode::Hard,
                                    )));
                                }
                                self.task_stack.push(Task::BeginScope {
                                    cue_id: cue.id,
                                    context,
                                });
                            }
                        }
                    }
                }
            } else {
                match active_instance.engine_type {
                    EngineType::Audio => {
                        let command = match stop_mode {
                            StopMode::Soft => AudioCommand::SoftStop { id: cue_id },
                            StopMode::Hard => AudioCommand::HardStop { id: cue_id },
                        };
                        self.audio_tx.send(command).await?;
                    }
                    EngineType::Wait => {
                        self.wait_tx
                            .send(WaitCommand::Stop {
                                wait_type: WaitType::Wait,
                                instance_id: cue_id,
                            })
                            .await?;
                    }
                    EngineType::Fade => {
                        log::warn!("Stop command is not available for Fade cue. ignoring...");
                    }
                    EngineType::Playback => {
                        self.active_instances.remove(&cue_id);
                        self.emit_stopped(cue_id).await?;
                    }
                    EngineType::Group => {
                        if self.in_flight.contains(&cue_id) {
                            log::error!("cyclic group containment; skipping. cue_id={}", cue_id);
                            return Ok(());
                        }
                        if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await
                            && let CueParam::Group { children, .. } = cue.params
                        {
                            let active_children: Vec<_> = children
                                .iter()
                                .filter(|c| self.active_instances.contains_key(c))
                                .rev()
                                .collect();
                            if !active_children.is_empty() {
                                let context = ScopeContext::GroupStop;
                                self.task_stack.push(Task::EndScope {
                                    cue_id: cue.id,
                                    context,
                                    watermark: self.error_stack.len(),
                                });
                                for child_id in active_children {
                                    self.task_stack.push(Task::Dispatch(ExecutorCommand::Stop(
                                        *child_id, stop_mode,
                                    )));
                                }
                                self.task_stack.push(Task::BeginScope {
                                    cue_id: cue.id,
                                    context,
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn seek_to_cue(&self, cue_id: Uuid, position: f64) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            if active_instance.is_prewaiting {
                self.wait_tx
                    .send(WaitCommand::SeekTo {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                        position,
                    })
                    .await?;
                return Ok(());
            }
            match active_instance.engine_type {
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::SeekTo {
                            id: cue_id,
                            position,
                        })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::SeekTo {
                            wait_type: WaitType::Wait,
                            instance_id: cue_id,
                            position,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("SeekTo command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("SeekTo command is not available for Transport cues. ignoring...");
                }
                EngineType::Group => {
                    log::warn!("SeekTo command is not available for Group cues. ignoring...");
                }
            }
        }
        Ok(())
    }

    async fn seek_by_cue(&self, cue_id: Uuid, amount: f64) -> Result<(), anyhow::Error> {
        if let Some(active_instance) = self.active_instances.get(&cue_id) {
            if active_instance.is_prewaiting {
                self.wait_tx
                    .send(WaitCommand::SeekBy {
                        wait_type: WaitType::PreWait,
                        instance_id: cue_id,
                        amount,
                    })
                    .await?;
                return Ok(());
            }
            match active_instance.engine_type {
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::SeekBy { id: cue_id, amount })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::SeekBy {
                            wait_type: WaitType::Wait,
                            instance_id: cue_id,
                            amount,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("SeekBy command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("SeekBy command is not available for Transport cues. ignoring...");
                }
                EngineType::Group => {
                    log::warn!("SeekTo command is not available for Group cues. ignoring...");
                }
            }
        }
        Ok(())
    }

    async fn handle_engine_event(&mut self, event: EngineEvent) -> Result<(), anyhow::Error> {
        match event {
            EngineEvent::Audio(audio_event) => {
                let cue_id = audio_event.id();

                let playback_event = match audio_event {
                    AudioEngineEvent::Loaded {
                        position, duration, ..
                    } => ExecutorEvent::Loaded {
                        cue_id,
                        position,
                        duration,
                    },
                    AudioEngineEvent::Started {
                        position,
                        duration,
                        initial_params,
                        ..
                    } => ExecutorEvent::Started {
                        cue_id,
                        position,
                        duration,
                        initial_params: StateParam::Audio(initial_params),
                    },
                    AudioEngineEvent::Progress {
                        position, duration, ..
                    } => {
                        let event = ExecutorEvent::Progress {
                            cue_id,
                            position,
                            duration,
                        };
                        if let Err(e) = self.executor_event_tx.try_send(event) {
                            log::warn!("EngineEvent dropped: {:?}", e);
                        }
                        return Ok(());
                    }
                    AudioEngineEvent::Paused {
                        position, duration, ..
                    } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = true);
                        ExecutorEvent::Paused {
                            cue_id,
                            position,
                            duration,
                        }
                    }
                    AudioEngineEvent::Resumed { .. } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = false);
                        ExecutorEvent::Resumed { cue_id }
                    }
                    AudioEngineEvent::Seeked { position, .. } => {
                        ExecutorEvent::Seeked { cue_id, position }
                    }
                    AudioEngineEvent::Stopping {
                        position, duration, ..
                    } => {
                        let event = ExecutorEvent::Stopping {
                            cue_id,
                            position,
                            duration,
                        };
                        if let Err(e) = self.executor_event_tx.try_send(event) {
                            log::warn!("EngineEvent dropped: {:?}", e);
                        }
                        return Ok(());
                    }
                    AudioEngineEvent::Stopped { .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_stopped(cue_id).await;
                    }
                    AudioEngineEvent::Completed { .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_completed(cue_id).await;
                    }
                    AudioEngineEvent::StateParamUpdated { params, .. } => {
                        ExecutorEvent::StateParamUpdated {
                            cue_id,
                            params: StateParam::Audio(params),
                        }
                    }
                    AudioEngineEvent::Error { error, .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_error(cue_id, error).await;
                    }
                };

                self.executor_event_tx.send(playback_event).await?;
            }
            EngineEvent::PreWait(wait_event) => {
                let cue_id = wait_event.id();

                let executor_event = match wait_event {
                    WaitEvent::Loaded { .. } => unreachable!(),
                    WaitEvent::Started { duration, .. } => {
                        ExecutorEvent::PreWaitStarted { cue_id, duration }
                    }
                    WaitEvent::Progress {
                        position, duration, ..
                    } => {
                        let event = ExecutorEvent::PreWaitProgress {
                            cue_id,
                            position,
                            duration,
                        };
                        if let Err(e) = self.executor_event_tx.try_send(event) {
                            log::warn!("EngineEvent dropped: {:?}", e);
                        }
                        return Ok(());
                    }
                    WaitEvent::Paused {
                        position, duration, ..
                    } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = true);
                        ExecutorEvent::PreWaitPaused {
                            cue_id,
                            position,
                            duration,
                        }
                    }
                    WaitEvent::Resumed { .. } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = false);
                        ExecutorEvent::PreWaitResumed { cue_id }
                    }
                    WaitEvent::Seeked { position, .. } => {
                        ExecutorEvent::Seeked { cue_id, position }
                    }
                    WaitEvent::Stopped { .. } => {
                        if self.active_instances.remove(&cue_id).is_some() {
                            log::info!("PreWaitStopped cue_id={}", cue_id);
                            self.emit_stopped(cue_id).await?;
                        }
                        return Ok(());
                    }
                    WaitEvent::Completed { .. } => {
                        if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                            log::info!("PreWaitCompleted cue_id={}", cue.id);
                            self.executor_event_tx
                                .send(ExecutorEvent::PreWaitCompleted { cue_id })
                                .await?;
                            self.execute_cue(&cue).await?;
                            return Ok(());
                        } else {
                            self.executor_event_tx
                                .send(ExecutorEvent::PreWaitCompleted { cue_id })
                                .await?;
                            self.active_instances.remove(&cue_id);
                            anyhow::bail!("PreWait: cue to execute not found. id={}", cue_id);
                        }
                    }
                };

                self.executor_event_tx.send(executor_event).await?;
            }
            EngineEvent::Wait(wait_event) | EngineEvent::Fade(wait_event) => {
                let cue_id = wait_event.id();

                let playback_event = match wait_event {
                    WaitEvent::Loaded {
                        position, duration, ..
                    } => ExecutorEvent::Loaded {
                        cue_id,
                        position,
                        duration,
                    },
                    WaitEvent::Started {
                        position, duration, ..
                    } => ExecutorEvent::Started {
                        cue_id,
                        position,
                        duration,
                        initial_params: StateParam::None,
                    },
                    WaitEvent::Progress {
                        position, duration, ..
                    } => {
                        let event = ExecutorEvent::Progress {
                            cue_id,
                            position,
                            duration,
                        };
                        if let Err(e) = self.executor_event_tx.try_send(event) {
                            log::warn!("EngineEvent dropped: {:?}", e);
                        }
                        return Ok(());
                    }
                    WaitEvent::Paused {
                        position, duration, ..
                    } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = true);
                        ExecutorEvent::Paused {
                            cue_id,
                            position,
                            duration,
                        }
                    }
                    WaitEvent::Resumed { .. } => {
                        self.active_instances
                            .entry(cue_id)
                            .and_modify(|instance| instance.is_paused = false);
                        ExecutorEvent::Resumed { cue_id }
                    }
                    WaitEvent::Seeked { position, .. } => {
                        ExecutorEvent::Seeked { cue_id, position }
                    }
                    WaitEvent::Stopped { .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_stopped(cue_id).await;
                    }
                    WaitEvent::Completed { .. } => {
                        self.active_instances.remove(&cue_id);
                        return self.emit_completed(cue_id).await;
                    }
                };

                self.executor_event_tx.send(playback_event).await?;
            }
        }
        Ok(())
    }

    async fn emit_stopped(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        self.executor_event_tx
            .send(ExecutorEvent::Stopped { cue_id })
            .await?;
        self.task_stack.push(Task::SettleStop {
            cue_id,
            is_completed: false,
        });
        Ok(())
    }

    async fn emit_error(&mut self, cue_id: Uuid, error: String) -> Result<(), anyhow::Error> {
        self.executor_event_tx
            .send(ExecutorEvent::Error { cue_id, error })
            .await?;
        self.task_stack.push(Task::SettleStop {
            cue_id,
            is_completed: false,
        });
        Ok(())
    }

    async fn emit_completed(&mut self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        self.executor_event_tx
            .send(ExecutorEvent::Completed { cue_id })
            .await?;
        self.task_stack.push(Task::SettleStop {
            cue_id,
            is_completed: true,
        });
        self.resolve_after_complete_chain(cue_id).await; // trigger cue chain before stop parents
        Ok(())
    }

    async fn resolve_after_start_chain(&mut self, cue_id: Uuid) {
        let Some(target) = self.resolve_chain_target(cue_id, ChainType::Start).await else { return };

        if self.record_and_check_chain_trigger(target) {
            self.task_stack
                .push(Task::Dispatch(ExecutorCommand::Execute(target)));
        }
    }

    async fn resolve_after_complete_chain(&mut self, cue_id: Uuid) {
        let Some(target) = self.resolve_chain_target(cue_id, ChainType::Complete).await else { return };

        if self.record_and_check_chain_trigger(target) {
            self.task_stack
                .push(Task::Dispatch(ExecutorCommand::Execute(target)));
        }
    }

    async fn resolve_chain_target(&self, cue_id: Uuid, chain_type: ChainType) -> Option<Uuid> {
        if let Some(chain) = self.model_handle.get_cue_chain_by_id(&cue_id).await {
            match (chain_type, &chain) {
                (ChainType::Start, CueChain::AfterStart { target_id }) => {
                    if let Some(target) = target_id {
                        Some(*target)
                    } else {
                        self.model_handle.get_next_cue_id_by_id(&cue_id).await
                    }
                }
                (ChainType::Complete, CueChain::AfterComplete { target_id }) => {
                    if let Some(target) = target_id {
                        Some(*target)
                    } else {
                        self.model_handle.get_next_cue_id_by_id(&cue_id).await
                    }
                }
                (_, _) => None,
            }
        } else {
            log::warn!("Unknown cue found. model may be broken. cue_id={}", cue_id);
            None
        }
    }

    fn record_and_check_chain_trigger(&mut self, cue_id: Uuid) -> bool {
        const WINDOW: Duration = Duration::from_secs(2);
        const MAX_TRIGGERS_IN_WINDOW: usize = 8;

        let now = Instant::now();
        let history = self.chain_trigger_history.entry(cue_id).or_default();
        while let Some(&oldest) = history.front() {
            if now.duration_since(oldest) > WINDOW {
                history.pop_front();
            } else {
                break;
            }
        }
        history.push_back(now);

        if history.len() > MAX_TRIGGERS_IN_WINDOW {
            log::error!(
                "Cue chain appears to be looping: cue_id={} was auto-chained into {} times within {:?}.",
                cue_id,
                history.len(),
                WINDOW
            );
            false
        } else {
            true
        }
    }
}
