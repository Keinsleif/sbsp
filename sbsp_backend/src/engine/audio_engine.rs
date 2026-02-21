mod command;
mod event;
pub mod level_meter;
mod mono_effect;

pub use command::{AudioCommand, AudioCommandData};
pub use event::AudioEngineEvent;

use anyhow::{Context, Result};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Panning, StartTime, Tween,
    clock::{ClockHandle, ClockSpeed, ClockTime},
    sound::{
        EndPosition, FromFileError, IntoOptionalRegion, PlaybackPosition, PlaybackState, Region,
        static_sound::{StaticSoundData, StaticSoundHandle},
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
};
use std::{collections::HashMap, time::Duration};
use tokio::{sync::mpsc, time};
use uuid::Uuid;

use super::EngineEvent;
use crate::{
    action::AudioAction,
    controller::state::AudioStateParam,
    engine::audio_engine::level_meter::{LevelMeterEffect, SharedLevel},
    model::{
        cue::audio::{FadeParam, SoundType},
        settings::ShowAudioSettings,
    },
};
use mono_effect::{MonoEffectBuilder, MonoEffectHandle};

enum SoundHandle {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<FromFileError>),
}

impl SoundHandle {
    fn state(&self) -> PlaybackState {
        match self {
            Self::Static(handle) => handle.state(),
            Self::Streaming(handle) => handle.state(),
        }
    }

    fn position(&self) -> f64 {
        match self {
            Self::Static(handle) => handle.position(),
            Self::Streaming(handle) => handle.position(),
        }
    }

    fn pause(&mut self, tween: Tween) {
        match self {
            Self::Static(handle) => handle.pause(tween),
            Self::Streaming(handle) => handle.pause(tween),
        }
    }

    fn resume(&mut self, tween: Tween) {
        match self {
            Self::Static(handle) => handle.resume(tween),
            Self::Streaming(handle) => handle.resume(tween),
        }
    }

    fn stop(&mut self, tween: Tween) {
        match self {
            Self::Static(handle) => handle.stop(tween),
            Self::Streaming(handle) => handle.stop(tween),
        }
    }

    fn seek_to(&mut self, position: f64) {
        match self {
            Self::Static(handle) => handle.seek_to(position),
            Self::Streaming(handle) => handle.seek_to(position),
        }
    }

    fn seek_by(&mut self, amount: f64) {
        match self {
            Self::Static(handle) => handle.seek_by(amount),
            Self::Streaming(handle) => handle.seek_by(amount),
        }
    }

    fn set_loop_region(&mut self, loop_region: impl IntoOptionalRegion) {
        match self {
            Self::Static(handle) => handle.set_loop_region(loop_region),
            Self::Streaming(handle) => handle.set_loop_region(loop_region),
        }
    }

    fn set_volume(&mut self, volume: f32, tween: Tween) {
        match self {
            Self::Static(handle) => handle.set_volume(volume, tween),
            Self::Streaming(handle) => handle.set_volume(volume, tween),
        }
    }
}

struct PlaybackHandle {
    handle: SoundHandle,
    clock: ClockHandle,
    duration: f64,
    fade_out_tween: Option<Tween>,
    repeat: bool,
    volume: f32,
    fade_volume: f32,
}

impl PlaybackHandle {
    fn state(&self) -> PlaybackState {
        self.handle.state()
    }

    fn is_repeating(&self) -> bool {
        self.repeat
    }

    fn get_volume(&self) -> f32 {
        self.volume
    }

    fn position(&self) -> f64 {
        self.handle.position()
    }

    fn start(&mut self) {
        self.clock.start();
    }

    fn pause(&mut self) {
        self.handle.pause(Tween::default());
    }

    fn resume(&mut self) {
        self.handle.resume(Tween::default());
    }

    fn stop(&mut self) {
        if self.state().eq(&PlaybackState::Stopping) {
            // Hard Stop
            self.handle.stop(Tween::default());
        } else {
            self.handle.stop(self.fade_out_tween.unwrap_or_default());
        }
    }

    fn seek_to(&mut self, position: f64) {
        self.handle.seek_to(position);
    }

    fn seek_by(&mut self, amount: f64) {
        self.handle.seek_by(amount);
    }

    fn set_repeat(&mut self, repeat: bool) {
        if repeat {
            self.handle.set_loop_region(Some(Region {
                start: PlaybackPosition::Seconds(0.0),
                end: EndPosition::EndOfAudio,
            }));
        } else {
            self.handle.set_loop_region(None);
        }
        self.repeat = repeat;
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
        self.handle
            .set_volume(self.volume + self.fade_volume, Tween::default());
    }

    fn set_fade(&mut self, volume: f32, tween: Tween) {
        self.fade_volume = volume;
        self.handle
            .set_volume(self.volume + self.fade_volume, tween);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LastStatus {
    state: PlaybackState,
    position: f64,
    duration: f64,
}

struct PlayingSound {
    handle: PlaybackHandle,
    last_status: Option<LastStatus>,
    manual_stop_sent: bool,
}

pub struct AudioEngine {
    manager: Option<AudioManager>,
    mono_effect_handle: MonoEffectHandle,
    settings: ShowAudioSettings,
    command_rx: mpsc::Receiver<AudioCommand>,
    event_tx: mpsc::Sender<EngineEvent>,
    playing_sounds: HashMap<Uuid, PlayingSound>,
    loaded_sounds: HashMap<Uuid, PlaybackHandle>,
}

impl AudioEngine {
    pub fn new(
        command_rx: mpsc::Receiver<AudioCommand>,
        event_tx: mpsc::Sender<EngineEvent>,
        settings: ShowAudioSettings,
    ) -> Result<Self> {
        let mut audio_manager_settings = AudioManagerSettings::default();
        let mono_effect_handle = audio_manager_settings
            .main_track_builder
            .add_effect(MonoEffectBuilder::new(settings.mono_output));
        let manager = AudioManager::<DefaultBackend>::new(audio_manager_settings)?;

        Ok(Self {
            manager: Some(manager),
            mono_effect_handle,
            settings,
            command_rx,
            event_tx,
            playing_sounds: HashMap::new(),
            loaded_sounds: HashMap::new(),
        })
    }

    pub fn new_with_level_meter(
        command_rx: mpsc::Receiver<AudioCommand>,
        event_tx: mpsc::Sender<EngineEvent>,
        settings: ShowAudioSettings,
    ) -> Result<(Self, SharedLevel)> {
        let shared_level = SharedLevel::default();
        let mut audio_manager_settings = AudioManagerSettings::default();
        let mono_effect_handle = audio_manager_settings
            .main_track_builder
            .add_effect(MonoEffectBuilder::new(settings.mono_output));
        audio_manager_settings
            .main_track_builder
            .add_built_effect(Box::new(LevelMeterEffect::new(shared_level.clone())));
        let manager = AudioManager::<DefaultBackend>::new(audio_manager_settings)?;

        Ok((
            Self {
                manager: Some(manager),
                mono_effect_handle,
                settings,
                command_rx,
                event_tx,
                playing_sounds: HashMap::new(),
                loaded_sounds: HashMap::new(),
            },
            shared_level,
        ))
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
                        AudioCommand::SeekTo { id, position } => self.handle_seek_to(id, position).await,
                        AudioCommand::SeekBy { id, amount } => self.handle_seek_by(id, amount).await,
                        AudioCommand::FadeVolume { id, volume, fade_param } => self.handle_fade_volume(id, volume, fade_param).await,
                        AudioCommand::PerformAction { id, action } => self.handle_action(id, action).await,
                        AudioCommand::Reconfigure(settings) => {
                            if settings.mono_output != self.settings.mono_output {
                                self.mono_effect_handle.set_enable(settings.mono_output);
                            }
                            self.settings = settings.clone();
                            Ok(())
                        },
                    };
                    if let Err(e) = result {
                        if self.event_tx.send(EngineEvent::Audio(AudioEngineEvent::Error { instance_id, error: format!("{}",e) })).await.is_err() {
                            log::warn!("AudioEngineEvent bus dropped.");
                        }
                        log::error!("Error processing audio_engine command: {:?}", e);
                    }
                },
                _ = poll_timer.tick() => {
                    let keys = self.playing_sounds.keys().clone();
                    for id in keys {
                        let Some(playing_sound) = self.playing_sounds.get(id) else {
                            continue;
                        };
                        let playback_state = playing_sound.handle.state();
                        let position = playing_sound.handle.position();
                        let event = match playback_state {
                            kira::sound::PlaybackState::Playing => {
                                let event = EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position, duration: playing_sound.handle.duration });
                                if let Err(e) = self.event_tx.try_send(event) {
                                    log::warn!("EngineEvent dropped: {:?}", e);
                                }
                                continue;
                            },
                            kira::sound::PlaybackState::Pausing => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position, duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::Paused => {
                                if let Some(last_state) = &playing_sound.last_status
                                    && last_state.eq(&LastStatus {state: playback_state, position, duration: playing_sound.handle.duration }) {
                                        continue;
                                }
                                EngineEvent::Audio(AudioEngineEvent::Paused { instance_id: *id, position, duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::WaitingToResume => {
                                continue
                            },
                            kira::sound::PlaybackState::Resuming => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position, duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::Stopping => {
                                EngineEvent::Audio(AudioEngineEvent::Stopping { instance_id: *id, position, duration: playing_sound.handle.duration })
                            },
                            kira::sound::PlaybackState::Stopped => {
                                if playing_sound.manual_stop_sent {
                                    log::info!("STOP: id={}", *id);
                                    EngineEvent::Audio(AudioEngineEvent::Stopped { instance_id: *id })
                                } else {
                                    log::info!("COMPLETE: id={}", *id);
                                    EngineEvent::Audio(AudioEngineEvent::Completed { instance_id: *id })
                                }
                            },
                        };
                        if self.event_tx.send(event).await.is_err() {
                            log::warn!("AudioEngineEvent bus dropped.");
                        }
                    }
                    for playing_sound in self.playing_sounds.values_mut() {
                        playing_sound.last_status = Some(LastStatus{
                            state: playing_sound.handle.state(),
                            position: playing_sound.handle.position(),
                            duration: playing_sound.handle.duration,
                        });
                    }
                    self.playing_sounds.retain(|_, value| !matches!(value.handle.state(), kira::sound::PlaybackState::Stopped));
                },
                else => break
            }
        }
        log::info!("AudioEngine run loop finished.");
    }

    async fn handle_load(&mut self, id: Uuid, data: AudioCommandData) -> Result<()> {
        let manager = self
            .manager
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("AudioManager is not available"))?;
        let clock = manager
            .add_clock(ClockSpeed::SecondsPerTick(1.0))
            .map_err(|_| anyhow::anyhow!("Failed to create audio. Playback limit reached."))?;

        let filepath_clone = data.filepath.clone();

        let duration;
        let handle;

        if data.sound_type == SoundType::Static {
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
                    .volume(Decibels::from(data.volume))
                    .panning(Panning::from(data.pan))
                    .start_time(StartTime::ClockTime(ClockTime::from_ticks_f64(&clock, 0.0)));

            if data.repeat {
                sound_data = sound_data.loop_region(Some(Region {
                    start: PlaybackPosition::Seconds(0.0),
                    end: EndPosition::EndOfAudio,
                }));
            }

            if let Some(fade_in_param) = data.fade_in_param {
                sound_data = sound_data.fade_in_tween(Tween {
                    start_time: StartTime::Immediate,
                    duration: Duration::from_secs_f64(fade_in_param.duration),
                    easing: fade_in_param.easing.into(),
                });
            }

            duration = sound_data.duration().as_secs_f64();

            log::info!("LOAD: id={}, file={}", id, data.filepath.display());
            handle = SoundHandle::Static(manager.play(sound_data)?);
        } else {
            let mut sound_data =
                tokio::task::spawn_blocking(move || StreamingSoundData::from_file(filepath_clone))
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
                    .volume(Decibels::from(data.volume))
                    .panning(Panning::from(data.pan))
                    .start_time(StartTime::ClockTime(ClockTime::from_ticks_f64(&clock, 0.0)));

            if data.repeat {
                sound_data = sound_data.loop_region(Some(Region {
                    start: PlaybackPosition::Seconds(0.0),
                    end: EndPosition::EndOfAudio,
                }));
            }

            if let Some(fade_in_param) = data.fade_in_param {
                sound_data = sound_data.fade_in_tween(Tween {
                    start_time: StartTime::Immediate,
                    duration: Duration::from_secs_f64(fade_in_param.duration),
                    easing: fade_in_param.easing.into(),
                });
            }

            duration = sound_data.duration().as_secs_f64();

            log::info!("LOAD: id={}, file={}", id, data.filepath.display());
            handle = SoundHandle::Streaming(manager.play(sound_data)?);
        }

        let fade_out_tween = data.fade_out_param.map(|fade_out_param| Tween {
            start_time: StartTime::Immediate,
            duration: Duration::from_secs_f64(fade_out_param.duration),
            easing: fade_out_param.easing.into(),
        });

        self.event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Loaded {
                instance_id: id,
                position: handle.position(),
                duration,
            }))
            .await?;

        self.loaded_sounds.insert(
            id,
            PlaybackHandle {
                handle,
                clock,
                duration,
                repeat: data.repeat,
                volume: data.volume,
                fade_volume: 0.0,
                fade_out_tween,
            },
        );
        Ok(())
    }

    async fn handle_play(&mut self, id: Uuid, data: AudioCommandData) -> Result<()> {
        if !self.loaded_sounds.contains_key(&id) {
            self.handle_load(id, data.clone()).await?;
        }

        let mut handle = self
            .loaded_sounds
            .remove(&id)
            .ok_or_else(|| anyhow::anyhow!("Failed to get loaded sound."))?;
        handle.start();

        log::info!("PLAY: id={}, file={}", id, data.filepath.display());

        self.event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Started {
                instance_id: id,
                duration: handle.duration,
                initial_params: AudioStateParam {
                    repeating: data.repeat,
                    volume: data.volume,
                },
            }))
            .await?;

        self.playing_sounds.insert(
            id,
            PlayingSound {
                handle,
                last_status: None,
                manual_stop_sent: false,
            },
        );
        Ok(())
    }

    async fn handle_pause(&mut self, id: Uuid) -> Result<()> {
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
            anyhow::bail!("unknown instance_id. id={}", id);
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
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    fn handle_stop(&mut self, id: Uuid) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.stop();
            playing_sound.manual_stop_sent = true;
            Ok(())
        } else if let Some(mut loaded_sound) = self.loaded_sounds.remove(&id) {
            loaded_sound.stop();
            Ok(())
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_seek_to(&mut self, id: Uuid, position: f64) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.seek_to(position);
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Seeked {
                    instance_id: id,
                    position,
                }))
                .await?;
            Ok(())
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            loaded_handle.seek_to(position);
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Loaded {
                    instance_id: id,
                    position,
                    duration: loaded_handle.duration,
                }))
                .await?;
            Ok(())
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_seek_by(&mut self, id: Uuid, amount: f64) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            let position = playing_sound.handle.position() + amount;
            playing_sound.handle.seek_by(amount);
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Seeked {
                    instance_id: id,
                    position,
                }))
                .await?;
            Ok(())
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            loaded_handle.seek_by(amount);
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Loaded {
                    instance_id: id,
                    position: loaded_handle.position(),
                    duration: loaded_handle.duration,
                }))
                .await?;
            Ok(())
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_fade_volume(&mut self, id: Uuid, volume: f32, param: FadeParam) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.set_fade(
                volume,
                Tween {
                    start_time: StartTime::Immediate,
                    duration: Duration::from_secs_f64(param.duration),
                    easing: param.easing.into(),
                },
            );
            Ok(())
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            loaded_handle.set_fade(
                volume,
                Tween {
                    start_time: StartTime::Immediate,
                    duration: Duration::from_secs_f64(param.duration),
                    easing: param.easing.into(),
                },
            );
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Loaded {
                    instance_id: id,
                    position: loaded_handle.position(),
                    duration: loaded_handle.duration,
                }))
                .await?;
            Ok(())
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_action(&mut self, id: Uuid, action: AudioAction) -> Result<()> {
        match action {
            AudioAction::ToggleRepeat => {
                if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
                    let repeat_state = playing_sound.handle.is_repeating();
                    playing_sound.handle.set_repeat(!repeat_state);
                    let volume = playing_sound.handle.get_volume();
                    self.event_tx
                        .send(EngineEvent::Audio(AudioEngineEvent::StateParamUpdated {
                            instance_id: id,
                            params: AudioStateParam {
                                repeating: !repeat_state,
                                volume,
                            },
                        }))
                        .await?;
                } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
                    let repeat_state = loaded_handle.is_repeating();
                    loaded_handle.set_repeat(!repeat_state);
                    let volume = loaded_handle.get_volume();
                    self.event_tx
                        .send(EngineEvent::Audio(AudioEngineEvent::StateParamUpdated {
                            instance_id: id,
                            params: AudioStateParam {
                                repeating: !repeat_state,
                                volume,
                            },
                        }))
                        .await?;
                } else {
                    anyhow::bail!("unknown instance_id. id={}", id);
                }
            }
            AudioAction::SetVolume(volume) => {
                if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
                    let repeat_state = playing_sound.handle.is_repeating();
                    playing_sound.handle.set_volume(volume);
                    self.event_tx
                        .send(EngineEvent::Audio(AudioEngineEvent::StateParamUpdated {
                            instance_id: id,
                            params: AudioStateParam {
                                repeating: repeat_state,
                                volume,
                            },
                        }))
                        .await?;
                } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
                    let repeat_state = loaded_handle.is_repeating();
                    loaded_handle.set_volume(volume);
                    self.event_tx
                        .send(EngineEvent::Audio(AudioEngineEvent::StateParamUpdated {
                            instance_id: id,
                            params: AudioStateParam {
                                repeating: repeat_state,
                                volume,
                            },
                        }))
                        .await?;
                } else {
                    anyhow::bail!("unknown instance_id. id={}", id);
                }
            }
        }
        Ok(())
    }
}
