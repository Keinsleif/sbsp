// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod command;
mod handle;
pub mod state;

pub use command::ControllerCommand;
pub use handle::CueControllerHandle;

use std::collections::HashMap;

use anyhow::Result;
use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;

use crate::{
    BackendSettings, controller::state::{ActiveCue, PlaybackStatus, ShowState, StateParam}, event::BackendEvent, executor::{ExecutorCommand, ExecutorEvent, StopMode}, manager::ShowModelHandle, model::cue::{CueChain, CueParam, group::GroupMode},
};

pub struct CueController {
    model_handle: ShowModelHandle,
    settings_rx: watch::Receiver<BackendSettings>,
    executor_tx: mpsc::Sender<ExecutorCommand>,
    command_rx: mpsc::Receiver<ControllerCommand>,

    executor_event_rx: mpsc::Receiver<ExecutorEvent>,
    state_tx: watch::Sender<ShowState>,
    event_tx: broadcast::Sender<BackendEvent>,
    event_rx: broadcast::Receiver<BackendEvent>,

    advance_cursor_when_go: bool,
    wait_tasks: HashMap<Uuid, CueChain>,
}

impl CueController {
    pub fn new(
        model_handle: ShowModelHandle,
        settings_rx: watch::Receiver<BackendSettings>,
        executor_tx: mpsc::Sender<ExecutorCommand>,
        executor_event_rx: mpsc::Receiver<ExecutorEvent>,
        state_tx: watch::Sender<ShowState>,
        event_tx: broadcast::Sender<BackendEvent>,
    ) -> (Self, CueControllerHandle) {
        let event_rx = event_tx.subscribe();
        let advance_cursor_when_go = settings_rx.borrow().advance_cursor_when_go;
        let (command_tx, command_rx) = mpsc::channel::<ControllerCommand>(32);
        (
            Self {
                model_handle,
                settings_rx,
                executor_tx,
                command_rx,
                executor_event_rx,
                state_tx,
                event_tx,
                event_rx,
                advance_cursor_when_go,
                wait_tasks: HashMap::new(),
            },
            CueControllerHandle { command_tx },
        )
    }

    pub async fn run(mut self) {
        log::info!("CueController run loop started.");
        loop {
            tokio::select! {
                Ok(_) = self.settings_rx.changed() => {
                    self.advance_cursor_when_go = self.settings_rx.borrow().advance_cursor_when_go;
                }
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
                                    {
                                        let model = self.model_handle.read().await;
                                        if let Some(first_id) = model.cue_list.root_ids.first() {
                                            self.state_tx.send_modify(|state| {
                                                state.playback_cursor = Some(*first_id);
                                            });
                                        } else {
                                            self.state_tx.send_modify(|state| {
                                                state.playback_cursor = None;
                                            });
                                        }
                                    }
                                    if let Err(e) = self.hard_stop_all().await {
                                        log::error!("Failed to stop active cues before reset. {}", e);
                                    }
                                },
                                BackendEvent::ShowModelReset{..} => {
                                    self.state_tx.send_modify(|state| {
                                        state.playback_cursor = None;
                                    });
                                    if let Err(e) = self.hard_stop_all().await {
                                        log::error!("Failed to stop active cues before reset. {}", e);
                                    }
                                },
                                BackendEvent::CueRemoved{cue_ids} => {
                                    let state = self.state_tx.borrow().clone();
                                    if let Some(cursor) = state.playback_cursor && cue_ids.contains(&cursor) {
                                        let model = self.model_handle.read().await;
                                        if let Some(first_id) = model.cue_list.root_ids.first() {
                                            self.state_tx.send_modify(|state| {
                                                state.playback_cursor = Some(*first_id);
                                            });
                                        } else {
                                            self.state_tx.send_modify(|state| {
                                                state.playback_cursor = None;
                                            });
                                        }
                                    }
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
            ControllerCommand::Go => {
                let cue_id = if let Some(cursor) = state.playback_cursor {
                    cursor
                } else {
                    anyhow::bail!("GO: playback_cursor is unavailable.");
                };
                self.handle_go(cue_id).await?;

                if self.advance_cursor_when_go {
                    self.update_playback_cursor().await?;
                }
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
                        self.executor_tx.send(ExecutorCommand::SeekTo(cue_id, position)).await?;
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
                        self.executor_tx.send(ExecutorCommand::SeekBy(cue_id, amount)).await?;
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
                    if let Some(active_cue) = state.active_cues.get(&cue_id) && (active_cue.status == PlaybackStatus::PreWaiting || active_cue.status == PlaybackStatus::Playing) {
                        self.executor_tx.send(ExecutorCommand::Pause(cue_id)).await?;
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
                    if let Some(active_cue) = state.active_cues.get(&cue_id) && (active_cue.status == PlaybackStatus::PreWaitPaused || active_cue.status == PlaybackStatus::Paused) {
                        self.executor_tx.send(ExecutorCommand::Resume(cue_id)).await?;
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
                        self.executor_tx.send(ExecutorCommand::Stop(cue_id, stop_mode)).await?;
                    }
                } else {
                    anyhow::bail!("Stop: cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::PerformAction(cue_id, cue_action) => {
                if self.model_handle.is_cue_exists(&cue_id).await {
                    if state.active_cues.contains_key(&cue_id) {
                        self.executor_tx.send(ExecutorCommand::PerformAction(cue_id, cue_action)).await?;
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
                for (cue_id, active_cue) in &state.active_cues {
                    let is_group = self
                        .model_handle
                        .get_cue_by_id(cue_id)
                        .await
                        .is_some_and(|cue| matches!(cue.params, CueParam::Group { .. }));
                    if !is_group {
                        let executor_command = match command {
                            ControllerCommand::PauseAll => {
                                match active_cue.status {
                                    PlaybackStatus::PreWaiting |
                                    PlaybackStatus::Playing => {
                                        ExecutorCommand::Pause(*cue_id)
                                    }
                                    _ => continue,
                                }
                            },
                            ControllerCommand::ResumeAll => {
                                match active_cue.status {
                                    PlaybackStatus::PreWaitPaused |
                                    PlaybackStatus::Paused => {
                                        ExecutorCommand::Resume(*cue_id)
                                    }
                                    _ => continue,
                                }
                            },
                            ControllerCommand::StopAll => {
                                if active_cue.status == PlaybackStatus::Stopping {
                                    ExecutorCommand::Stop(*cue_id, StopMode::Hard)
                                } else {
                                    ExecutorCommand::Stop(*cue_id, StopMode::Soft)
                                }
                            },
                            _ => unreachable!(),
                        };
                        self.executor_tx.send(executor_command).await?;
                    }
                }
                Ok(())
            }
            ControllerCommand::SetPlaybackCursor { cue_id } => {
                if let Some(cursor_cue_id) = cue_id
                    && self
                        .model_handle
                        .get_cue_by_id(&cursor_cue_id)
                        .await
                        .is_none()
                {
                    anyhow::bail!(
                        "Invalid playback cursor destination cue_id. cue_id = {}",
                        cursor_cue_id
                    );
                }
                self.state_tx.send_modify(|state| {
                    if state.playback_cursor.ne(&cue_id) {
                        state.playback_cursor = cue_id;
                        if self
                            .event_tx
                            .send(BackendEvent::PlaybackCursorMoved { cue_id })
                            .is_err()
                        {
                            log::trace!("No UI clients are listening to playback events.");
                        }
                    }
                });
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

    async fn update_playback_cursor(&self) -> Result<()> {
        let state = self.state_tx.borrow().clone();
        let playback_cursor = if let Some(cursor) = state.playback_cursor {
            cursor
        } else {
            anyhow::bail!("Playback cursor unavailable.");
        };
        let next_cursor = if let Some(cue) = self.model_handle.get_cue_by_id(&playback_cursor).await
            && let CueParam::Group { base, children } = &cue.params
            && let GroupMode::StartFirst { enter } = base.mode
            && enter
        {
            if let Some(target_id) = children.get(1) {
                Some(*target_id)
            } else {
                self.model_handle
                    .get_next_cue_id_by_id(&playback_cursor)
                    .await
            }
        } else {
            self.model_handle
                .get_next_cue_id_by_id(&playback_cursor)
                .await
        };
        self.set_playback_cursor(next_cursor).await?;
        Ok(())
    }

    async fn set_playback_cursor(&self, cursor: Option<Uuid>) -> Result<()> {
        self.state_tx.send_modify(|state| {
            state.playback_cursor = cursor;
        });
        self.event_tx
            .send(BackendEvent::PlaybackCursorMoved { cue_id: cursor })?;
        Ok(())
    }

    async fn hard_stop_all(&self) -> Result<()> {
        let state = self.state_tx.borrow().clone();
        for cue_id in state.active_cues.keys() {
            let is_group = self
                .model_handle
                .get_cue_by_id(cue_id)
                .await
                .is_some_and(|cue| matches!(cue.params, CueParam::Group { .. }));
            if !is_group {
                self.executor_tx.send(ExecutorCommand::Stop(*cue_id, StopMode::Hard)).await?;
            }
        }
        Ok(())
    }

    async fn handle_executor_event(&mut self, event: ExecutorEvent) -> Result<(), anyhow::Error> {
        let mut show_state = self.state_tx.borrow().clone();
        let mut state_changed = false;
        let mut send_event = true;

        match &event {
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
                if let Some(chain) = self.model_handle.get_cue_chain_by_id(cue_id).await {
                    match &chain {
                        CueChain::AfterComplete { .. } => {
                            self.wait_tasks.insert(*cue_id, chain);
                        }
                        CueChain::AfterStart { target_id } => {
                            if let Some(target) = target_id {
                                if let Err(e) = self.handle_go(*target).await {
                                    log::error!(
                                        "Failed to perform cue chain. ignoring. error={}",
                                        e
                                    );
                                }
                            } else if let Some(next_id) =
                                self.model_handle.get_next_cue_id_by_id(cue_id).await
                                && let Err(e) = self.handle_go(next_id).await
                            {
                                log::error!("Failed to perform cue chain. ignoring. error={}", e);
                            }
                        }
                        CueChain::DoNotChain => {}
                    }
                } else {
                    log::warn!(
                        "Unknown cue started. model may be broken. cue_id={}",
                        cue_id
                    );
                }
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
            ExecutorEvent::Stopped { cue_id } => {
                self.wait_tasks.remove(cue_id);
                show_state.active_cues.shift_remove(cue_id);
                state_changed = true;
            }
            ExecutorEvent::Completed { cue_id, .. } => {
                if let Some(chain) = self.wait_tasks.remove(cue_id)
                    && let CueChain::AfterComplete { target_id } = chain
                {
                    if let Some(target) = target_id {
                        if let Err(e) = self.handle_go(target).await {
                            log::error!("Failed to perform cue chain. ignoring. error={}", e);
                        }
                    } else if let Some(next_id) =
                        self.model_handle.get_next_cue_id_by_id(cue_id).await
                        && let Err(e) = self.handle_go(next_id).await
                    {
                        log::error!("Failed to perform cue chain. ignoring. error={}", e);
                    }
                }
                show_state.active_cues.shift_remove(cue_id);
                state_changed = true;
            }
            ExecutorEvent::StateParamUpdated { cue_id, params } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.params = *params;
                    state_changed = true;
                }
            }
            ExecutorEvent::Error { cue_id, .. } => {
                show_state.active_cues.shift_remove(cue_id);
                state_changed = true;
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
            ExecutorEvent::PreWaitStopped { cue_id } => {
                show_state.active_cues.shift_remove(cue_id);
                state_changed = true;
            }
            ExecutorEvent::PreWaitCompleted { .. } => {} // skip to keep active cue because cue will be started. but event is emitted for client.
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

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use crate::{
        BackendSettings,
        event::CueStatusEventParam,
        manager::ShowModelManager,
        model::{
            self,
            cue::{
                Cue, CueColor,
                audio::{AudioCueParam, Decibels, Easing, FadeParam, SoundType},
            },
        },
    };

    use super::*;

    use tokio::sync::{
        mpsc::{self, Receiver, Sender},
        watch,
    };

    async fn setup_controller(
        cue_ids: &[Uuid],
    ) -> (
        CueController,
        CueControllerHandle,
        Receiver<ExecutorCommand>,
        Sender<ExecutorEvent>,
        watch::Receiver<ShowState>,
        broadcast::Receiver<BackendEvent>,
    ) {
        let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
        let (playback_event_tx, playback_event_rx) = mpsc::channel::<ExecutorEvent>(32);
        let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
        let (event_tx, event_rx) = broadcast::channel::<BackendEvent>(32);

        let (_, settings_rx) = watch::channel(BackendSettings {
            advance_cursor_when_go: true,
            ..Default::default()
        });

        let (manager, handle) = ShowModelManager::new(event_tx.clone(), settings_rx.clone());
        let mut write_lock = manager.write().await;
        write_lock.name = "TestShowModel".to_string();
        for cue_id in cue_ids {
            write_lock.cue_list.root_ids.push(*cue_id);
            write_lock.cue_list.cues.insert(
                *cue_id,
                Cue {
                    id: *cue_id,
                    number: "1".to_string(),
                    name: None,
                    notes: "".to_string(),
                    color: CueColor::None,
                    pre_wait: 0.0,
                    chain: model::cue::CueChain::DoNotChain,
                    parent_id: None,
                    params: model::cue::CueParam::Audio(AudioCueParam {
                        target: PathBuf::from("./I.G.Y.flac"),
                        start_time: Some(5.0),
                        fade_in_param: Some(FadeParam {
                            duration: 2.0,
                            easing: Easing::Linear,
                        }),
                        end_time: Some(50.0),
                        fade_out_param: Some(FadeParam {
                            duration: 5.0,
                            easing: Easing::InPow(2.0),
                        }),
                        volume: Decibels::IDENTITY,
                        pan: 0.0,
                        repeat: false,
                        sound_type: SoundType::Streaming,
                        envelope: Vec::new(),
                    }),
                },
            );
        }
        let (controller, controller_handle) = CueController::new(
            handle.clone(),
            settings_rx,
            exec_tx,
            playback_event_rx,
            state_tx,
            event_tx,
        );

        (
            controller,
            controller_handle,
            exec_rx,
            playback_event_tx,
            state_rx,
            event_rx,
        )
    }

    #[tokio::test]
    async fn go_command() {
        let cue_id = Uuid::new_v4();
        let (controller, controller_handle, mut exec_rx, _, _, _) =
            setup_controller(&[cue_id]).await;

        tokio::spawn(controller.run());

        controller_handle
            .set_playback_cursor(Some(cue_id))
            .await
            .unwrap();
        controller_handle.go().await.unwrap();

        if let Some(ExecutorCommand::Execute(id)) = exec_rx.recv().await {
            assert_eq!(id, cue_id);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn set_playback_cursor() {
        let cue_id = Uuid::new_v4();
        let cue_id_next = Uuid::new_v4();
        let (controller, controller_handle, _, _, state_rx, mut event_rx) =
            setup_controller(&[cue_id, cue_id_next]).await;

        tokio::spawn(controller.run());

        assert_eq!(state_rx.borrow().playback_cursor, None);

        controller_handle
            .set_playback_cursor(Some(cue_id_next))
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert_eq!(
            event,
            BackendEvent::PlaybackCursorMoved {
                cue_id: Some(cue_id_next)
            }
        );
        if let Some(playback_cursor) = state_rx.borrow().playback_cursor {
            assert_eq!(playback_cursor, cue_id_next);
        }
    }

    #[tokio::test]
    async fn started_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) =
            setup_controller(&[cue_id]).await;

        tokio::spawn(controller.run());

        playback_event_tx
            .send(ExecutorEvent::Started {
                cue_id,
                position: 0.0,
                duration: 43.0,
                initial_params: StateParam::None,
            })
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert!(
            event.eq(&BackendEvent::CueStatus(CueStatusEventParam::Started {
                cue_id,
                position: 0.0,
                duration: 43.0,
                params: StateParam::None
            }))
        );
        if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
            assert_eq!(active_cue.cue_id, cue_id);
            assert_eq!(active_cue.status, PlaybackStatus::Playing);
            assert_eq!(active_cue.duration, 43.0);
            assert_eq!(active_cue.position, 0.0);
            assert_eq!(active_cue.params, StateParam::None);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn progress_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, mut state_rx, mut event_rx) =
            setup_controller(&[cue_id]).await;
        tokio::spawn(controller.run());

        state_rx.mark_unchanged();

        playback_event_tx
            .send(ExecutorEvent::Started {
                cue_id,
                position: 0.0,
                duration: 50.0,
                initial_params: StateParam::None,
            })
            .await
            .unwrap();

        let _ = state_rx.changed().await;
        assert_eq!(
            event_rx.recv().await.unwrap(),
            BackendEvent::CueStatus(CueStatusEventParam::Started {
                cue_id,
                position: 0.0,
                duration: 50.0,
                params: StateParam::None
            })
        );

        playback_event_tx
            .send(ExecutorEvent::Progress {
                cue_id,
                position: 20.0,
                duration: 50.0,
            })
            .await
            .unwrap();

        let _ = state_rx.changed().await;
        if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
            assert_eq!(active_cue.cue_id, cue_id);
            assert_eq!(active_cue.status, PlaybackStatus::Playing);
            assert_eq!(active_cue.position, 20.0);
            assert_eq!(active_cue.duration, 50.0);
        }
        assert!(event_rx.is_empty());
    }

    #[tokio::test]
    async fn pause_n_resume_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, mut state_rx, mut event_rx) =
            setup_controller(&[cue_id]).await;

        tokio::spawn(controller.run());

        state_rx.mark_unchanged();

        playback_event_tx
            .send(ExecutorEvent::Started {
                cue_id,
                position: 0.0,
                duration: 50.0,
                initial_params: StateParam::None,
            })
            .await
            .unwrap();

        let _ = state_rx.changed().await;
        let event = event_rx.recv().await.unwrap();
        assert_eq!(
            event,
            BackendEvent::CueStatus(CueStatusEventParam::Started {
                cue_id,
                position: 0.0,
                duration: 50.0,
                params: StateParam::None
            })
        );

        playback_event_tx
            .send(ExecutorEvent::Paused {
                cue_id,
                position: 21.0,
                duration: 50.0,
            })
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert_eq!(
            event,
            BackendEvent::CueStatus(CueStatusEventParam::Paused {
                cue_id,
                position: 21.0
            })
        );
        if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
            assert_eq!(active_cue.cue_id, cue_id);
            assert_eq!(active_cue.status, PlaybackStatus::Paused);
            assert_eq!(active_cue.position, 21.0);
            assert_eq!(active_cue.duration, 50.0);
        } else {
            unreachable!();
        }

        playback_event_tx
            .send(ExecutorEvent::Resumed { cue_id })
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert!(
            event.eq(&BackendEvent::CueStatus(CueStatusEventParam::Resumed {
                cue_id
            }))
        );
        if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
            assert_eq!(active_cue.cue_id, cue_id);
            assert_eq!(active_cue.status, PlaybackStatus::Playing);
            assert_eq!(active_cue.position, 21.0);
            assert_eq!(active_cue.duration, 50.0);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn completed_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, mut state_rx, mut event_rx) =
            setup_controller(&[cue_id]).await;

        tokio::spawn(controller.run());

        state_rx.mark_unchanged();

        playback_event_tx
            .send(ExecutorEvent::Started {
                cue_id,
                position: 0.0,
                duration: 50.0,
                initial_params: StateParam::None,
            })
            .await
            .unwrap();

        let _ = state_rx.changed().await;
        let event = event_rx.recv().await.unwrap();
        assert_eq!(
            event,
            BackendEvent::CueStatus(CueStatusEventParam::Started {
                cue_id,
                position: 0.0,
                duration: 50.0,
                params: StateParam::None
            })
        );

        playback_event_tx
            .send(ExecutorEvent::Completed { cue_id })
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert!(
            event.eq(&BackendEvent::CueStatus(CueStatusEventParam::Completed {
                cue_id
            }))
        );
        assert!(!state_rx.borrow().active_cues.contains_key(&cue_id));
    }
}
