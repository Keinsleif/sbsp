use anyhow::{Context, Result};
use kira::{
    sound::{
        static_sound::{StaticSoundData, StaticSoundHandle}, EndPosition, PlaybackPosition, Region
    }, AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Easing, StartTime, Tween
};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tokio::{sync::mpsc, time};
use uuid::Uuid;

use crate::{
    executor::EngineEvent,
    model::cue::AudioCueLevels,
};

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Play {
        id: Uuid,
        filepath: PathBuf,
        levels: AudioCueLevels,
        start_time: Option<f64>,
        end_time: Option<f64>,
    },
    Pause {
        id: Uuid,
    },
    Resume {
        id: Uuid,
    },
    Stop {
        id: Uuid,
        fade_out: Duration,
    },
    SetLevels {
        id: Uuid,
        levels: AudioCueLevels,
        duration: f64,
        easing: Easing,
    },
}

struct PlayingSound {
    duration: f64,
    handle: StaticSoundHandle,
}

pub struct AudioEngine {
    manager: Option<AudioManager>,
    command_rx: mpsc::Receiver<AudioCommand>,
    event_tx: mpsc::Sender<EngineEvent>,
    playing_sounds: HashMap<Uuid, PlayingSound>,
}

impl AudioEngine {
    pub fn new(
        command_rx: mpsc::Receiver<AudioCommand>,
        event_tx: mpsc::Sender<EngineEvent>,
    ) -> Result<Self> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .context("Failed to initialize AudioManager")?;

        Ok(Self {
            manager: Some(manager),
            command_rx,
            event_tx,
            playing_sounds: HashMap::new(),
        })
    }

    pub async fn run(mut self) {
        let mut poll_timer = time::interval(Duration::from_millis(50));
        log::info!("AudioEngine run loop started");
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    log::debug!("AudioEngine received command: {:?}", command);

                    let result = match command {
                        // TODO: output is ignored. AudioEngine should have AudioManager for enabled devices
                        AudioCommand::Play {
                            id,
                            filepath,
                            levels,
                            start_time,
                            end_time,
                        } => {
                            self.handle_play(id, filepath, levels, start_time, end_time)
                                .await
                        }
                        AudioCommand::Pause { id } => self.handle_pause(id).await,
                        AudioCommand::Resume { id } => self.handle_resume(id).await,
                        AudioCommand::Stop { id, fade_out } => self.handle_stop(id, fade_out),
                        AudioCommand::SetLevels {id,levels, duration, easing } => self.handle_set_levels(id, levels, duration, easing),
                    };
                    if let Err(e) = result {
                        log::error!("Error processing audio_engine command: {:?}", e);
                    }
                },
                _ = poll_timer.tick() => {
                    let keys = self.playing_sounds.keys().clone();
                    for id in keys {
                        let Some(playing_sound) = self.playing_sounds.get(id) else {
                            log::warn!("Received event for unknown instance_id: {}", id);
                            continue;
                        };
                        let event = match playing_sound.handle.state() {
                            kira::sound::PlaybackState::Playing => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.duration })
                            },
                            kira::sound::PlaybackState::Pausing => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.duration })
                            },
                            kira::sound::PlaybackState::Paused => {
                                EngineEvent::Audio(AudioEngineEvent::Paused { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.duration })
                            },
                            kira::sound::PlaybackState::WaitingToResume => {
                                continue
                            },
                            kira::sound::PlaybackState::Resuming => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.duration })
                            },
                            kira::sound::PlaybackState::Stopping => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.duration })
                            },
                            kira::sound::PlaybackState::Stopped => {
                                EngineEvent::Audio(AudioEngineEvent::Completed { instance_id: *id })
                            },
                        };
                        if let Err(e) = self.event_tx.send(event).await {
                            log::error!("Error polling Sound status: {:?}", e);
                        }
                    }
                    // 停止状態のPlayingSoundを削除
                    self.playing_sounds.retain(|_, value| !matches!(value.handle.state(), kira::sound::PlaybackState::Stopped));
                },
                else => break
            }
        }
        log::info!("AudioEngine run loop finished.");
    }

    async fn handle_play(
        &mut self,
        id: Uuid,
        filepath: PathBuf,
        levels: AudioCueLevels,
        start_time_param: Option<f64>,
        end_time_param: Option<f64>,
    ) -> Result<()> {
        log::info!("PLAY: id={}, file={}", id, filepath.display());

        let manager = self.manager.as_mut().unwrap();

        let filepath_clone = filepath.clone();
        let sound_data =
            tokio::task::spawn_blocking(move || StaticSoundData::from_file(filepath_clone))
                .await?
                .with_context(|| format!("Failed to load sound data from: {}", filepath.display()))?
                .slice(Region {
                    start: PlaybackPosition::Seconds(start_time_param.unwrap_or(0.0)),
                    end: if let Some(end_time) = end_time_param {
                        EndPosition::Custom(PlaybackPosition::Seconds(end_time))
                    } else {
                        EndPosition::EndOfAudio
                    },
                })
                .volume(Decibels::from(levels.master as f32));

        let duration = sound_data.duration().as_secs_f64();

        let handle = manager.play(sound_data)?;

        self.event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Started {
                instance_id: id,
            }))
            .await?;

        self.playing_sounds.insert(
            id,
            PlayingSound {
                duration,
                handle,
            },
        );
        Ok(())
    }

    async fn handle_pause(&mut self, id: Uuid) -> Result<()> {
        log::info!("PAUSE: id={}", id);
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.pause(Tween::default());
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Paused {
                    instance_id: id,
                    position: playing_sound.handle.position(),
                    duration: playing_sound.duration,
                }))
                .await?;
            Ok(())
        } else {
            log::warn!("Pause command received for non-existent ID: {}", id);
            Err(anyhow::anyhow!("Sound with ID {} not found for pause.", id))
        }
    }

    async fn handle_resume(&mut self, id: Uuid) -> Result<()> {
        log::info!("RESUME: id={}", id);
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            if playing_sound
                .handle
                .state()
                .eq(&kira::sound::PlaybackState::Paused)
            {
                playing_sound.handle.resume(Tween::default());
                self.event_tx
                    .send(EngineEvent::Audio(AudioEngineEvent::Resumed {
                        instance_id: id,
                    }))
                    .await?;
            }
            Ok(())
        } else {
            log::warn!("Resume command received for non-existent ID: {}", id);
            Err(anyhow::anyhow!(
                "Sound with ID {} not found for resume.",
                id
            ))
        }
    }

    fn handle_stop(&mut self, id: Uuid, fade_out: Duration) -> Result<()> {
        log::info!("STOP: id={}, fade_out={:?}", id, fade_out);
        if let Some(mut playing_sound) = self.playing_sounds.remove(&id) {
            let fade_tween = Tween {
                start_time: StartTime::Immediate,
                duration: fade_out,
                easing: Easing::default(),
            };
            playing_sound.handle.stop(fade_tween);
            Ok(())
        } else {
            log::warn!("Stop command received for non-existent ID: {}", id);
            Err(anyhow::anyhow!("Sound with ID {} not found for stop.", id))
        }
    }

    fn handle_set_levels(
        &mut self,
        id: Uuid,
        levels: AudioCueLevels,
        duration: f64,
        easing: Easing,
    ) -> Result<()> {
        log::info!("SET LEVELS: id={}, levels={:?}", id, levels);
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound
                .handle
                .set_volume(levels.master as f32, Tween{
                    start_time: StartTime::Immediate,
                    duration: Duration::from_secs_f64(duration),
                    easing,
                });
            Ok(())
        } else {
            log::warn!("SetLevels command received for non-existent ID: {}", id);
            Err(anyhow::anyhow!(
                "Sound with ID {} not found for set levels.",
                id
            ))
        }
    }
}

#[derive(Debug)]
pub enum AudioEngineEvent {
    Started {
        instance_id: Uuid,
    },
    Progress {
        instance_id: Uuid,
        position: f64,
        duration: f64,
    },
    Paused {
        instance_id: Uuid,
        position: f64,
        duration: f64,
    },
    Resumed {
        instance_id: Uuid,
    },
    Completed {
        instance_id: Uuid,
    },
    Error {
        instance_id: Uuid,
        error: String,
    },
}

impl AudioEngineEvent {
    pub fn instance_id(&self) -> Uuid {
        match self {
            Self::Started { instance_id } => *instance_id,
            Self::Progress { instance_id, .. } => *instance_id,
            Self::Paused { instance_id, .. } => *instance_id,
            Self::Resumed { instance_id } => *instance_id,
            Self::Completed { instance_id } => *instance_id,
            Self::Error { instance_id, .. } => *instance_id,
        }
    }
}
