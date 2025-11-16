mod command;
mod handle;
pub mod state;

pub use command::ControllerCommand;
pub use handle::CueControllerHandle;

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use tokio::sync::{RwLock, broadcast, mpsc, watch};
use uuid::Uuid;

use crate::{
    BackendSettings, controller::state::{ActiveCue, PlaybackStatus, ShowState, StateParam}, event::UiEvent, executor::{ExecutorCommand, ExecutorEvent}, manager::ShowModelHandle, model::cue::CueSequence
};

pub struct CueController {
    model_handle: ShowModelHandle,
    settings_rx: watch::Receiver<BackendSettings>,
    executor_tx: mpsc::Sender<ExecutorCommand>,
    command_rx: mpsc::Receiver<ControllerCommand>,

    executor_event_rx: mpsc::Receiver<ExecutorEvent>,
    state_tx: watch::Sender<ShowState>,
    event_tx: broadcast::Sender<UiEvent>,
    event_rx: broadcast::Receiver<UiEvent>,

    wait_tasks: Arc<RwLock<HashMap<Uuid, CueSequence>>>,
}

impl CueController {
    pub fn new(
        model_handle: ShowModelHandle,
        settings_rx: watch::Receiver<BackendSettings>,
        executor_tx: mpsc::Sender<ExecutorCommand>,
        executor_event_rx: mpsc::Receiver<ExecutorEvent>,
        state_tx: watch::Sender<ShowState>,
        event_tx: broadcast::Sender<UiEvent>,
    ) -> (Self, CueControllerHandle) {
        let event_rx = event_tx.subscribe();
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
                wait_tasks: Arc::new(RwLock::new(HashMap::new())),
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
                Ok(event) = self.event_rx.recv() => {
                    match event {
                        UiEvent::ShowModelLoaded{..} => {
                            if self.state_tx.borrow().playback_cursor.is_none() {
                                let model = self.model_handle.read().await;
                                if let Some(first_cue) = model.cues.first() {
                                    self.state_tx.send_modify(|state| {
                                        state.playback_cursor = Some(first_cue.id);
                                    });
                                }
                            }
                            if let Err(e) = self.handle_command(ControllerCommand::StopAll).await {
                                log::error!("Failed to stop active cues before load. {}", e);
                            }
                            if let Err(e) = self.handle_command(ControllerCommand::StopAll).await {
                                log::error!("Failed to stop active cues before load. {}", e);
                            }
                        },
                        UiEvent::CueAdded{cue, ..} => {
                            if self.state_tx.borrow().playback_cursor.is_none() {
                                self.state_tx.send_modify(|state| {
                                    state.playback_cursor = Some(cue.id);
                                });
                            }
                        },
                        UiEvent::CueRemoved{cue_id} => {
                            if self.state_tx.borrow().playback_cursor.eq(&Some(cue_id)) {
                                let model = self.model_handle.read().await;
                                if let Some(first_cue) = model.cues.first() {
                                    self.state_tx.send_modify(|state| {
                                        state.playback_cursor = Some(first_cue.id);
                                    });
                                } else {
                                    self.state_tx.send_modify(|state| {
                                        state.playback_cursor = None;
                                    });
                                }
                            }
                            if self.state_tx.borrow().active_cues.contains_key(&cue_id) {
                                if let Err(e) = self.executor_tx.send(ExecutorCommand::Stop(cue_id)).await {
                                    log::error!("Failed to stop removed cue. {}", e);
                                }
                                if let Err(e) = self.executor_tx.send(ExecutorCommand::Stop(cue_id)).await {
                                    log::error!("Failed to stop removed cue. {}", e);
                                }
                            }
                        }
                        UiEvent::SettingsUpdated{ new_settings } => {
                            if let Err(e) = self.executor_tx.send(ExecutorCommand::ReconfigureEngines(new_settings)).await {
                                log::error!("{}", e);
                            }
                        }
                        _ => {}
                    }
                }
                else => break,
            }
        }
        log::info!("CueController run loop finished.");
    }

    async fn handle_command(&self, command: ControllerCommand) -> Result<(), anyhow::Error> {
        match command {
            ControllerCommand::Go => {
                let state = self.state_tx.borrow().clone();
                let cue_id = if let Some(cursor) = state.playback_cursor {
                    cursor
                } else {
                    return Err(anyhow::anyhow!("GO: Playback Cursor is unavailable."))
                };
                self.handle_go(cue_id).await?;
                let advance_cursor_when_go = {
                    let settings = self.settings_rx.borrow();
                    settings.advance_cursor_when_go
                };
                if advance_cursor_when_go {
                    self.update_playback_cursor().await?;
                }
                Ok(())
            }
            ControllerCommand::Load(cue_id)
            | ControllerCommand::SeekTo(cue_id, ..)
            | ControllerCommand::SeekBy(cue_id, ..)
            | ControllerCommand::Pause(cue_id)
            | ControllerCommand::Resume(cue_id)
            | ControllerCommand::Stop(cue_id)
            | ControllerCommand::PerformAction(cue_id, ..) => {
                let model = self.model_handle.read().await;

                if model.cues.iter().any(|cue| cue.id.eq(&cue_id)) {
                    let executor_command = command.into_executor_command();
                    self.executor_tx.send(executor_command).await?;
                } else {
                    anyhow::bail!("DIRECT: Cue not found. cue_id={}", cue_id);
                }
                Ok(())
            }
            ControllerCommand::PauseAll
            | ControllerCommand::ResumeAll
            | ControllerCommand::StopAll => {
                let state = self.state_tx.borrow().clone();

                for cue_id in state.active_cues.keys() {
                    let executor_command = command
                        .try_all_into_single_executor_command(*cue_id);
                    self.executor_tx.send(executor_command).await?;
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
                            .send(UiEvent::PlaybackCursorMoved { cue_id })
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
        let model = self.model_handle.read().await;
        let state = self.state_tx.borrow().clone();

        if model.cues.iter().any(|cue| cue.id.eq(&cue_id)) {
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
        let model = self.model_handle.read().await;
        let state = self.state_tx.borrow().clone();
        let playback_cursor = if let Some(cursor) = state.playback_cursor {
            cursor
        } else {
            anyhow::bail!("Playback cursor unavailable.");
        };
        if let Some(cue_index) = model
            .cues
            .iter()
            .position(|cue| cue.id == playback_cursor)
        {
            let next_cue_id = if cue_index + 1 < model.cues.len() {
                Some(model.cues[cue_index + 1].id)
            } else {
                None
            };
            self.set_playback_cursor(next_cue_id).await?;
        }
        Ok(())
    }

    async fn set_playback_cursor(&self, cursor: Option<Uuid>) -> Result<()> {
        self.state_tx.send_modify(|state| {
            state.playback_cursor = cursor;
        });
        self.event_tx
            .send(UiEvent::PlaybackCursorMoved { cue_id: cursor })?;
        Ok(())
    }

    async fn handle_executor_event(&self, event: ExecutorEvent) -> Result<(), anyhow::Error> {
        let mut show_state = self.state_tx.borrow().clone();
        let mut state_changed = false;

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
                initial_params,
            } => {
                if let Some(cue) = self
                    .model_handle
                    .read()
                    .await
                    .cues
                    .iter()
                    .find(|cue| cue.id == *cue_id) {
                    if !matches!(cue.sequence, CueSequence::DoNotContinue) {
                        self.wait_tasks
                            .write()
                            .await
                            .insert(*cue_id, cue.sequence.clone());
                    }
                } else {
                    log::warn!("Unknown cue started. model may be broken. cue_id={}", cue_id);
                }
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.position = 0.0;
                    active_cue.duration = 0.0;
                    active_cue.status = PlaybackStatus::Playing;
                    active_cue.params = *initial_params;
                } else {
                    let active_cue = ActiveCue {
                        cue_id: *cue_id,
                        position: 0.0,
                        duration: 0.0,
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
                {
                    let mut wait_tasks = self.wait_tasks.write().await;
                    if let Some(sequence) = wait_tasks.get(cue_id)
                        && let CueSequence::AutoContinue {
                            target_id,
                            post_wait,
                        } = sequence
                        && position > post_wait
                    {
                        if let Some(target) = target_id {
                            if let Err(e) = self.handle_go(*target).await {
                                log::error!("Failed to perform cue sequence. ignoring. error={}", e);
                            }
                        } else {
                            let model = self.model_handle.read().await;
                            if let Some(cue_index) =
                            model.cues.iter().position(|cue| cue.id == *cue_id)
                            && cue_index + 1 < model.cues.len()
                            && let Err(e) = self.handle_go(model.cues[cue_index + 1].id).await {
                                log::error!("Failed to perform cue sequence. ignoring. error={}", e);
                            }
                        }
                        wait_tasks.remove(cue_id);
                    }
                }
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
                } else {
                    show_state.active_cues.insert(
                        *cue_id,
                        ActiveCue {
                            cue_id: *cue_id,
                            position: *position,
                            duration: *duration,
                            status: PlaybackStatus::Playing,
                            params: StateParam::None,
                        },
                    );
                    state_changed = true;
                }
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
                    show_state.active_cues.insert(
                        *cue_id,
                        ActiveCue {
                            cue_id: *cue_id,
                            position: *position,
                            duration: *duration,
                            status: PlaybackStatus::Paused,
                            params: StateParam::None,
                        },
                    );
                    state_changed = true;
                }
            }
            ExecutorEvent::Resumed { cue_id } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id)
                    && !active_cue.status.eq(&PlaybackStatus::Playing)
                {
                    active_cue.status = PlaybackStatus::Playing;
                    state_changed = true;
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
                    }
                } else {
                    show_state.active_cues.insert(
                        *cue_id,
                        ActiveCue {
                            cue_id: *cue_id,
                            position: *position,
                            duration: *duration,
                            status: PlaybackStatus::Stopping,
                            params: StateParam::None,
                        },
                    );
                    state_changed = true;
                }
            }
            ExecutorEvent::Stopped { cue_id } => {
                self.wait_tasks.write().await.remove(cue_id);
                self.state_tx.send_modify(|state| {
                    if let Some(active_cue) = state.active_cues.get_mut(cue_id) {
                        active_cue.status = PlaybackStatus::Stopped;
                    }
                });
                show_state.active_cues.shift_remove(cue_id);
                state_changed = true;
            }
            ExecutorEvent::Completed { cue_id, .. } => {
                let mut wait_tasks = self.wait_tasks.write().await;
                if let Some(sequence) = wait_tasks.remove(cue_id)
                    && let CueSequence::AutoFollow { target_id } = sequence
                {
                    if let Some(target) = target_id {
                        if let Err(e) = self.handle_go(target).await {
                            log::error!("Failed to perform cue sequence. ignoring. error={}", e);
                        }
                    } else {
                        let model = self.model_handle.read().await;
                        if let Some(cue_index) = model.cues.iter().position(|cue| cue.id == *cue_id)
                        && cue_index + 1 < model.cues.len()
                        && let Err(e) = self.handle_go(model.cues[cue_index + 1].id).await {
                            log::error!("Failed to perform cue sequence. ignoring. error={}", e);
                        }
                    }
                }
                drop(wait_tasks);
                self.state_tx.send_modify(|state| {
                    if let Some(active_cue) = state.active_cues.get_mut(cue_id) {
                        active_cue.status = PlaybackStatus::Completed;
                    }
                });
                show_state.active_cues.shift_remove(cue_id);
                state_changed = true;
            }
            ExecutorEvent::StateParamUpdated { cue_id, params } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.params = *params;
                    state_changed = true;
                }
            }
            ExecutorEvent::Error { cue_id, error, .. } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.status = PlaybackStatus::Error;
                    state_changed = true;
                    log::error!("State: Cue error on '{}': {}", active_cue.cue_id, error);
                }
            }
            ExecutorEvent::PreWaitStarted { cue_id } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.position = 0.0;
                    active_cue.duration = 0.0;
                    active_cue.status = PlaybackStatus::PreWaiting;
                } else {
                    let active_cue = ActiveCue {
                        cue_id: *cue_id,
                        position: 0.0,
                        duration: 0.0,
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
                } else {
                    show_state.active_cues.insert(
                        *cue_id,
                        ActiveCue {
                            cue_id: *cue_id,
                            position: *position,
                            duration: *duration,
                            status: PlaybackStatus::PreWaiting,
                            params: StateParam::None,
                        },
                    );
                    state_changed = true;
                }
            }
            ExecutorEvent::PreWaitPaused {
                cue_id,
                position,
                duration,
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    if !active_cue.status.eq(&PlaybackStatus::Paused) {
                        active_cue.position = *position;
                        active_cue.duration = *duration;
                        active_cue.status = PlaybackStatus::PreWaitPaused;
                    }
                } else {
                    show_state.active_cues.insert(
                        *cue_id,
                        ActiveCue {
                            cue_id: *cue_id,
                            position: *position,
                            duration: *duration,
                            status: PlaybackStatus::PreWaitPaused,
                            params: StateParam::None,
                        },
                    );
                }
                state_changed = true;
            }
            ExecutorEvent::PreWaitResumed { cue_id } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id)
                    && !active_cue.status.eq(&PlaybackStatus::PreWaiting)
                {
                    active_cue.status = PlaybackStatus::PreWaiting;
                    state_changed = true;
                }
            }
            ExecutorEvent::PreWaitStopped { cue_id } => {
                show_state.active_cues.shift_remove(cue_id);
                state_changed = true;
            }
            ExecutorEvent::PreWaitCompleted { .. } => {}
        }

        if state_changed && self.state_tx.send(show_state).is_err() {
            log::trace!("No UI clients are listening to state updates.");
        }

        match &event {
            ExecutorEvent::Started { .. }
            | ExecutorEvent::Paused { .. }
            | ExecutorEvent::Resumed { .. }
            | ExecutorEvent::Completed { .. }
            | ExecutorEvent::Error { .. } => {
                if state_changed && self.event_tx.send(UiEvent::from(event)).is_err() {
                    log::trace!("No UI clients are listening to playback events.");
                }
            }
            _ => (),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use crate::{
        BackendSettings,
        manager::ShowModelManager,
        model::{
            self,
            cue::{
                Cue,
                audio::{AudioCueParam, FadeParam, Easing, SoundType},
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
        broadcast::Receiver<UiEvent>,
    ) {
        let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
        let (playback_event_tx, playback_event_rx) = mpsc::channel::<ExecutorEvent>(32);
        let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
        let (event_tx, event_rx) = broadcast::channel::<UiEvent>(32);

        let (_, settings_rx) = watch::channel(BackendSettings {
                advance_cursor_when_go: true,
                ..Default::default()
        });

        let (manager, handle) = ShowModelManager::new(event_tx.clone(), settings_rx.clone());
        let mut write_lock = manager.write().await;
        write_lock.name = "TestShowModel".to_string();
        for cue_id in cue_ids {
            write_lock.cues.push(Cue {
                id: *cue_id,
                number: "1".to_string(),
                name: None,
                notes: "".to_string(),
                pre_wait: 0.0,
                sequence: model::cue::CueSequence::DoNotContinue,
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
                        easing: Easing::InPowi(2),
                    }),
                    volume: 0.0,
                    pan: 0.0,
                    repeat: false,
                    sound_type: SoundType::Streaming,
                }),
            });
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
        println!("{}", cue_id);
        let cue_id_next = Uuid::new_v4();
        println!("{}", cue_id_next);
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
            UiEvent::PlaybackCursorMoved {
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
                initial_params: StateParam::None,
            })
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert!(event.eq(&UiEvent::CueStarted { cue_id }));
        if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
            assert_eq!(active_cue.cue_id, cue_id);
            assert_eq!(active_cue.status, PlaybackStatus::Playing);
            assert_eq!(active_cue.duration, 0.0);
            assert_eq!(active_cue.position, 0.0);
            assert_eq!(active_cue.params, StateParam::None);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn progress_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, mut state_rx, event_rx) =
            setup_controller(&[cue_id]).await;
        state_rx.mark_unchanged();

        tokio::spawn(controller.run());

        playback_event_tx
            .send(ExecutorEvent::Progress {
                cue_id,
                position: 20.0,
                duration: 50.0,
            })
            .await
            .unwrap();

        assert!(event_rx.is_empty());
        state_rx.changed().await.unwrap();
        if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
            assert_eq!(active_cue.cue_id, cue_id);
            assert_eq!(active_cue.status, PlaybackStatus::Playing);
            assert_eq!(active_cue.position, 20.0);
            assert_eq!(active_cue.duration, 50.0);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn pause_n_resume_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) =
            setup_controller(&[cue_id]).await;

        tokio::spawn(controller.run());

        playback_event_tx
            .send(ExecutorEvent::Paused {
                cue_id,
                position: 21.0,
                duration: 50.0,
            })
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert!(event.eq(&UiEvent::CuePaused { cue_id }));
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
        assert!(event.eq(&UiEvent::CueResumed { cue_id }));
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
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) =
            setup_controller(&[cue_id]).await;

        tokio::spawn(controller.run());

        playback_event_tx
            .send(ExecutorEvent::Completed { cue_id })
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert!(event.eq(&UiEvent::CueCompleted { cue_id }));
        assert!(!state_rx.borrow().active_cues.contains_key(&cue_id));
    }
}
