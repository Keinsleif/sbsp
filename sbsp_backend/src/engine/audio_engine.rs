use anyhow::{Context, Result};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Easing, StartTime, Tween,
    clock::{ClockHandle, ClockSpeed, ClockTime},
    sound::{
        EndPosition, PlaybackPosition, PlaybackState, Region,
        static_sound::{StaticSoundData, StaticSoundHandle},
    },
};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tokio::{sync::mpsc, time};
use uuid::Uuid;

use crate::{
    executor::EngineEvent,
    model::cue::{AudioCueFadeParam, AudioCueLevels, LoopRegion},
};

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Load {
        id: Uuid,
        data: AudioCommandData,
    },
    Play {
        id: Uuid,
        data: AudioCommandData,
    },
    Pause {
        id: Uuid,
    },
    Resume {
        id: Uuid,
    },
    Stop {
        id: Uuid,
    },
    SetLevels {
        id: Uuid,
        levels: AudioCueLevels,
        duration: f64,
        easing: Easing,
    },
}

impl AudioCommand {
    fn id(&self) -> Uuid {
        match self {
            AudioCommand::Load { id, .. } => *id,
            AudioCommand::Play { id, .. } => *id,
            AudioCommand::Pause { id } => *id,
            AudioCommand::Resume { id } => *id,
            AudioCommand::Stop { id } => *id,
            AudioCommand::SetLevels { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioCommandData {
    pub filepath: PathBuf,
    pub levels: AudioCueLevels,
    pub start_time: Option<f64>,
    pub fade_in_param: Option<AudioCueFadeParam>,
    pub end_time: Option<f64>,
    pub fade_out_param: Option<AudioCueFadeParam>,
    pub loop_region: Option<LoopRegion>,
}

struct SoundHandle {
    handle: StaticSoundHandle,
    clock: ClockHandle,
    duration: f64,
}

impl SoundHandle {
    fn state(&self) -> PlaybackState {
        self.handle.state()
    }

    fn position(&self) -> f64 {
        self.handle.position()
    }

    fn start(&mut self) {
        self.clock.start();
    }

    fn pause(&mut self) {
        self.handle.pause(Tween::default());
        self.clock.pause();
    }

    fn resume(&mut self) {
        self.handle.resume(Tween::default());
        self.clock.start();
    }

    fn stop(&mut self) {
        self.handle.stop(Tween::default());
        self.clock.start();
    }

    fn set_volume(&mut self, levels: AudioCueLevels, tween: Tween) {
        self.handle.set_volume(levels.master as f32, tween);
    }
}

struct PlayingSound {
    handle: SoundHandle,
    last_state: PlaybackState,
}

pub struct AudioEngine {
    manager: Option<AudioManager>,
    command_rx: mpsc::Receiver<AudioCommand>,
    event_tx: mpsc::Sender<EngineEvent>,
    playing_sounds: HashMap<Uuid, PlayingSound>,
    loaded_sounds: HashMap<Uuid, SoundHandle>,
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
            loaded_sounds: HashMap::new(),
        })
    }

    pub async fn run(mut self) {
        let mut poll_timer = time::interval(Duration::from_millis(50));
        log::info!("AudioEngine run loop started");
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    log::debug!("AudioEngine received command: {:?}", command);
                    let instance_id = command.id();

                    let result = match command {
                        AudioCommand::Load { id, data } => {
                            self.handle_load(id, data).await
                        }
                        AudioCommand::Play {id, data} => {
                            self.handle_play(id, data)
                                .await
                        }
                        AudioCommand::Pause { id } => self.handle_pause(id).await,
                        AudioCommand::Resume { id } => self.handle_resume(id).await,
                        AudioCommand::Stop { id } => self.handle_stop(id),
                        AudioCommand::SetLevels {id,levels, duration, easing } => self.handle_set_levels(id, levels, duration, easing),
                    };
                    if let Err(e) = result {
                        self.event_tx.send(EngineEvent::Audio(AudioEngineEvent::Error { instance_id, error: format!("{}",e) })).await.unwrap();
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
                        let playback_state = playing_sound.handle.state();
                        let event = match playback_state {
                            kira::sound::PlaybackState::Playing => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::Pausing => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::Paused => {
                                if playing_sound.last_state.eq(&PlaybackState::Paused) {
                                    continue;
                                }
                                log::info!("PAUSE: id={}", *id);
                                EngineEvent::Audio(AudioEngineEvent::Paused { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::WaitingToResume => {
                                continue
                            },
                            kira::sound::PlaybackState::Resuming => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::Stopping => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position: playing_sound.handle.position(), duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::Stopped => {
                                if playing_sound.last_state.eq(&PlaybackState::Stopped) {
                                    continue;
                                }
                                log::info!("STOP: id={}", *id);
                                EngineEvent::Audio(AudioEngineEvent::Completed { instance_id: *id })
                            },
                        };
                        if let Err(e) = self.event_tx.send(event).await {
                            log::error!("Error polling Sound status: {:?}", e);
                        }
                    }
                    for playing_sound in self.playing_sounds.values_mut() {
                        playing_sound.last_state = playing_sound.handle.state();
                    }
                    // 停止状態のPlayingSoundを削除
                    self.playing_sounds.retain(|_, value| !matches!(value.handle.state(), kira::sound::PlaybackState::Stopped));
                },
                else => break
            }
        }
        log::info!("AudioEngine run loop finished.");
    }

    async fn handle_load(&mut self, id: Uuid, data: AudioCommandData) -> Result<()> {
        let manager = self.manager.as_mut().unwrap();
        let clock = manager.add_clock(ClockSpeed::SecondsPerTick(1.0)).unwrap();

        let filepath_clone = data.filepath.clone();
        let mut sound_data =
            tokio::task::spawn_blocking(move || StaticSoundData::from_file(filepath_clone))
                .await?
                .with_context(|| {
                    format!(
                        "Failed to load sound data from: {}",
                        data.filepath.display()
                    )
                })?
                .slice(Region {
                    start: PlaybackPosition::Seconds(data.start_time.unwrap_or(0.0)),
                    end: if let Some(end_time) = data.end_time {
                        EndPosition::Custom(PlaybackPosition::Seconds(end_time))
                    } else {
                        EndPosition::EndOfAudio
                    },
                })
                .volume(Decibels::from(data.levels.master as f32))
                .start_time(StartTime::ClockTime(ClockTime::from_ticks_f64(&clock, 0.0)));

        if let Some(region) = data.loop_region {
            sound_data = sound_data.loop_region(region);
        }

        if let Some(fade_in_param) = data.fade_in_param {
            sound_data = sound_data.fade_in_tween(Tween {
                start_time: StartTime::Immediate,
                duration: Duration::from_secs_f64(fade_in_param.duration),
                easing: fade_in_param.easing.into(),
            });
        }

        let duration = sound_data.duration().as_secs_f64();

        log::info!("LOAD: id={}, file={}", id, data.filepath.display());
        let mut handle = manager.play(sound_data)?;

        if let Some(fade_out_param) = data.fade_out_param {
            handle.set_volume(
                Decibels::SILENCE,
                Tween {
                    start_time: StartTime::ClockTime(ClockTime::from_ticks_f64(
                        &clock,
                        duration - fade_out_param.duration,
                    )),
                    duration: Duration::from_secs_f64(fade_out_param.duration),
                    easing: fade_out_param.easing.into(),
                },
            );
        }

        self.event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Loaded {
                instance_id: id,
            }))
            .await?;

        self.loaded_sounds.insert(
            id,
            SoundHandle {
                handle,
                clock,
                duration,
            },
        );
        Ok(())
    }

    async fn handle_play(&mut self, id: Uuid, data: AudioCommandData) -> Result<()> {
        if !self.loaded_sounds.contains_key(&id) {
            self.handle_load(id, data.clone()).await?;
        }

        let mut handle = self.loaded_sounds.remove(&id).unwrap();
        handle.start();

        log::info!("PLAY: id={}, file={}", id, data.filepath.display());

        self.event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Started {
                instance_id: id,
            }))
            .await?;

        self.playing_sounds.insert(
            id,
            PlayingSound {
                handle,
                last_state: PlaybackState::Playing,
            },
        );
        Ok(())
    }

    async fn handle_pause(&mut self, id: Uuid) -> Result<()> {
        log::info!("PAUSE: id={}", id);
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.pause();
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Paused {
                    instance_id: id,
                    position: playing_sound.handle.position(),
                    duration: playing_sound.handle.duration,
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
                playing_sound.handle.resume();
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

    fn handle_stop(&mut self, id: Uuid) -> Result<()> {
        log::info!("STOP: id={}", id);
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.stop();
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
            playing_sound.handle.set_volume(
                levels,
                Tween {
                    start_time: StartTime::Immediate,
                    duration: Duration::from_secs_f64(duration),
                    easing,
                },
            );
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
    Loaded {
        instance_id: Uuid,
    },
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
            Self::Loaded { instance_id } => *instance_id,
            Self::Started { instance_id } => *instance_id,
            Self::Progress { instance_id, .. } => *instance_id,
            Self::Paused { instance_id, .. } => *instance_id,
            Self::Resumed { instance_id } => *instance_id,
            Self::Completed { instance_id } => *instance_id,
            Self::Error { instance_id, .. } => *instance_id,
        }
    }
}
