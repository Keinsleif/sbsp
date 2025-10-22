mod command;
mod event;
mod mono_effect;

pub use command::{AudioCommand, AudioCommandData};
pub use event::AudioEngineEvent;

use anyhow::{Context, Result};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Panning, StartTime, Tween,
    clock::{ClockHandle, ClockSpeed, ClockTime},
    sound::{
        EndPosition, FromFileError, PlaybackPosition, PlaybackState, Region, IntoOptionalRegion,
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
    model::{cue::audio::SoundType, settings::AudioSettings},
    controller::state::AudioStateParam,
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
}

struct PlaybackHandle {
    handle: SoundHandle,
    clock: ClockHandle,
    duration: f64,
    fade_out_tween: Option<Tween>,
    repeat: bool,
}

impl PlaybackHandle {
    fn state(&self) -> PlaybackState {
        self.handle.state()
    }

    fn is_repeating(&self) -> bool {
        self.repeat
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
    settings: AudioSettings,
    command_rx: mpsc::Receiver<AudioCommand>,
    event_tx: mpsc::Sender<EngineEvent>,
    playing_sounds: HashMap<Uuid, PlayingSound>,
    loaded_sounds: HashMap<Uuid, PlaybackHandle>,
}

impl AudioEngine {
    pub fn new(
        command_rx: mpsc::Receiver<AudioCommand>,
        event_tx: mpsc::Sender<EngineEvent>,
        settings: AudioSettings,
    ) -> Result<Self> {
        let mut audio_manager_settings = AudioManagerSettings::default();
        let mono_effect_handle = audio_manager_settings
            .main_track_builder
            .add_effect(MonoEffectBuilder::new(settings.mono_output));
        let manager = AudioManager::<DefaultBackend>::new(audio_manager_settings)
            .context("Failed to initialize AudioManager")?;

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
                        let position = playing_sound.handle.position();
                        let event = match playback_state {
                            kira::sound::PlaybackState::Playing => {
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position, duration: playing_sound.handle.duration })
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
                                EngineEvent::Audio(AudioEngineEvent::Progress { instance_id: *id, position, duration: playing_sound.handle.duration })
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
                        if let Err(e) = self.event_tx.send(event).await {
                            log::error!("Error polling Sound status: {:?}", e);
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
        let manager = self.manager.as_mut().unwrap();
        let clock = manager.add_clock(ClockSpeed::SecondsPerTick(1.0)).unwrap();

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
                fade_out_tween,
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
                initial_params: AudioStateParam {
                    repeating: data.repeat,
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
            Err(anyhow::anyhow!(
                "Sound with ID {} not found for resume.",
                id
            ))
        }
    }

    fn handle_stop(&mut self, id: Uuid) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.stop();
            playing_sound.manual_stop_sent = true;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Sound with ID {} not found for stop.", id))
        }
    }

    async fn handle_seek_to(&mut self, id: Uuid, position: f64) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.seek_to(position);
            Ok(())
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            loaded_handle.seek_to(position);
            self.event_tx.send(EngineEvent::Audio(AudioEngineEvent::Loaded { instance_id: id, position: loaded_handle.position(), duration: loaded_handle.duration })).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Sound with ID {} not found for seek.", id))
        }
    }

    async fn handle_seek_by(&mut self, id: Uuid, amount: f64) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.seek_by(amount);
            Ok(())
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            loaded_handle.seek_by(amount);
            self.event_tx.send(EngineEvent::Audio(AudioEngineEvent::Loaded { instance_id: id, position: loaded_handle.position(), duration: loaded_handle.duration })).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Sound with ID {} not found for seek.", id))
        }
    }

    async fn handle_action(&mut self, id: Uuid, action: AudioAction) -> Result<()> {
        match action {
            AudioAction::ToggleRepeat => {
                if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
                    let repeat_state = playing_sound.handle.is_repeating();
                    playing_sound.handle.set_repeat(!repeat_state);
                    self.event_tx.send(EngineEvent::Audio(AudioEngineEvent::StateParamUpdated{
                        instance_id: id,
                        params: AudioStateParam {
                            repeating: !repeat_state,
                        },
                    })).await?;
                } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
                    let repeat_state = loaded_handle.is_repeating();
                    loaded_handle.set_repeat(!repeat_state);
                    self.event_tx.send(EngineEvent::Audio(AudioEngineEvent::StateParamUpdated{
                        instance_id: id,
                        params: AudioStateParam {
                            repeating: !repeat_state,
                        },
                    })).await?;
                } else {
                    return Err(anyhow::anyhow!("Sound with ID {} not found for seek.", id));
                }
            }
        }
        Ok(())
    }
}
