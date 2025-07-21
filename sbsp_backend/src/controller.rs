use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::{executor::{ExecutorCommand, PlaybackEvent}, manager::ShowModelManager};

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
    Go {
        cue_id: Uuid
    },
    StopAll,
}

pub struct CueController {
    model_manager: ShowModelManager,
    executor_tx: mpsc::Sender<ExecutorCommand>, // Executorへの指示用チャネル
    command_rx: mpsc::Receiver<ControllerCommand>, // 外部からのトリガー受信用チャネル

    event_rx: mpsc::Receiver<PlaybackEvent>,
    active_cues: Arc<RwLock<HashMap<Uuid, ActiveCue>>>,
}

impl CueController {
    pub fn new(
        model_manager: ShowModelManager,
        executor_tx: mpsc::Sender<ExecutorCommand>,
        command_rx: mpsc::Receiver<ControllerCommand>,
        event_rx: mpsc::Receiver<PlaybackEvent>,
    ) -> Self {
        Self {
            model_manager,
            executor_tx,
            command_rx,
            event_rx,
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
                    }
                },
                Some(event) = self.event_rx.recv() => {
                    if let Err(e) = self.handle_playback_event(event).await {
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
            PlaybackEvent::Progress { cue_id, position, duration, .. } => {
                if let Some(active_cue) = active_cues.get_mut(&cue_id) {
                    active_cue.position = position;
                    active_cue.duration = duration;
                    active_cue.status = PlaybackStatus::Playing
                }
            }
            PlaybackEvent::Paused { cue_id, position, duration } => {
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