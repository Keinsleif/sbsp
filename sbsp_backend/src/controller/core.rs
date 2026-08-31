// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use anyhow::Result;
use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;

use crate::{
    controller::state::{ActiveCue, PlaybackStatus, ShowState, StateParam},
    event::BackendEvent,
    executor::{ExecutorCommand, ExecutorEvent, StopMode},
    manager::ShowModelHandle,
};

use super::{ControllerCommand, CueControllerHandle};

pub struct CueController {
    model_handle: ShowModelHandle,
    executor_tx: mpsc::Sender<ExecutorCommand>,
    command_rx: mpsc::Receiver<ControllerCommand>,

    executor_event_rx: mpsc::Receiver<ExecutorEvent>,
    state_tx: watch::Sender<ShowState>,
    event_tx: broadcast::Sender<BackendEvent>,
    event_rx: broadcast::Receiver<BackendEvent>,
}

impl CueController {
    pub fn new(
        model_handle: ShowModelHandle,
        executor_tx: mpsc::Sender<ExecutorCommand>,
        executor_event_rx: mpsc::Receiver<ExecutorEvent>,
        state_tx: watch::Sender<ShowState>,
        event_tx: broadcast::Sender<BackendEvent>,
    ) -> (Self, CueControllerHandle) {
        let event_rx = event_tx.subscribe();
        let (command_tx, command_rx) = mpsc::channel::<ControllerCommand>(32);
        (
            Self {
                model_handle,
                executor_tx,
                command_rx,
                executor_event_rx,
                state_tx,
                event_tx,
                event_rx,
            },
            CueControllerHandle { command_tx },
        )
    }

    pub async fn run(mut self) {
        log::info!("CueController run loop started.");
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    if let Err(e) = self.handle_command(command).await {
                        log::error!("Error handling controller command: {}", e);
                    }
                },
                Some(event) = self.executor_event_rx.recv() => {
                    if let Err(e) = self.handle_executor_event(event).await {
                        log::error!("Error handling playback event: {}", e);
                    }
                },
                result = self.event_rx.recv() => {
                    match result {
                        Ok(event) => {
                            match event {
                                BackendEvent::ShowModelLoaded{..} => {
                                    if let Err(e) = self.hard_stop_all().await {
                                        log::error!("Failed to stop active cues before reset. {}", e);
                                    }
                                },
                                BackendEvent::ShowModelReset{..} => {
                                    if let Err(e) = self.hard_stop_all().await {
                                        log::error!("Failed to stop active cues before reset. {}", e);
                                    }
                                },
                                BackendEvent::CueRemoved{cue_ids} => {
                                    let state = self.state_tx.borrow().clone();
                                    for rm_id in cue_ids {
                                        if state.active_cues.contains_key(&rm_id)
                                            && let Err(e) = self.executor_tx.send(ExecutorCommand::Stop(rm_id, StopMode::Hard)).await {
                                            log::error!("Failed to stop removed cue. {}", e);
                                        }
                                    }
                                }
                                BackendEvent::SettingsUpdated{ new_settings } => {
                                    if let Err(e) = self.executor_tx.send(ExecutorCommand::ReconfigureEngines(new_settings)).await {
                                        log::error!("{}", e);
                                    }
                                }
                                _ => {}
                            }
                        },
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(_) => {
                            log::warn!("Event monitoring receiver Lagged.");
                        },
                    }
                }
                else => break,
            }
        }
        log::info!("CueController run loop finished.");
    }

    async fn handle_command(&self, command: ControllerCommand) -> Result<(), anyhow::Error> {
        let state = self.state_tx.borrow().clone();
        match command {
            ControllerCommand::Execute(cue_id) => {
                self.handle_go(cue_id).await?;
                Ok(())
            }
            ControllerCommand::Load(cue_id) => {
                if self.model_handle.is_cue_exists(&cue_id).await {
                    if !state.active_cues.contains_key(&cue_id) {
                        self.executor_tx.send(ExecutorCommand::Load(cue_id)).await?;
                    } else {
                        anyhow::bail!("Load: cue already executed. cue_id={}", cue_id);
                    }
                } else {
                    anyhow::bail!("Load: cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::SeekTo(cue_id, position) => {
                if self.model_handle.is_cue_exists(&cue_id).await {
                    if state.active_cues.contains_key(&cue_id) {
                        self.executor_tx
                            .send(ExecutorCommand::SeekTo(cue_id, position))
                            .await?;
                    } else {
                        anyhow::bail!("SeekTo: cue is not executed. cue_id={}", cue_id);
                    }
                } else {
                    anyhow::bail!("SeekTo: cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::SeekBy(cue_id, amount) => {
                if self.model_handle.is_cue_exists(&cue_id).await {
                    if state.active_cues.contains_key(&cue_id) {
                        self.executor_tx
                            .send(ExecutorCommand::SeekBy(cue_id, amount))
                            .await?;
                    } else {
                        anyhow::bail!("SeekBy: cue is not executed. cue_id={}", cue_id);
                    }
                } else {
                    anyhow::bail!("SeekBy: cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::Pause(cue_id) => {
                if self.model_handle.is_cue_exists(&cue_id).await {
                    if let Some(active_cue) = state.active_cues.get(&cue_id)
                        && (active_cue.status == PlaybackStatus::PreWaiting
                            || active_cue.status == PlaybackStatus::Playing)
                    {
                        self.executor_tx
                            .send(ExecutorCommand::Pause(cue_id))
                            .await?;
                    } else {
                        anyhow::bail!("Pause: cue is not playing. cue_id={}", cue_id);
                    }
                } else {
                    anyhow::bail!("Pause: cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::Resume(cue_id) => {
                if self.model_handle.is_cue_exists(&cue_id).await {
                    if let Some(active_cue) = state.active_cues.get(&cue_id)
                        && (active_cue.status == PlaybackStatus::PreWaitPaused
                            || active_cue.status == PlaybackStatus::Paused)
                    {
                        self.executor_tx
                            .send(ExecutorCommand::Resume(cue_id))
                            .await?;
                    } else {
                        anyhow::bail!("Resume: cue is not paused. cue_id={}", cue_id);
                    }
                } else {
                    anyhow::bail!("Resume: cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::Stop(cue_id) => {
                if self.model_handle.is_cue_exists(&cue_id).await {
                    if let Some(active_cue) = state.active_cues.get(&cue_id) {
                        let stop_mode = if active_cue.status == PlaybackStatus::Stopping {
                            StopMode::Hard
                        } else {
                            StopMode::Soft
                        };
                        self.executor_tx
                            .send(ExecutorCommand::Stop(cue_id, stop_mode))
                            .await?;
                    }
                } else {
                    anyhow::bail!("Stop: cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::PerformAction(cue_id, cue_action) => {
                if self.model_handle.is_cue_exists(&cue_id).await {
                    if state.active_cues.contains_key(&cue_id) {
                        self.executor_tx
                            .send(ExecutorCommand::PerformAction(cue_id, cue_action))
                            .await?;
                    } else {
                        anyhow::bail!("PerformAction: cue is not executed. cue_id={}", cue_id);
                    }
                } else {
                    anyhow::bail!("PerformAction: cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::PauseAll
            | ControllerCommand::ResumeAll
            | ControllerCommand::StopAll => {
                let root_ids = self.model_handle.read().await.cue_list.root_ids.clone();
                for cue_id in &root_ids {
                    if let Some(active_cue) = state.active_cues.get(cue_id) {
                        let executor_command = match command {
                            ControllerCommand::PauseAll => match active_cue.status {
                                PlaybackStatus::PreWaiting | PlaybackStatus::Playing => {
                                    ExecutorCommand::Pause(*cue_id)
                                }
                                _ => continue,
                            },
                            ControllerCommand::ResumeAll => match active_cue.status {
                                PlaybackStatus::PreWaitPaused | PlaybackStatus::Paused => {
                                    ExecutorCommand::Resume(*cue_id)
                                }
                                _ => continue,
                            },
                            ControllerCommand::StopAll => {
                                if active_cue.status == PlaybackStatus::Stopping {
                                    ExecutorCommand::Stop(*cue_id, StopMode::Hard)
                                } else {
                                    ExecutorCommand::Stop(*cue_id, StopMode::Soft)
                                }
                            }
                            _ => unreachable!(),
                        };
                        self.executor_tx.send(executor_command).await?;
                    }
                }
                Ok(())
            }
        }
    }

    async fn handle_go(&self, cue_id: Uuid) -> Result<()> {
        let state = self.state_tx.borrow().clone();

        if self.model_handle.is_cue_exists(&cue_id).await {
            if let Some(active_cue) = state.active_cues.get(&cue_id)
                && active_cue.status != PlaybackStatus::Loaded
            {
                log::warn!("GO: cue already executed.");
            } else {
                self.executor_tx
                    .send(ExecutorCommand::Execute(cue_id))
                    .await?;
            };
        } else {
            anyhow::bail!("invalid cue id. cue_id={}", cue_id);
        }
        Ok(())
    }

    async fn hard_stop_all(&self) -> Result<()> {
        let state = self.state_tx.borrow().clone();
        for cue_id in state.active_cues.keys() {
            self.executor_tx
                .send(ExecutorCommand::Stop(*cue_id, StopMode::Hard))
                .await?;
        }
        Ok(())
    }

    async fn handle_executor_event(&mut self, event: ExecutorEvent) -> Result<(), anyhow::Error> {
        let mut show_state = self.state_tx.borrow().clone();
        let mut state_changed = false;
        let mut send_event = true;

        match &event {
            ExecutorEvent::Triggered { .. } => {}
            ExecutorEvent::Loaded {
                cue_id,
                position,
                duration,
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.position = *position;
                    active_cue.duration = *duration;
                    active_cue.status = PlaybackStatus::Loaded;
                } else {
                    let active_cue = ActiveCue {
                        cue_id: *cue_id,
                        position: *position,
                        duration: *duration,
                        status: PlaybackStatus::Loaded,
                        params: StateParam::None,
                    };
                    show_state.active_cues.insert(*cue_id, active_cue);
                }
                state_changed = true;
            }
            ExecutorEvent::Started {
                cue_id,
                position,
                duration,
                initial_params,
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.position = *position;
                    active_cue.duration = *duration;
                    active_cue.params = *initial_params;
                    active_cue.status = PlaybackStatus::Playing;
                } else {
                    let active_cue = ActiveCue {
                        cue_id: *cue_id,
                        position: *position,
                        duration: *duration,
                        status: PlaybackStatus::Playing,
                        params: *initial_params,
                    };
                    show_state.active_cues.insert(*cue_id, active_cue);
                }
                state_changed = true;
            }
            ExecutorEvent::Progress {
                cue_id,
                position,
                duration,
                ..
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    if (position - active_cue.position).abs() > 0.1 {
                        active_cue.position = (position * 10.0).floor() / 10.0;
                        state_changed = true;
                    }
                    if active_cue.duration != *duration {
                        active_cue.duration = *duration;
                        state_changed = true;
                    }
                    if active_cue.status != PlaybackStatus::Playing {
                        active_cue.status = PlaybackStatus::Playing;
                        state_changed = true;
                    }
                }
                send_event = false; // skip sending Progress event
            }
            ExecutorEvent::Paused {
                cue_id,
                position,
                duration,
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    if active_cue.position != *position {
                        active_cue.position = *position;
                        state_changed = true;
                    }
                    if active_cue.duration != *duration {
                        active_cue.duration = *duration;
                        state_changed = true;
                    }
                    if !active_cue.status.eq(&PlaybackStatus::Paused) {
                        active_cue.status = PlaybackStatus::Paused;
                        state_changed = true;
                    }
                } else {
                    send_event = false;
                }
            }
            ExecutorEvent::Resumed { cue_id } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id)
                    && !active_cue.status.eq(&PlaybackStatus::Playing)
                {
                    active_cue.status = PlaybackStatus::Playing;
                    state_changed = true;
                } else {
                    send_event = false;
                }
            }
            ExecutorEvent::Seeked { cue_id, position } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.position = *position;
                    state_changed = true;
                } else {
                    send_event = false;
                }
            }
            ExecutorEvent::Stopping {
                cue_id,
                position,
                duration,
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    if (position - active_cue.position).abs() > 0.1 {
                        active_cue.position = (position * 10.0).floor() / 10.0;
                        state_changed = true;
                    }
                    if active_cue.duration != *duration {
                        active_cue.duration = *duration;
                        state_changed = true;
                    }
                    if active_cue.status != PlaybackStatus::Stopping {
                        active_cue.status = PlaybackStatus::Stopping;
                        state_changed = true;
                    } else {
                        send_event = false; // only send first Stopping event
                    }
                } else {
                    send_event = false;
                }
            }
            ExecutorEvent::Stopped { cue_id }
            | ExecutorEvent::Completed { cue_id, .. }
            | ExecutorEvent::Error { cue_id, .. } => {
                show_state.active_cues.shift_remove(cue_id);
                state_changed = true;
            }
            ExecutorEvent::StateParamUpdated { cue_id, params } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.params = *params;
                    state_changed = true;
                }
            }
            ExecutorEvent::PreWaitStarted { cue_id, duration } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.position = 0.0;
                    active_cue.duration = *duration;
                    active_cue.status = PlaybackStatus::PreWaiting;
                } else {
                    let active_cue = ActiveCue {
                        cue_id: *cue_id,
                        position: 0.0,
                        duration: *duration,
                        status: PlaybackStatus::PreWaiting,
                        params: StateParam::None,
                    };
                    show_state.active_cues.insert(*cue_id, active_cue);
                }
                state_changed = true;
            }
            ExecutorEvent::PreWaitProgress {
                cue_id,
                position,
                duration,
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    if (position - active_cue.position).abs() > 0.1 {
                        active_cue.position = (position * 10.0).floor() / 10.0;
                        state_changed = true;
                    }
                    if active_cue.duration != *duration {
                        active_cue.duration = *duration;
                        state_changed = true;
                    }
                    if active_cue.status != PlaybackStatus::PreWaiting {
                        active_cue.status = PlaybackStatus::PreWaiting;
                        state_changed = true;
                    }
                }
                send_event = false; // skip sending PreWaitProgress event
            }
            ExecutorEvent::PreWaitPaused {
                cue_id,
                position,
                duration,
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    if active_cue.position != *position {
                        active_cue.position = *position;
                        state_changed = true;
                    }
                    if active_cue.duration != *duration {
                        active_cue.duration = *duration;
                        state_changed = true;
                    }
                    if !active_cue.status.eq(&PlaybackStatus::PreWaitPaused) {
                        active_cue.status = PlaybackStatus::PreWaitPaused;
                        state_changed = true;
                    }
                } else {
                    send_event = false;
                }
            }
            ExecutorEvent::PreWaitResumed { cue_id } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id)
                    && !active_cue.status.eq(&PlaybackStatus::PreWaiting)
                {
                    active_cue.status = PlaybackStatus::PreWaiting;
                    state_changed = true;
                } else {
                    send_event = false;
                }
            }
            ExecutorEvent::PreWaitCompleted { .. } => {} // skip to keep active cue because cue will be started. but event is emitted for client.
            ExecutorEvent::AudioOutputFallback { .. } => {}
        }

        if state_changed && self.state_tx.send(show_state).is_err() {
            log::trace!("No UI clients are listening to state updates.");
        }

        if send_event
            && let Ok(ui_event) = BackendEvent::try_from(event)
            && self.event_tx.send(ui_event).is_err()
        {
            log::trace!("No UI clients are listening to playback events.");
        }
        Ok(())
    }
}
