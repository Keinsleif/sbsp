mod command;
mod event;

pub use command::ExecutorCommand;
pub use event::ExecutorEvent;

use std::{collections::HashMap, sync::Arc};

use async_recursion::async_recursion;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    action::CueAction,
    controller::state::StateParam,
    engine::{
        EngineEvent, EngineType,
        audio_engine::{AudioCommand, AudioCommandData, AudioEngineEvent},
        wait_engine::{WaitCommand, WaitEvent, WaitType},
    },
    manager::ShowModelHandle,
    model::cue::{Cue, CueParam, audio::AudioCueParam},
};

#[derive(Debug)]
pub struct ActiveInstance {
    cue_id: Uuid,
    engine_type: EngineType,
    executed: bool,
}

pub struct Executor {
    model_handle: ShowModelHandle,
    command_rx: mpsc::Receiver<ExecutorCommand>,
    audio_tx: mpsc::Sender<AudioCommand>,
    wait_tx: mpsc::Sender<WaitCommand>,
    executor_event_tx: mpsc::Sender<ExecutorEvent>,
    engine_event_rx: mpsc::Receiver<EngineEvent>,

    active_instances: Arc<RwLock<HashMap<Uuid, ActiveInstance>>>,
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
            active_instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

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

    #[async_recursion]
    async fn process_command(&self, command: ExecutorCommand) -> Result<(), anyhow::Error> {
        match command {
            ExecutorCommand::Load(cue_id) => {
                if self.active_instances.read().await.iter().any(|(_, instance)| instance.cue_id == cue_id) {
                    log::warn!("Cue already loaded or executed. cue_id={}", cue_id);
                } else if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                    let instance_id = Uuid::now_v7();
                    self.load_cue(&cue, instance_id).await?;
                    self.active_instances.write().await.insert(
                        instance_id,
                        ActiveInstance {
                            cue_id,
                            engine_type: EngineType::Audio,
                            executed: false,
                        },
                    );
                }
            }
            ExecutorCommand::Execute(cue_id) => {
                if self.active_instances.read().await.iter().any(|(_, instance)| instance.cue_id == cue_id && instance.executed) {
                    log::warn!("Cue already executed. cue_id={}", cue_id);
                } else if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                    let mut instance_id = Uuid::now_v7();
                    let mut active_instances = self.active_instances.write().await;
                    if cue.pre_wait > 0.0 {
                        if let Some(loaded_instance) = active_instances
                            .iter_mut()
                            .find(|cue| cue.1.cue_id == cue_id)
                        {
                            log::debug!("EXECUTE: loaded cue found");
                            instance_id = *loaded_instance.0;
                            loaded_instance.1.engine_type = EngineType::PreWait;
                        } else {
                            active_instances.insert(
                                instance_id,
                                ActiveInstance {
                                    cue_id,
                                    engine_type: EngineType::PreWait,
                                    executed: true,
                                },
                            );
                            self.load_cue(&cue, instance_id).await?;
                        }
                        self.wait_tx
                            .send(WaitCommand::Start {
                                wait_type: WaitType::PreWait,
                                instance_id,
                                duration: cue.pre_wait,
                            })
                            .await?;
                    } else {
                        if let Some(loaded_instance) = active_instances
                            .iter_mut()
                            .find(|cue| cue.1.cue_id == cue_id)
                        {
                            log::debug!("EXECUTE: loaded cue found");
                            instance_id = *loaded_instance.0;
                        }
                        drop(active_instances);
                        self.execute_cue(&cue, instance_id).await?;
                    }
                } else {
                    anyhow::bail!("EXECUTE: cue not found. cue_id={}", cue_id);
                }
            }
            ExecutorCommand::Pause(cue_id) => self.pause_cue(cue_id).await?,
            ExecutorCommand::Resume(cue_id) => self.resume_cue(cue_id).await?,
            ExecutorCommand::Stop(cue_id) => self.stop_cue(cue_id).await?,
            ExecutorCommand::SeekTo(cue_id, position) => self.seek_to_cue(cue_id, position).await?,
            ExecutorCommand::SeekBy(cue_id, amount) => self.seek_by_cue(cue_id, amount).await?,
            ExecutorCommand::PerformAction(cue_id, action) => {
                let active_instances = self.active_instances.read().await;
                if let Some((instance_id, active_instance)) =
                    active_instances.iter().find(|map| map.1.cue_id == cue_id)
                {
                    match (action, active_instance.engine_type) {
                        (CueAction::Audio(audio_action), EngineType::Audio) => {
                            self.audio_tx
                                .send(AudioCommand::PerformAction {
                                    id: *instance_id,
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

    async fn load_cue(&self, cue: &Cue, instance_id: Uuid) -> Result<(), anyhow::Error> {
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
            }) => {
                let filepath = if let Some(model_path) = self
                    .model_handle
                    .get_current_file_path()
                    .await && let Some(parent) = model_path.parent() {
                        parent.join(target)
                    } else {
                        target.clone()
                    };

                self.audio_tx
                    .send(AudioCommand::Load {
                        id: instance_id,
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
                        },
                    })
                    .await?;
            }
            CueParam::Wait { duration } => {
                self.wait_tx
                    .send(WaitCommand::Load {
                        wait_type: WaitType::Wait,
                        instance_id,
                        duration: *duration,
                    })
                    .await?;
            }
            CueParam::Fade { fade_param, .. } => {
                self.wait_tx
                    .send(WaitCommand::Load {
                        wait_type: WaitType::FadeWait,
                        instance_id,
                        duration: fade_param.duration,
                    })
                    .await?;
            }
            CueParam::Start { .. } |
            CueParam::Stop { .. } |
            CueParam::Pause { .. } |
            CueParam::Load { .. } => {
                log::warn!("Loading transport cues is not available. ignoring...");
            }
        }
        Ok(())
    }

    async fn execute_cue(&self, cue: &Cue, instance_id: Uuid) -> Result<(), anyhow::Error> {
        let engine_type = match &cue.params {
            CueParam::Audio(..) => EngineType::Audio,
            CueParam::Wait { .. } => EngineType::Wait,
            CueParam::Fade { .. } => EngineType::Fade,
            CueParam::Start { .. } |
            CueParam::Stop { .. } |
            CueParam::Pause { .. } |
            CueParam::Load { .. } => EngineType::Playback,
        };
        if engine_type != EngineType::Playback {
            if let Some(active_instance) =
                self.active_instances.write().await.get_mut(&instance_id)
            {
                active_instance.engine_type = engine_type;
            } else {
                self.active_instances.write().await.insert(
                    instance_id,
                    ActiveInstance {
                        cue_id: cue.id,
                        engine_type,
                        executed: true,
                    },
                );
            }
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
            }) => {
                let filepath = if let Some(model_path) = self
                    .model_handle
                    .get_current_file_path()
                    .await && let Some(parent) = model_path.parent() {
                        parent.join(target)
                    } else {
                        target.clone()
                    };

                let audio_command = AudioCommand::Play {
                    id: instance_id,
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
                    },
                };
                self.audio_tx.send(audio_command).await?;
            }
            CueParam::Wait { duration } => {
                self.wait_tx
                    .send(WaitCommand::Start {
                        wait_type: WaitType::Wait,
                        instance_id,
                        duration: *duration,
                    })
                    .await?;
            }
            CueParam::Fade { target, volume, fade_param } => {
                if let Some((target_id, _)) = self.active_instances.read().await.iter().find(|(_, instance)| instance.cue_id == *target && instance.engine_type == EngineType::Audio) 
                && self.audio_tx.send(AudioCommand::SetVolume { id: *target_id, volume: *volume, fade_param: *fade_param }).await.is_err() {
                    anyhow::bail!("cannot send AudioCommand");
                }

                self.wait_tx
                    .send(WaitCommand::Start {
                        wait_type: WaitType::FadeWait,
                        instance_id,
                        duration: fade_param.duration,
                    })
                    .await?;
            }
            CueParam::Start { target } => {
                if self.active_instances.read().await.iter().any(|(_, instance)| instance.cue_id == *target && instance.executed) {
                    self.process_command(ExecutorCommand::Resume(*target)).await?;
                } else {
                    self.process_command(ExecutorCommand::Execute(*target)).await?;
                }
            }
            CueParam::Stop { target } => {
                self.process_command(ExecutorCommand::Stop(*target)).await?;
            }
            CueParam::Pause { target } => {
                self.process_command(ExecutorCommand::Pause(*target)).await?;
            }
            CueParam::Load { target } => {
                self.process_command(ExecutorCommand::Load(*target)).await?;
            }
        }
        Ok(())
    }

    async fn pause_cue(&self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        let active_instances = self.active_instances.read().await;
        if let Some((instance_id, active_instance)) =
            active_instances.iter().find(|map| map.1.cue_id == cue_id)
        {
            match active_instance.engine_type {
                EngineType::PreWait => {
                    self.wait_tx
                        .send(WaitCommand::Pause {
                            instance_id: *instance_id,
                        })
                        .await?;
                }
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::Pause { id: *instance_id })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::Pause {
                            instance_id: *instance_id,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("Pause command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("Pause command is not available for Transport cues. ignoring...");
                }
            }
        }
        Ok(())
    }

    async fn resume_cue(&self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        let active_instances = self.active_instances.read().await;
        if let Some((instance_id, active_instance)) =
            active_instances.iter().find(|map| map.1.cue_id == cue_id)
        {
            match active_instance.engine_type {
                EngineType::PreWait => {
                    self.wait_tx
                        .send(WaitCommand::Resume {
                            instance_id: *instance_id,
                        })
                        .await?;
                }
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::Resume { id: *instance_id })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::Resume {
                            instance_id: *instance_id,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("Resume command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("Resume command is not available for Transport cues. ignoring...");
                }
            }
        }
        Ok(())
    }

    async fn stop_cue(&self, cue_id: Uuid) -> Result<(), anyhow::Error> {
        let active_instances = self.active_instances.read().await;
        if let Some((instance_id, active_instance)) =
            active_instances.iter().find(|map| map.1.cue_id == cue_id)
        {
            match active_instance.engine_type {
                EngineType::PreWait => {
                    self.wait_tx
                        .send(WaitCommand::Stop {
                            instance_id: *instance_id,
                        })
                        .await?;
                    self.executor_event_tx
                        .send(ExecutorEvent::PreWaitStopped { cue_id })
                        .await?;
                }
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::Stop { id: *instance_id })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::Stop {
                            instance_id: *instance_id,
                        })
                        .await?;
                }
                EngineType::Fade => {
                    log::warn!("Stop command is not available for Fade cue. ignoring...");
                }
                EngineType::Playback => {
                    log::warn!("Stop command is not available for Transport cues. ignoring...");
                }
            }
        }
        Ok(())
    }

    async fn seek_to_cue(&self, cue_id: Uuid, position: f64) -> Result<(), anyhow::Error> {
        let active_instances = self.active_instances.read().await;
        if let Some((instance_id, active_instance)) =
            active_instances.iter().find(|map| map.1.cue_id == cue_id)
        {
            match active_instance.engine_type {
                EngineType::PreWait => {
                    self.wait_tx
                        .send(WaitCommand::SeekTo {
                            instance_id: *instance_id,
                            position,
                        })
                        .await?;
                }
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::SeekTo {
                            id: *instance_id,
                            position,
                        })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::SeekTo {
                            instance_id: *instance_id,
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
            }
        }
        Ok(())
    }

    async fn seek_by_cue(&self, cue_id: Uuid, amount: f64) -> Result<(), anyhow::Error> {
        let active_instances = self.active_instances.read().await;
        if let Some((instance_id, active_instance)) =
            active_instances.iter().find(|map| map.1.cue_id == cue_id)
        {
            match active_instance.engine_type {
                EngineType::PreWait => {
                    self.wait_tx
                        .send(WaitCommand::SeekBy {
                            instance_id: *instance_id,
                            amount,
                        })
                        .await?;
                }
                EngineType::Audio => {
                    self.audio_tx
                        .send(AudioCommand::SeekBy {
                            id: *instance_id,
                            amount,
                        })
                        .await?;
                }
                EngineType::Wait => {
                    self.wait_tx
                        .send(WaitCommand::SeekBy {
                            instance_id: *instance_id,
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
            }
        }
        Ok(())
    }

    async fn handle_engine_event(&self, event: EngineEvent) -> Result<(), anyhow::Error> {
        match event {
            EngineEvent::Audio(audio_event) => {
                let instance_id = audio_event.instance_id();

                let cue_id = {
                    let instances = self.active_instances.read().await;
                    let Some(instance) = instances.get(&instance_id) else {
                        anyhow::bail!("unknown instance_id id={}", instance_id);
                    };
                    instance.cue_id
                };

                let playback_event = match audio_event {
                    AudioEngineEvent::Loaded {
                        position, duration, ..
                    } => ExecutorEvent::Loaded {
                        cue_id,
                        position,
                        duration,
                    },
                    AudioEngineEvent::Started { initial_params, .. } => ExecutorEvent::Started {
                        cue_id,
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
                    } => ExecutorEvent::Paused {
                        cue_id,
                        position,
                        duration,
                    },
                    AudioEngineEvent::Resumed { .. } => ExecutorEvent::Resumed { cue_id },
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
                        self.active_instances.write().await.remove(&instance_id);
                        ExecutorEvent::Stopped { cue_id }
                    }
                    AudioEngineEvent::Completed { .. } => {
                        self.active_instances.write().await.remove(&instance_id);
                        ExecutorEvent::Completed { cue_id }
                    }
                    AudioEngineEvent::StateParamUpdated { params, .. } => {
                        ExecutorEvent::StateParamUpdated {
                            cue_id,
                            params: StateParam::Audio(params),
                        }
                    }
                    AudioEngineEvent::Error { error, .. } => {
                        self.active_instances.write().await.remove(&instance_id);
                        ExecutorEvent::Error { cue_id, error }
                    }
                };

                self.executor_event_tx.send(playback_event).await?;
            }
            EngineEvent::PreWait(wait_event) => {
                let instance_id = wait_event.instance_id();
                let cue_id = {
                    let instances = self.active_instances.read().await;
                    let Some(instance) = instances.get(&instance_id) else {
                        anyhow::bail!("unknown instance_id id={}", instance_id);
                    };
                    instance.cue_id
                };

                let executor_event = match wait_event {
                    WaitEvent::Loaded { .. } => unreachable!(),
                    WaitEvent::Started { .. } => ExecutorEvent::PreWaitStarted { cue_id },
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
                    } => ExecutorEvent::PreWaitPaused {
                        cue_id,
                        position,
                        duration,
                    },
                    WaitEvent::Resumed { .. } => ExecutorEvent::PreWaitResumed { cue_id },
                    WaitEvent::Stopped { .. } => {
                        if self
                            .active_instances
                            .write()
                            .await
                            .remove(&cue_id)
                            .is_some()
                        {
                            log::info!("PreWaitStopped cue_id={}", cue_id);
                        }
                        ExecutorEvent::PreWaitStopped { cue_id }
                    }
                    WaitEvent::Completed { instance_id } => {
                        if let Some(cue) = self.model_handle.get_cue_by_id(&cue_id).await {
                            log::info!("PreWaitCompleted cue_id={}", cue.id);
                            self.execute_cue(&cue, instance_id).await?;
                        } else {
                            anyhow::bail!("PreWait: cue to execute not found. id={}", cue_id);
                        }
                        ExecutorEvent::PreWaitCompleted { cue_id }
                    }
                };

                self.executor_event_tx.send(executor_event).await?;
            }
            EngineEvent::Wait(wait_event) |
            EngineEvent::Fade(wait_event) => {
                let instance_id = wait_event.instance_id();

                let cue_id = {
                    let instances = self.active_instances.read().await;
                    let Some(instance) = instances.get(&instance_id) else {
                        anyhow::bail!("unknown instance_id. id={}", instance_id);
                    };
                    instance.cue_id
                };

                let playback_event = match wait_event {
                    WaitEvent::Loaded {
                        position, duration, ..
                    } => ExecutorEvent::Loaded {
                        cue_id,
                        position,
                        duration,
                    },
                    WaitEvent::Started { .. } => ExecutorEvent::Started {
                        cue_id,
                        initial_params: StateParam::Wait,
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
                    } => ExecutorEvent::Paused {
                        cue_id,
                        position,
                        duration,
                    },
                    WaitEvent::Resumed { .. } => ExecutorEvent::Resumed { cue_id },
                    WaitEvent::Stopped { .. } => {
                        self.active_instances.write().await.remove(&instance_id);
                        ExecutorEvent::Stopped { cue_id }
                    }
                    WaitEvent::Completed { .. } => {
                        self.active_instances.write().await.remove(&instance_id);
                        ExecutorEvent::Completed { cue_id }
                    }
                };

                self.executor_event_tx.send(playback_event).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use tokio::sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
        watch,
    };
    use uuid::Uuid;

    use crate::{
        BackendSettings,
        controller::state::AudioStateParam,
        engine::audio_engine::{AudioCommand, AudioEngineEvent},
        event::UiEvent,
        manager::ShowModelManager,
        model::{
            self,
            cue::audio::{FadeParam, Easing, SoundType},
        },
    };

    async fn setup_executor(
        cue_id: Uuid,
    ) -> (
        ShowModelManager,
        Sender<ExecutorCommand>,
        Receiver<AudioCommand>,
        Sender<EngineEvent>,
        Receiver<ExecutorEvent>,
    ) {
        let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
        let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
        let (wait_tx, _wait_rx) = mpsc::channel::<WaitCommand>(32);
        let (playback_event_tx, playback_event_rx) = mpsc::channel::<ExecutorEvent>(32);
        let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);
        let (event_tx, _) = broadcast::channel::<UiEvent>(32);
        let (_, settings_rx) = watch::channel(BackendSettings::default());

        let (manager, handle) = ShowModelManager::new(event_tx.clone(), settings_rx);
        let mut write_lock = manager.write().await;
        write_lock.name = "TestShowModel".to_string();
        write_lock.cues.push(Cue {
            id: cue_id,
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
        drop(write_lock);

        let executor = Executor::new(
            handle.clone(),
            exec_rx,
            audio_tx,
            wait_tx,
            playback_event_tx,
            engine_event_rx,
        );

        tokio::spawn(executor.run());

        (
            manager,
            exec_tx,
            audio_rx,
            engine_event_tx,
            playback_event_rx,
        )
    }

    #[tokio::test]
    async fn play_command() {
        let cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, _, _) = setup_executor(cue_id).await;

        let old_id = Uuid::now_v7();

        exec_tx
            .send(ExecutorCommand::Execute(cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        if let AudioCommand::Play { id, data } = command {
            assert!(id > old_id);
            let now_id = Uuid::now_v7();
            assert!(id < now_id);
            assert_eq!(data.filepath, PathBuf::from("./I.G.Y.flac"));
            assert_eq!(data.volume, 0.0);
            assert_eq!(data.pan, 0.0);
            assert_eq!(data.start_time, Some(5.0));
            assert_eq!(
                data.fade_in_param,
                Some(FadeParam {
                    duration: 2.0,
                    easing: Easing::Linear
                })
            );
            assert_eq!(data.end_time, Some(50.0));
            assert_eq!(
                data.fade_out_param,
                Some(FadeParam {
                    duration: 5.0,
                    easing: Easing::InPowi(2)
                })
            );
            assert!(!data.repeat);
        } else {
            unreachable!();
        }
    }

    #[tokio::test]
    async fn started_event() {
        let orig_cue_id = Uuid::new_v4();

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
            setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::Execute(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Started {
                instance_id,
                initial_params: AudioStateParam::default(),
            }))
            .await
            .unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Started {
                cue_id,
                initial_params,
            } = event
            {
                assert_eq!(cue_id, orig_cue_id);
                assert_eq!(
                    initial_params,
                    StateParam::Audio(AudioStateParam::default())
                );
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

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
            setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::Execute(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Progress {
                instance_id,
                position: 20.0,
                duration: 50.0,
            }))
            .await
            .unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Progress {
                cue_id,
                position,
                duration,
            } = event
            {
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

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
            setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::Execute(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Paused {
                instance_id,
                position: 24.0,
                duration: 50.0,
            }))
            .await
            .unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Paused {
                cue_id,
                position,
                duration,
            } = event
            {
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

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
            setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::Execute(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Resumed {
                instance_id,
            }))
            .await
            .unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Resumed { cue_id } = event {
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

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
            setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::Execute(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Completed {
                instance_id,
            }))
            .await
            .unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Completed { cue_id } = event {
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

        let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
            setup_executor(orig_cue_id).await;

        exec_tx
            .send(ExecutorCommand::Execute(orig_cue_id))
            .await
            .unwrap();

        let command = audio_rx.recv().await.unwrap();

        let instance_id = if let AudioCommand::Play { id, .. } = command {
            id
        } else {
            unreachable!();
        };

        engine_event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Error {
                instance_id,
                error: "Error".to_string(),
            }))
            .await
            .unwrap();

        if let Some(event) = playback_event_rx.recv().await {
            if let ExecutorEvent::Error { cue_id, error } = event {
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
