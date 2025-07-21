use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    engine::audio_engine::{AudioCommand, AudioEngineEvent},
    manager::ShowModelManager,
    model::cue::{Cue, CueParam},
};

#[derive(Debug)]
pub enum ExecutorCommand {
    ExecuteCue(Uuid), // cue_id
}

#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    Started {
        cue_id: Uuid,
    },
    Progress {
        cue_id: Uuid,
        // ここでは単純な経過時間(秒)としますが、より詳細な情報も可能です
        position: f64,
        duration: f64,
    },
    Paused {
        cue_id: Uuid,
        position: f64,
        duration: f64,
    },
    Resumed {
        cue_id: Uuid,
    },
    Completed {
        cue_id: Uuid,
    },
    Error {
        cue_id: Uuid,
        error: String,
    },
}

#[derive(Debug)]
pub enum EngineEvent {
    Audio(AudioEngineEvent),
    // Midi(MidiEngineEvent), // 将来の拡張
}

pub struct Executor {
    model_manager: ShowModelManager,
    command_rx: mpsc::Receiver<ExecutorCommand>, // CueControllerからの指示受信用
    audio_tx: mpsc::Sender<AudioCommand>,        // AudioEngineへのコマンド送信用
    // midi_tx: mpsc::Sender<MidiCommand>, // 将来の拡張用
    // osc_tx: mpsc::Sender<OscCommand>,   // 将来の拡張用
    playback_event_tx: mpsc::Sender<PlaybackEvent>, // CueControllerへのイベント送信用
    engine_event_rx: mpsc::Receiver<EngineEvent>,   // 各エンジンからのイベント受信用

    active_instances: Arc<RwLock<HashMap<Uuid, Uuid>>>,
}

impl Executor {
    /// 新しいExecutorを生成します。
    pub fn new(
        model_manager: ShowModelManager,
        command_rx: mpsc::Receiver<ExecutorCommand>,
        audio_tx: mpsc::Sender<AudioCommand>,
        playback_event_tx: mpsc::Sender<PlaybackEvent>,
        engine_event_rx: mpsc::Receiver<EngineEvent>,
    ) -> Self {
        Self {
            model_manager,
            command_rx,
            audio_tx,
            playback_event_tx,
            engine_event_rx,
            active_instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Executorのメインループ。指示を待ち受け、処理します。
    pub async fn run(mut self) {
        log::info!("Executor run loop started.");
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    log::debug!("Executor received command: {:?}", command);
                    if let Err(e) = self.process_command(command).await {
                        log::error!("Error processing executor command: {:?}", e);
                    }
                },
                Some(event) = self.engine_event_rx.recv() => {
                    if let Err(e) = self.handle_engine_event(event).await {
                        log::error!("Error handling engine event: {:?}", e);
                    }
                }
                else => break,
            }
        }
        log::info!("Executor run loop finished.");
    }

    /// 個別の指示を処理します。
    async fn process_command(&self, command: ExecutorCommand) -> Result<(), anyhow::Error> {
        match command {
            ExecutorCommand::ExecuteCue(cue_id) => {
                // ShowModelからIDでキューの詳細データを取得
                if let Some(cue) = self.model_manager.get_cue_by_id(&cue_id).await {
                    // キューのタイプに応じて処理を振り分け
                    self.dispatch_cue(&cue).await?;
                } else {
                    log::error!("Cannot execute cue: Cue with id '{}' not found.", cue_id);
                }
            }
        }
        Ok(())
    }

    /// キューを解釈し、適切なエンジンにコマンドを送信します。
    async fn dispatch_cue(&self, cue: &Cue) -> Result<(), anyhow::Error> {
        let instance_id = Uuid::now_v7();
        log::info!(
            "Dispatching cue '{}' with new instance_id '{}'",
            cue.name,
            instance_id
        );

        match &cue.param {
            CueParam::Audio {
                target,
                output,
                start_time,
                fade_in_param,
                end_time,
                fade_out_param,
                levels,
                loop_region,
            } => {
                // AudioEngineが理解できるAudioCommandに変換
                let audio_command = AudioCommand::Play {
                    id: instance_id,
                    filepath: target.clone(),
                    levels: levels.clone(),
                    start_time: *start_time,
                    end_time: *end_time,
                };
                // AudioEngineにコマンドを送信
                self.audio_tx.send(audio_command).await?;
            }
            CueParam::Wait { duration } => {
                // イベント送信用チャネルのクローンを新しいタスクに渡す
                let event_tx = self.playback_event_tx.clone();
                let cue_id = cue.id;
                let wait_duration = *duration;

                // 待機処理を別の非同期タスクとして実行
                tokio::spawn(async move {
                    // 1. 開始イベントを送信
                    if let Err(e) = event_tx.send(PlaybackEvent::Started {
                        cue_id
                    }).await {
                        log::error!("Failed to send Started event for Wait cue: {}", e);
                        return; // 送信に失敗したらタスク終了
                    }

                    // 2. 指定された時間だけ待機
                    tokio::time::sleep(std::time::Duration::from_secs_f64(wait_duration)).await;

                    // 3. 完了イベントを送信
                    if let Err(e) = event_tx.send(PlaybackEvent::Completed {
                        cue_id,
                    }).await {
                        log::error!("Failed to send Completed event for Wait cue: {}", e);
                    }
                });
            }
        }

        self.active_instances.write().await.insert(instance_id, cue.id);
        Ok(())
    }

    async fn handle_engine_event(&self, event: EngineEvent) -> Result<(), anyhow::Error> {
        match event {
            EngineEvent::Audio(audio_event) => {
                let instance_id = audio_event.instance_id();

                let instances = self.active_instances.read().await;
                let Some(cue_id) = instances.get(&instance_id).cloned() else {
                    log::warn!("Received event for unknown instance_id: {}", instance_id);
                    return Ok(());
                };

                let playback_event = match audio_event {
                    AudioEngineEvent::Started { .. } => PlaybackEvent::Started { cue_id },
                    AudioEngineEvent::Progress {
                        position, duration, ..
                    } => PlaybackEvent::Progress {
                        cue_id,
                        position,
                        duration,
                    },
                    AudioEngineEvent::Paused { position, duration, .. } => PlaybackEvent::Paused { cue_id, position, duration },
                    AudioEngineEvent::Resumed { .. } => PlaybackEvent::Resumed { cue_id },
                    AudioEngineEvent::Completed { .. } => {
                        drop(instances);
                        self.active_instances.write().await.remove(&instance_id);
                        PlaybackEvent::Completed { cue_id }
                    }
                    AudioEngineEvent::Error { error, .. } => {
                        drop(instances);
                        self.active_instances.write().await.remove(&instance_id);
                        PlaybackEvent::Error { cue_id, error }
                    }
                };

                self.playback_event_tx.send(playback_event).await?;
            }
        }
        Ok(())
    }
}
