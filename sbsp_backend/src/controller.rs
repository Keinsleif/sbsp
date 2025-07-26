use std::{collections::HashMap, sync::Arc};

use anyhow::bail;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, watch, RwLock};
use uuid::Uuid;

use crate::{
    event::UiEvent, executor::{ExecutorCommand, ExecutorEvent}, manager::ShowModelManager
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveCue {
    pub cue_id: Uuid,
    pub position: f64,
    pub duration: f64,
    pub status: PlaybackStatus,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", content = "params", rename_all = "camelCase")]
pub enum ControllerCommand {
    Go,
    GoFromCue {
        cue_id: Uuid,
    },
    StopAll,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ShowState {
    pub playback_cursor: Option<Uuid>,
    pub active_cues: HashMap<Uuid, ActiveCue>,
}

impl ShowState {
    pub fn new() -> Self {
        Self {
            playback_cursor: None,
            active_cues: HashMap::new(),
        }
    }
}

pub struct CueController {
    model_manager: ShowModelManager,
    executor_tx: mpsc::Sender<ExecutorCommand>, // Executorへの指示用チャネル
    command_rx: mpsc::Receiver<ControllerCommand>, // 外部からのトリガー受信用チャネル

    executor_event_rx: mpsc::Receiver<ExecutorEvent>,
    state_tx: watch::Sender<ShowState>,
    event_tx: broadcast::Sender<UiEvent>,

    show_state: Arc<RwLock<ShowState>>,
}

impl CueController {
    pub async fn new(
        model_manager: ShowModelManager,
        executor_tx: mpsc::Sender<ExecutorCommand>,
        command_rx: mpsc::Receiver<ControllerCommand>,
        executor_event_rx: mpsc::Receiver<ExecutorEvent>,
        state_tx: watch::Sender<ShowState>,
        event_tx: broadcast::Sender<UiEvent>,
    ) -> Self {
        let manager = model_manager.read().await;
        let show_state = if let Some(first_cue) = manager.cues.first() {
            Arc::new(RwLock::new(ShowState { playback_cursor: Some(first_cue.id), ..Default::default() }))
        } else {
            Arc::new(RwLock::new(ShowState::new()))
        };
        drop(manager);
        Self {
            model_manager,
            executor_tx,
            command_rx,
            executor_event_rx,
            state_tx,
            event_tx,
            show_state,
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
                else => break,
            }
        }
        log::info!("CueController run loop finished.");
    }

    async fn handle_command(&self, command: ControllerCommand) -> Result<(), anyhow::Error> {
        match command {
            ControllerCommand::Go => {
                let state = self.show_state.read().await;
                if let Some(cue_id) = state.playback_cursor {
                    self.handle_go(cue_id).await
                } else {
                    bail!("Playback cursor is not available.")
                }
            },
            ControllerCommand::GoFromCue { cue_id } => {
                self.handle_go(cue_id).await
            }
            ControllerCommand::StopAll => Ok(()), /* TODO */
        }
    }

    async fn handle_go(&self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        let model = self.model_manager.read().await;

        if model.cues.iter().any(|cue| cue.id.eq(&cue_id)) {
            let command = ExecutorCommand::ExecuteCue(cue_id);
            self.executor_tx.send(command).await?;
        } else {
            log::warn!("GO: Reached end of cue list.");
        }
        Ok(())
    }

    /// Executorからの再生イベントを処理します
    async fn handle_executor_event(&self, event: ExecutorEvent) -> Result<(), anyhow::Error> {
        let mut show_state = self.show_state.write().await;

        let mut state_changed = false;

        match &event {
            ExecutorEvent::Started { cue_id } => {
                let active_cue = ActiveCue {
                    cue_id: *cue_id,
                    position: 0.0,
                    duration: 0.0,
                    status: PlaybackStatus::Playing,
                };
                show_state.active_cues.insert(*cue_id, active_cue);
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
                if let Some(active_cue) = show_state.active_cues.get_mut(cue_id) {
                    if !active_cue.status.eq(&PlaybackStatus::Playing) {
                        active_cue.status = PlaybackStatus::Playing;
                        state_changed = true;
                    }
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
        }
        drop(show_state);

        if state_changed && self.state_tx.send(self.show_state.read().await.clone()).is_err() {
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

    use crate::model::{
        self,
        cue::{AudioCueFadeParam, AudioCueLevels, Cue},
    };

    use super::*;

    use kira::sound::Region;
    use tokio::sync::{
        mpsc::{self, Receiver, Sender},
        watch,
    };

    async fn setup_controller(
        cue_id: Uuid,
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

        let manager = ShowModelManager::new();
        manager
            .write_with(|model| {
                model.name = "TestShowModel".to_string();
                model.cues.push(Cue {
                    id: cue_id,
                    number: "1".to_string(),
                    name: "Play IGY".to_string(),
                    notes: "".to_string(),
                    pre_wait: 0.0,
                    post_wait: 0.0,
                    sequence: model::cue::CueSequence::DoNotContinue,
                    param: model::cue::CueParam::Audio {
                        target: PathBuf::from("./I.G.Y.flac"),
                        start_time: Some(5.0),
                        fade_in_param: Some(AudioCueFadeParam {
                            duration: 2.0,
                            easing: kira::Easing::Linear,
                        }),
                        end_time: Some(50.0),
                        fade_out_param: Some(AudioCueFadeParam {
                            duration: 5.0,
                            easing: kira::Easing::InPowi(2),
                        }),
                        levels: AudioCueLevels { master: 0.0 },
                        loop_region: Some(Region {
                            start: kira::sound::PlaybackPosition::Seconds(2.0),
                            end: kira::sound::EndPosition::EndOfAudio,
                        }),
                    },
                });
                cue_id
            })
            .await;

        let controller = CueController::new(
            manager.clone(),
            exec_tx,
            ctrl_rx,
            playback_event_rx,
            state_tx,
            event_tx,
        ).await;

        (controller, ctrl_tx, exec_rx, playback_event_tx, state_rx, event_rx)
    }

    #[tokio::test]
    async fn go_command() {
        let cue_id = Uuid::new_v4();
        let (controller, ctrl_tx, mut exec_rx, _, _, _) = setup_controller(cue_id).await;

        tokio::spawn(controller.run());

        ctrl_tx
            .send(ControllerCommand::Go)
            .await
            .unwrap();

        if let Some(ExecutorCommand::ExecuteCue(id)) = exec_rx.recv().await {
            assert_eq!(id, cue_id);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn go_from_cue_command() {
        let cue_id = Uuid::new_v4();
        let (controller, ctrl_tx, mut exec_rx, _, _, _) = setup_controller(cue_id).await;

        tokio::spawn(controller.run());

        ctrl_tx
            .send(ControllerCommand::GoFromCue { cue_id })
            .await
            .unwrap();

        if let Some(ExecutorCommand::ExecuteCue(id)) = exec_rx.recv().await {
            assert_eq!(id, cue_id);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn started_event() {
        let cue_id = Uuid::new_v4();
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) = setup_controller(cue_id).await;

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
        let (controller, _, _, playback_event_tx, mut state_rx, event_rx) = setup_controller(cue_id).await;

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
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) = setup_controller(cue_id).await;

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
        let (controller, _, _, playback_event_tx, state_rx, mut event_rx) = setup_controller(cue_id).await;

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
