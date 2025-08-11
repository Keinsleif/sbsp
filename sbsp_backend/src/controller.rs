pub mod state;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;

use crate::{
    controller::state::{ActiveCue, PlaybackStatus, ShowState}, event::UiEvent, executor::{ExecutorCommand, ExecutorEvent}, manager::ShowModelHandle
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", content = "params", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ControllerCommand {
    Go,
    StopAll,
    SetPlaybackCursor {
        cue_id: Uuid,
    },
}
pub struct CueController {
    model_handle: ShowModelHandle,
    executor_tx: mpsc::Sender<ExecutorCommand>, // Executorへの指示用チャネル
    command_rx: mpsc::Receiver<ControllerCommand>, // 外部からのトリガー受信用チャネル

    executor_event_rx: mpsc::Receiver<ExecutorEvent>,
    state_tx: watch::Sender<ShowState>,
    event_tx: broadcast::Sender<UiEvent>,
    event_rx: broadcast::Receiver<UiEvent>,
}

impl CueController {
    pub fn new(
        model_handle: ShowModelHandle,
        executor_tx: mpsc::Sender<ExecutorCommand>,
        command_rx: mpsc::Receiver<ControllerCommand>,
        executor_event_rx: mpsc::Receiver<ExecutorEvent>,
        state_tx: watch::Sender<ShowState>,
        event_tx: broadcast::Sender<UiEvent>,
    ) -> Self {
        let event_rx = event_tx.subscribe();
        Self {
            model_handle,
            executor_tx,
            command_rx,
            executor_event_rx,
            state_tx,
            event_tx,
            event_rx,
        }
    }

    pub async fn run(mut self) {
        log::info!("CueController run loop started.");
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    if let Err(e) = self.handle_command(command).await {
                        log::error!("Error handling controller command: {:?}", e);
                    }
                },
                Some(event) = self.executor_event_rx.recv() => {
                    if let Err(e) = self.handle_executor_event(event).await {
                        log::error!("Error handling playback event: {:?}", e);
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
                let cue_id = state.playback_cursor.expect("Playback Cursor is unavailable.");
                self.handle_go(cue_id).await
            },
            ControllerCommand::StopAll => Ok(()), /* TODO */
            ControllerCommand::SetPlaybackCursor { cue_id } => {
                if self.model_handle.get_cue_by_id(&cue_id).await.is_some() {
                    self.state_tx.send_modify(|state| {
                        if state.playback_cursor.ne(&Some(cue_id)) {
                            state.playback_cursor = Some(cue_id);
                            if self.event_tx.send(UiEvent::PlaybackCursorMoved { cue_id }).is_err() {
                                log::trace!("No UI clients are listening to playback events.");
                            }
                        }
                    });
                }
                Ok(())
            }
        }
    }

    async fn handle_go(&self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        let model = self.model_handle.read().await;

        if model.cues.iter().any(|cue| cue.id.eq(&cue_id)) {
            let command = ExecutorCommand::Execute(cue_id);
            self.executor_tx.send(command).await?;
            let model = self.model_handle.read().await;
            if let Some(cue_index) = model.cues.iter().position(|cue| cue.id == cue_id) {
                if cue_index + 1 < model.cues.len() {
                    self.state_tx.send_modify(|state| {
                        state.playback_cursor = Some(model.cues[cue_index + 1].id);
                    });
                } else {
                    self.state_tx.send_modify(|state| {
                        state.playback_cursor = None;
                    });
                }
            } 
        } else {
            log::warn!("GO: Reached end of cue list.");
        }
        Ok(())
    }

    /// Executorからの再生イベントを処理します
    async fn handle_executor_event(&self, event: ExecutorEvent) -> Result<(), anyhow::Error> {
        let mut show_state = self.state_tx.borrow().clone();
        let mut state_changed = false;

        match &event {
            ExecutorEvent::Started { cue_id } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.position = 0.0;
                    active_cue.duration = 0.0;
                    active_cue.status = PlaybackStatus::Playing;
                } else {
                    let active_cue = ActiveCue {
                        cue_id: *cue_id,
                        position: 0.0,
                        duration: 0.0,
                        status: PlaybackStatus::Playing,
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
                    active_cue.position = *position;
                    active_cue.duration = *duration;
                    active_cue.status = PlaybackStatus::Playing
                } else {
                    show_state.active_cues.insert(
                        *cue_id,
                        ActiveCue {
                            cue_id: *cue_id,
                            position: *position,
                            duration: *duration,
                            status: PlaybackStatus::Playing,
                        },
                    );
                }
                state_changed = true;
            }
            ExecutorEvent::Paused {
                cue_id,
                position,
                duration,
            } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    if !active_cue.status.eq(&PlaybackStatus::Paused) {
                        active_cue.position = *position;
                        active_cue.duration = *duration;
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
                        },
                    );
                    state_changed = true;
                }
            }
            ExecutorEvent::Resumed { cue_id } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id)
                    && !active_cue.status.eq(&PlaybackStatus::Playing) {
                        active_cue.status = PlaybackStatus::Playing;
                        state_changed = true;
                }
            }
            ExecutorEvent::Completed { cue_id, .. } => {
                if let Some(mut active_cue) = show_state.active_cues.remove(cue_id) {
                    active_cue.status = PlaybackStatus::Completed;
                    state_changed = true;
                    // TODO: Auto-Followロジックをここでトリガー
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
                    };
                    show_state.active_cues.insert(*cue_id, active_cue);
                }
                state_changed = true;
            },
            ExecutorEvent::PreWaitProgress { cue_id, position, duration } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    active_cue.position = *position;
                    active_cue.duration = *duration;
                    active_cue.status = PlaybackStatus::PreWaiting
                } else {
                    show_state.active_cues.insert(
                        *cue_id,
                        ActiveCue {
                            cue_id: *cue_id,
                            position: *position,
                            duration: *duration,
                            status: PlaybackStatus::PreWaiting,
                        },
                    );
                }
                state_changed = true;
            },
            ExecutorEvent::PreWaitPaused { cue_id, position, duration } => {
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
                        },
                    );
                }
                state_changed = true;
            },
            ExecutorEvent::PreWaitResumed { cue_id } => {
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id)
                    && !active_cue.status.eq(&PlaybackStatus::PreWaiting) {
                        active_cue.status = PlaybackStatus::PreWaiting;
                        state_changed = true;
                }
            }
            ExecutorEvent::PreWaitCompleted { .. } => {},
        }

        if state_changed && self.state_tx.send(show_state).is_err() {
            log::trace!("No UI clients are listening to state updates.");
        }

        match &event {
            ExecutorEvent::Started { .. } |
            ExecutorEvent::Paused { .. } |
            ExecutorEvent::Resumed { .. } |
            ExecutorEvent::Completed { .. } |
            ExecutorEvent::Error { .. } => {
                if self.event_tx.send(UiEvent::from(event)).is_err() {
                    log::trace!("No UI clients are listening to playback events.");
                }
            },
            _ => ()
        }
        // TODO: ApiServerに状態変更を通知する
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use crate::{manager::ShowModelManager, model::{
        self,
        cue::{AudioCueFadeParam, AudioCueLevels, Cue, Easing},
    }};

    use super::*;

    use tokio::sync::{
        mpsc::{self, Receiver, Sender},
        watch,
    };

    async fn setup_controller(
        cue_ids: &[Uuid],
    ) -> (
        CueController,
        Sender<ControllerCommand>,
        Receiver<ExecutorCommand>,
        Sender<ExecutorEvent>,
        watch::Receiver<ShowState>,
        broadcast::Receiver<UiEvent>,
    ) {
        let (ctrl_tx, ctrl_rx) = mpsc::channel::<ControllerCommand>(32);
        let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
        let (playback_event_tx, playback_event_rx) = mpsc::channel::<ExecutorEvent>(32);
        let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
        let (event_tx, event_rx) = broadcast::channel::<UiEvent>(32);

        let (manager, handle) = ShowModelManager::new(event_tx.clone());
        manager
            .write_with(|model| {
                model.name = "TestShowModel".to_string();
                for cue_id in cue_ids {
                    model.cues.push(Cue {
                        id: *cue_id,
                        number: "1".to_string(),
                        name: "Play IGY".to_string(),
                        notes: "".to_string(),
                        pre_wait: 0.0,
                        sequence: model::cue::CueSequence::DoNotContinue,
                        params: model::cue::CueParam::Audio {
                            target: PathBuf::from("./I.G.Y.flac"),
                            start_time: Some(5.0),
                            fade_in_param: Some(AudioCueFadeParam {
                                duration: 2.0,
                                easing: Easing::Linear,
                            }),
                            end_time: Some(50.0),
                            fade_out_param: Some(AudioCueFadeParam {
                                duration: 5.0,
                                easing: Easing::InPowi(2),
                            }),
                            levels: AudioCueLevels { master: 0.0 },
                            loop_region: Some((Some(2.0), None).into()),
                        },
                    });
                }
            })
            .await;

        let controller = CueController::new(
            handle.clone(),
            exec_tx,
            ctrl_rx,
            playback_event_rx,
            state_tx,
            event_tx,
        );

        (controller, ctrl_tx, exec_rx, playback_event_tx, state_rx, event_rx)
    }

    #[tokio::test]
    async fn go_command() {
        let cue_id = Uuid::new_v4();
        let (controller, ctrl_tx, mut exec_rx, _, _, _) = setup_controller(&[cue_id]).await;

        tokio::spawn(controller.run());

        ctrl_tx
            .send(ControllerCommand::Go)
            .await
            .unwrap();

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
        let (controller, ctrl_tx, _, _, state_rx, mut event_rx) = setup_controller(&[cue_id, cue_id_next]).await;

        tokio::spawn(controller.run());

        assert_eq!(state_rx.borrow().playback_cursor, Some(cue_id));

        ctrl_tx.send(ControllerCommand::SetPlaybackCursor { cue_id: cue_id_next }).await.unwrap();

        let event = event_rx.recv().await.unwrap();
        assert_eq!(event, UiEvent::PlaybackCursorMoved { cue_id: cue_id_next });
        if let Some(playback_cursor) = state_rx.borrow().playback_cursor {
            assert_eq!(playback_cursor, cue_id_next);
        }
    }

    #[tokio::test]
    async fn started_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

        tokio::spawn(controller.run());

        playback_event_tx
            .send(ExecutorEvent::Started { cue_id })
            .await
            .unwrap();

        let event = event_rx.recv().await.unwrap();
        assert!(event.eq(&UiEvent::CueStarted {cue_id}));
        if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
            assert_eq!(active_cue.cue_id, cue_id);
            assert_eq!(active_cue.status, PlaybackStatus::Playing);
            assert_eq!(active_cue.duration, 0.0);
            assert_eq!(active_cue.position, 0.0);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn progress_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, mut state_rx, event_rx) = setup_controller(&[cue_id]).await;
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
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

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
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

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
