use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    engine::audio_engine::{AudioCommand, AudioEngineEvent, PlayCommandData},
    manager::ShowModelHandle,
    model::cue::{Cue, CueParam},
};

#[derive(Debug)]
pub enum ExecutorCommand {
    ExecuteCue(Uuid), // cue_id
}

#[derive(Debug, Clone)]
pub enum ExecutorEvent {
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
    model_handle: ShowModelHandle,
    command_rx: mpsc::Receiver<ExecutorCommand>, // CueControllerからの指示受信用
    audio_tx: mpsc::Sender<AudioCommand>,        // AudioEngineへのコマンド送信用
    // midi_tx: mpsc::Sender<MidiCommand>, // 将来の拡張用
    // osc_tx: mpsc::Sender<OscCommand>,   // 将来の拡張用
    playback_event_tx: mpsc::Sender<ExecutorEvent>, // CueControllerへのイベント送信用
    engine_event_rx: mpsc::Receiver<EngineEvent>,   // 各エンジンからのイベント受信用

    active_instances: Arc<RwLock<HashMap<Uuid, Uuid>>>,
}

impl Executor {
    /// 新しいExecutorを生成します。
    pub fn new(
        model_handle: ShowModelHandle,
        command_rx: mpsc::Receiver<ExecutorCommand>,
        audio_tx: mpsc::Sender<AudioCommand>,
        playback_event_tx: mpsc::Sender<ExecutorEvent>,
        engine_event_rx: mpsc::Receiver<EngineEvent>,
    ) -> Self {
        Self {
            model_handle,
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
                if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
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
                    data: PlayCommandData {
                        filepath: target.clone(),
                        levels: levels.clone(),
                        start_time: *start_time,
                        fade_in_param: *fade_in_param,
                        end_time: *end_time,
                        fade_out_param: *fade_out_param,
                        loop_region: *loop_region,
                    },
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
                    if let Err(e) = event_tx.send(ExecutorEvent::Started { cue_id }).await {
                        log::error!("Failed to send Started event for Wait cue: {}", e);
                        return; // 送信に失敗したらタスク終了
                    }

                    // 2. 指定された時間だけ待機
                    tokio::time::sleep(std::time::Duration::from_secs_f64(wait_duration)).await;

                    // 3. 完了イベントを送信
                    if let Err(e) = event_tx.send(ExecutorEvent::Completed { cue_id }).await {
                        log::error!("Failed to send Completed event for Wait cue: {}", e);
                    }
                });
            }
        }

        self.active_instances
            .write()
            .await
            .insert(instance_id, cue.id);
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
                    AudioEngineEvent::Started { .. } => ExecutorEvent::Started { cue_id },
                    AudioEngineEvent::Progress {
                        position, duration, ..
                    } => ExecutorEvent::Progress {
                        cue_id,
                        position,
                        duration,
                    },
                    AudioEngineEvent::Paused {
                        position, duration, ..
                    } => ExecutorEvent::Paused {
                        cue_id,
                        position,
                        duration,
                    },
                    AudioEngineEvent::Resumed { .. } => ExecutorEvent::Resumed { cue_id },
                    AudioEngineEvent::Completed { .. } => {
                        drop(instances);
                        self.active_instances.write().await.remove(&instance_id);
                        ExecutorEvent::Completed { cue_id }
                    }
                    AudioEngineEvent::Error { error, .. } => {
                        drop(instances);
                        self.active_instances.write().await.remove(&instance_id);
                        ExecutorEvent::Error { cue_id, error }
                    }
                };

                self.playback_event_tx.send(playback_event).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf};

    use kira::sound::Region;
    use tokio::sync::{broadcast, mpsc::{self, Receiver, Sender}};
    use uuid::Uuid;

    use crate::{
        engine::audio_engine::{AudioCommand, AudioEngineEvent}, event::UiEvent, manager::ShowModelManager, model::{
            self,
            cue::{AudioCueFadeParam, AudioCueLevels, Cue},
        }
    };

    async fn setup_executor(cue_id: Uuid) -> (ShowModelManager, Sender<ExecutorCommand>, Receiver<AudioCommand>, Sender<EngineEvent>, Receiver<ExecutorEvent>) {
        let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
        let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
        let (playback_event_tx, playback_event_rx) = mpsc::channel::<ExecutorEvent>(32);
        let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);
        let (event_tx, _) = broadcast::channel::<UiEvent>(32);

        let (manager, handle) = ShowModelManager::new(event_tx.clone());
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

        let executor = Executor::new(
            handle.clone(),
            exec_rx,
            audio_tx,
            playback_event_tx,
            engine_event_rx,
        );

        tokio::spawn(executor.run());

        (manager, exec_tx, audio_rx, engine_event_tx, playback_event_rx)
    }
    

    #[tokio::test]
    async fn play_command() {
        let cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, _, _) = setup_executor(cue_id).await;

        let old_id = Uuid::now_v7();

        exec_tx
            .send(ExecutorCommand::ExecuteCue(cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        if let AudioCommand::Play { id, data } = command {
            assert!(id > old_id);
            let now_id = Uuid::now_v7();
            assert!(id < now_id);
            assert_eq!(data.filepath, PathBuf::from("./I.G.Y.flac"));
            assert_eq!(data.levels, AudioCueLevels { master: 0.0 });
            assert_eq!(data.start_time, Some(5.0));
            assert_eq!(data.fade_in_param, Some(AudioCueFadeParam { duration: 2.0, easing: kira::Easing::Linear }));
            assert_eq!(data.end_time, Some(50.0));
            assert_eq!(data.fade_out_param, Some(AudioCueFadeParam { duration: 5.0, easing: kira::Easing::InPowi(2) }));
            assert_eq!(data.loop_region, Some(Region { start: kira::sound::PlaybackPosition::Seconds(2.0), end: kira::sound::EndPosition::EndOfAudio }));
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn started_event() {
        let orig_cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) = setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::ExecuteCue(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx.send(EngineEvent::Audio(AudioEngineEvent::Started { instance_id })).await.unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Started { cue_id  } = event {
                assert_eq!(cue_id, orig_cue_id);
            } else {
                panic!("Wrong Playback Event emitted.");
            }
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn progress_event() {
        let orig_cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) = setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::ExecuteCue(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx.send(EngineEvent::Audio(AudioEngineEvent::Progress { instance_id, position: 20.0, duration: 50.0 })).await.unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Progress {cue_id, position, duration } = event {
                assert_eq!(cue_id, orig_cue_id);
                assert_eq!(position, 20.0);
                assert_eq!(duration, 50.0);
            } else {
                panic!("Wrong Playback Event emitted.");
            }
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn pause_event() {
        let orig_cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) = setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::ExecuteCue(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx.send(EngineEvent::Audio(AudioEngineEvent::Paused { instance_id, position: 24.0, duration: 50.0 })).await.unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Paused {cue_id, position, duration } = event {
                assert_eq!(cue_id, orig_cue_id);
                assert_eq!(position, 24.0);
                assert_eq!(duration, 50.0);
            } else {
                panic!("Wrong Playback Event emitted.");
            }
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn resume_event() {
        let orig_cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) = setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::ExecuteCue(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx.send(EngineEvent::Audio(AudioEngineEvent::Resumed { instance_id })).await.unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Resumed {cue_id} = event {
                assert_eq!(cue_id, orig_cue_id);
            } else {
                panic!("Wrong Playback Event emitted.");
            }
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn completed_event() {
        let orig_cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) = setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::ExecuteCue(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx.send(EngineEvent::Audio(AudioEngineEvent::Completed { instance_id })).await.unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Completed {cue_id } = event {
                assert_eq!(cue_id, orig_cue_id);
            } else {
                panic!("Wrong Playback Event emitted.");
            }
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn error_event() {
        let orig_cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) = setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::ExecuteCue(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx.send(EngineEvent::Audio(AudioEngineEvent::Error { instance_id, error: "Error".to_string() })).await.unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Error {cue_id, error } = event {
                assert_eq!(cue_id, orig_cue_id);
                assert_eq!(error, "Error".to_string());
            } else {
                panic!("Wrong Playback Event emitted.");
            }
        } else {
            unreachable!();
        }
    }
}
