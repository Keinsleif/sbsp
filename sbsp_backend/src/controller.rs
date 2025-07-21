use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc, watch};
use uuid::Uuid;

use crate::{
    executor::{ExecutorCommand, PlaybackEvent},
    manager::ShowModelManager,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Completed,
    Error,
}

#[derive(Debug, Clone)]
pub struct ActiveCue {
    pub cue_id: Uuid,
    pub position: f64,
    pub duration: f64,
    pub status: PlaybackStatus,
}

#[derive(Debug)]
pub enum ControllerCommand {
    Go { cue_id: Uuid },
    StopAll,
}

pub struct CueController {
    model_manager: ShowModelManager,
    executor_tx: mpsc::Sender<ExecutorCommand>, // Executorへの指示用チャネル
    command_rx: mpsc::Receiver<ControllerCommand>, // 外部からのトリガー受信用チャネル

    event_rx: mpsc::Receiver<PlaybackEvent>,
    state_tx: watch::Sender<HashMap<Uuid, ActiveCue>>,
    active_cues: Arc<RwLock<HashMap<Uuid, ActiveCue>>>,
}

impl CueController {
    pub fn new(
        model_manager: ShowModelManager,
        executor_tx: mpsc::Sender<ExecutorCommand>,
        command_rx: mpsc::Receiver<ControllerCommand>,
        event_rx: mpsc::Receiver<PlaybackEvent>,
        state_tx: watch::Sender<HashMap<Uuid, ActiveCue>>,
    ) -> Self {
        Self {
            model_manager,
            executor_tx,
            command_rx,
            event_rx,
            state_tx,
            active_cues: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(mut self) {
        log::info!("CueController run loop started.");
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    if let Err(e) = self.handle_command(command).await {
                        log::error!("Error handling controller command: {:?}", e);
                    } else if self.state_tx.send(self.active_cues.read().await.clone()).is_err() {
                        log::trace!("No UI clients are listening to state updates.");
                    }
                },
                Some(event) = self.event_rx.recv() => {
                    if let Err(e) = self.handle_playback_event(event).await {
                        log::error!("Error handling playback event: {:?}", e);
                    } else if self.state_tx.send(self.active_cues.read().await.clone()).is_err() {
                        log::trace!("No UI clients are listening to state updates.");
                    }
                },
                else => break,
            }
        }
        log::info!("CueController run loop finished.");
    }

    async fn handle_command(&self, command: ControllerCommand) -> Result<(), anyhow::Error> {
        match command {
            ControllerCommand::Go { cue_id } => self.handle_go(cue_id).await,
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
    async fn handle_playback_event(&self, event: PlaybackEvent) -> Result<(), anyhow::Error> {
        let mut active_cues = self.active_cues.write().await;

        match event {
            PlaybackEvent::Started { cue_id } => {
                let active_cue = ActiveCue {
                    cue_id,
                    position: 0.0,
                    duration: 0.0,
                    status: PlaybackStatus::Playing,
                };
                active_cues.insert(cue_id, active_cue);
            }
            PlaybackEvent::Progress {
                cue_id,
                position,
                duration,
                ..
            } => {
                if let Some(active_cue) = active_cues.get_mut(&cue_id) {
                    active_cue.position = position;
                    active_cue.duration = duration;
                    active_cue.status = PlaybackStatus::Playing
                }
            }
            PlaybackEvent::Paused {
                cue_id,
                position,
                duration,
            } => {
                if let Some(active_cue) = active_cues.get_mut(&cue_id) {
                    active_cue.position = position;
                    active_cue.duration = duration;
                    active_cue.status = PlaybackStatus::Paused;
                }
            }
            PlaybackEvent::Resumed { cue_id } => {
                if let Some(active_cue) = active_cues.get_mut(&cue_id) {
                    active_cue.status = PlaybackStatus::Playing;
                }
            }
            PlaybackEvent::Completed { cue_id, .. } => {
                if let Some(mut active_cue) = active_cues.remove(&cue_id) {
                    active_cue.status = PlaybackStatus::Completed;
                    // TODO: Auto-Followロジックをここでトリガー
                }
            }
            PlaybackEvent::Error { cue_id, error, .. } => {
                if let Some(active_cue) = active_cues.get_mut(&cue_id) {
                    active_cue.status = PlaybackStatus::Error;
                    log::error!("State: Cue error on '{}': {}", active_cue.cue_id, error);
                }
            }
        }
        // TODO: ApiServerに状態変更を通知する
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use crate::model::{self, cue::{AudioCueFadeParam, AudioCueLevels, Cue}};

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
        Sender<PlaybackEvent>,
        watch::Receiver<HashMap<Uuid, ActiveCue>>,
    ) {
        let (ctrl_tx, ctrl_rx) = mpsc::channel::<ControllerCommand>(32);
        let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
        let (playback_event_tx, playback_event_rx) = mpsc::channel::<PlaybackEvent>(32);
        let (state_tx, state_rx) = watch::channel::<HashMap<Uuid, ActiveCue>>(HashMap::new());

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
                    loop_region: Some(Region { start: kira::sound::PlaybackPosition::Seconds(2.0), end: kira::sound::EndPosition::EndOfAudio }),
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
        );

        (controller, ctrl_tx, exec_rx, playback_event_tx, state_rx)
    }

    #[tokio::test]
    async fn go_command() {
        let cue_id = Uuid::new_v4();
        let (controller, ctrl_tx, mut exec_rx, _, _) = setup_controller(cue_id).await;

        tokio::spawn(controller.run());

        if let Some(ExecutorCommand::ExecuteCue(id)) = exec_rx.recv().await {
            assert_eq!(id, cue_id);
        } else {
            panic!("Unreachable");
        }
    }
}