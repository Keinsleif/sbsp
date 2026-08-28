// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod audio_source;
mod command;
mod event;
pub mod level_meter;
mod lowcost_skip;
mod mono;
mod static_source;

pub use command::{AudioCommand, AudioCommandData};
pub use event::AudioEngineEvent;

use anyhow::{Context, Result};
use rodio::{
    Decoder, DeviceTrait, Source,
    cpal::{BufferSize, DeviceId, SampleFormat, SupportedBufferSize, traits::HostTrait},
    mixer::Mixer,
    source::Zero,
    stream::{DeviceSinkBuilder, MixerDeviceSink},
};
use std::{
    collections::HashMap,
    fs::File,
    str::FromStr as _,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, watch},
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use super::EngineEvent;
use crate::{
    BackendAudioSettings, BackendSettings,
    action::AudioAction,
    controller::state::AudioStateParam,
    engine::audio_engine::{
        audio_source::{
            AudioPlaybackState, AudioSource, AudioSourceHandle, AudioSourceSettings, ChannelMapping,
        },
        level_meter::{LevelMeter, SharedLevel},
        static_source::StaticSource,
    },
    model::{
        cue::audio::{Decibels, FadeParam, SoundType},
        settings::ShowAudioSettings,
    },
};

const NANOS_PER_SEC: u64 = 1_000_000_000;

struct AudioOutput {
    _sink: MixerDeviceSink,
    mixer: Mixer,
}

impl AudioOutput {
    fn add<T>(&self, source: T)
    where
        T: Source + Send + 'static,
    {
        self.mixer.add(source);
    }
}

struct OutputFallbackInfo {
    device: bool,
    config: bool,
}

impl OutputFallbackInfo {
    fn is_fallbacked(&self) -> bool {
        self.device || self.config
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LastStatus {
    state: AudioPlaybackState,
    position: f64,
    duration: f64,
}

struct PlayingSound {
    handle: AudioSourceHandle,
    last_status: Option<LastStatus>,
}

pub struct AudioEngine {
    output: Option<AudioOutput>,
    is_mono: Arc<AtomicBool>,
    show_settings: ShowAudioSettings,
    backend_settings: BackendAudioSettings,
    command_rx: mpsc::Receiver<AudioCommand>,
    event_tx: mpsc::Sender<EngineEvent>,
    backend_settings_rx: watch::Receiver<BackendSettings>,
    level_meter: Option<SharedLevel>,

    playing_sounds: HashMap<Uuid, PlayingSound>,
    loaded_sounds: HashMap<Uuid, AudioSourceHandle>,
}

impl AudioEngine {
    pub fn new(
        command_rx: mpsc::Receiver<AudioCommand>,
        event_tx: mpsc::Sender<EngineEvent>,
        backend_settings_rx: watch::Receiver<BackendSettings>,
        show_settings: ShowAudioSettings,
    ) -> Result<Self> {
        let backend_settings = backend_settings_rx.borrow().audio.clone();
        let is_mono = Arc::new(AtomicBool::new(show_settings.mono_output));
        let (builder, fallback_info) = Self::get_builder(&backend_settings)?;
        if fallback_info.is_fallbacked()
            && let Err(e) =
                event_tx.blocking_send(EngineEvent::Audio(AudioEngineEvent::AudioOutputFallback {
                    device: fallback_info.device,
                    config: fallback_info.config,
                }))
        {
            log::error!("Failed to send audio output fallback event. e={}", e);
        }
        let mut sink = builder.open_stream()?;
        sink.log_on_drop(false);
        let (channel_count, sample_rate) = {
            let output_config = sink.config();
            (output_config.channel_count(), output_config.sample_rate())
        };

        let (main_mixer, mixer_source) = rodio::mixer::mixer(channel_count, sample_rate);
        main_mixer.add(Zero::new(channel_count, sample_rate));

        let main_source = mono::Mono::new(mixer_source, is_mono.clone());

        sink.mixer().add(main_source);

        let output = AudioOutput {
            _sink: sink,
            mixer: main_mixer,
        };

        Ok(Self {
            output: Some(output),
            is_mono,
            show_settings,
            backend_settings,
            command_rx,
            event_tx,
            backend_settings_rx,
            level_meter: None,
            playing_sounds: HashMap::new(),
            loaded_sounds: HashMap::new(),
        })
    }

    pub fn new_with_level_meter(
        command_rx: mpsc::Receiver<AudioCommand>,
        event_tx: mpsc::Sender<EngineEvent>,
        backend_settings_rx: watch::Receiver<BackendSettings>,
        show_settings: ShowAudioSettings,
    ) -> Result<(Self, SharedLevel)> {
        let shared_level = SharedLevel::default();
        let backend_settings = backend_settings_rx.borrow().audio.clone();
        let is_mono = Arc::new(AtomicBool::new(show_settings.mono_output));
        let (builder, fallback_info) = Self::get_builder(&backend_settings)?;
        if fallback_info.is_fallbacked()
            && let Err(e) =
                event_tx.blocking_send(EngineEvent::Audio(AudioEngineEvent::AudioOutputFallback {
                    device: fallback_info.device,
                    config: fallback_info.config,
                }))
        {
            log::error!("Failed to send audio output fallback event. e={}", e);
        }
        let mut sink = builder.open_stream()?;
        sink.log_on_drop(false);
        let (channel_count, sample_rate) = {
            let output_config = sink.config();
            (output_config.channel_count(), output_config.sample_rate())
        };

        let (main_mixer, mixer_source) = rodio::mixer::mixer(channel_count, sample_rate);
        main_mixer.add(Zero::new(channel_count, sample_rate));

        let main_source = LevelMeter::new(
            mono::Mono::new(mixer_source, is_mono.clone()),
            shared_level.clone(),
        );

        sink.mixer().add(main_source);
        let output = AudioOutput {
            _sink: sink,
            mixer: main_mixer,
        };

        Ok((
            Self {
                output: Some(output),
                is_mono,
                show_settings,
                backend_settings,
                command_rx,
                event_tx,
                backend_settings_rx,
                level_meter: Some(shared_level.clone()),
                playing_sounds: HashMap::new(),
                loaded_sounds: HashMap::new(),
            },
            shared_level,
        ))
    }

    async fn rebuild_output(&mut self, backend: &BackendAudioSettings) -> Result<()> {
        log::debug!("Rebuilding Audio Output...");
        self.output = None;
        let is_mono = self.is_mono.clone();
        let (builder, fallback_info) = Self::get_builder(backend)?;
        if fallback_info.is_fallbacked()
            && let Err(e) = self
                .event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::AudioOutputFallback {
                    device: fallback_info.device,
                    config: fallback_info.config,
                }))
                .await
        {
            log::error!("Failed to send audio output fallback event. e={}", e);
        }
        let mut sink = builder.open_stream()?;
        sink.log_on_drop(false);
        let (channel_count, sample_rate) = {
            let output_config = sink.config();
            (output_config.channel_count(), output_config.sample_rate())
        };

        let (main_mixer, mixer_source) = rodio::mixer::mixer(channel_count, sample_rate);
        main_mixer.add(Zero::new(channel_count, sample_rate));

        if let Some(shared_level) = &self.level_meter {
            let main_source = LevelMeter::new(
                mono::Mono::new(mixer_source, is_mono.clone()),
                shared_level.clone(),
            );

            sink.mixer().add(main_source);
        } else {
            let main_source = mono::Mono::new(mixer_source, is_mono.clone());

            sink.mixer().add(main_source);
        }
        self.backend_settings = backend.clone();
        self.output = Some(AudioOutput {
            _sink: sink,
            mixer: main_mixer,
        });
        Ok(())
    }

    fn get_builder(
        settings: &BackendAudioSettings,
    ) -> Result<(DeviceSinkBuilder, OutputFallbackInfo)> {
        let mut fallback_info = OutputFallbackInfo {
            device: false,
            config: false,
        };
        let device_result = if let Some(device_id) = &settings.device_id
            && let Ok(id) = DeviceId::from_str(device_id)
            && let Ok(host) = rodio::cpal::host_from_id(id.0)
            && let Some(device) = host.device_by_id(&id)
            && device.supports_output()
        {
            Ok(device)
        } else {
            let host = rodio::cpal::default_host();
            if settings.device_id.is_some() {
                fallback_info.device = true;
            }
            host.default_output_device()
                .ok_or(anyhow::anyhow!("No default device found."))
        };

        if let Ok(device) = device_result {
            let mut matched_config = None;

            let default_sample_rate = device.default_output_config().ok().map(|c| c.sample_rate());

            let target_sample_rate = settings.sample_rate.or(default_sample_rate);

            if let Ok(configs) = device.supported_output_configs() {
                for config in configs {
                    if config.sample_format() != SampleFormat::F32 {
                        continue;
                    }
                    if let Some(channels) = settings.channel_count
                        && config.channels() != channels
                    {
                        continue;
                    }
                    if let Some(buffer_size) = settings.buffer_size {
                        match config.buffer_size() {
                            SupportedBufferSize::Range { min, max }
                                if (buffer_size < *min || buffer_size > *max) =>
                            {
                                continue;
                            }
                            SupportedBufferSize::Range { .. } => {}
                            SupportedBufferSize::Unknown => continue,
                        }
                    }

                    if let Some(sample_rate) = target_sample_rate
                        && let Some(config) = config.try_with_sample_rate(sample_rate)
                    {
                        matched_config = Some(config.config());
                        break;
                    }
                }
            }
            let mut builder = DeviceSinkBuilder::from_device(device)?;
            if let Some(mut config) = matched_config {
                if let Some(buffer_size) = settings.buffer_size {
                    config.buffer_size = BufferSize::Fixed(buffer_size);
                }

                builder = builder
                    .with_config(&config)
                    .with_sample_format(SampleFormat::F32);
            } else if settings.channel_count.is_some() || settings.sample_rate.is_some() || settings.buffer_size.is_some() {
                fallback_info.config = true;
            }
            return Ok((builder, fallback_info));
        }
        Err(anyhow::anyhow!("Failed to create DeviceSinkBuilder."))
    }

    pub async fn run(mut self) {
        let mut poll_timer = time::interval(Duration::from_millis(50));
        poll_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
        log::info!("AudioEngine run loop started");
        loop {
            tokio::select! {
                Ok(_) = self.backend_settings_rx.changed() => {
                    let settings = self.backend_settings_rx.borrow().audio.clone();
                    if settings != self.backend_settings
                    && let Err(e) = self.rebuild_output(&settings).await {
                        log::error!("Failed to rebuild output. e={}", e);
                    }
                }
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
                        AudioCommand::SoftStop { id } => self.handle_stop(id, false).await,
                        AudioCommand::HardStop { id } => self.handle_stop(id, true).await,
                        AudioCommand::SeekTo { id, position } => self.handle_seek_to(id, position).await,
                        AudioCommand::SeekBy { id, amount } => self.handle_seek_by(id, amount).await,
                        AudioCommand::FadeVolume { id, volume, fade_param } => self.handle_fade_volume(id, volume, fade_param).await,
                        AudioCommand::PerformAction { id, action } => self.handle_action(id, action).await,
                        AudioCommand::Reconfigure(settings) => {
                            if settings.mono_output != self.show_settings.mono_output {
                                self.is_mono.store(settings.mono_output, Ordering::Release);
                            }
                            self.show_settings = settings.clone();
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
                    let keys: Vec<_> = self.playing_sounds.keys().cloned().collect();
                    for id in keys {
                        let event = {
                            let Some(playing_sound) = self.playing_sounds.get(&id) else {
                                continue;
                            };
                            let playback_state = playing_sound.handle.state();
                            let position = playing_sound.handle.position();
                            match playback_state {
                                AudioPlaybackState::Loaded => continue,
                                AudioPlaybackState::Playing |
                                AudioPlaybackState::Pausing |
                                AudioPlaybackState::Resuming => {
                                    AudioEngineEvent::Progress { instance_id: id, position, duration: playing_sound.handle.duration }
                                },
                                AudioPlaybackState::Paused => {
                                    if let Some(last_state) = &playing_sound.last_status
                                        && last_state.eq(&LastStatus {state: playback_state, position, duration: playing_sound.handle.duration }) {
                                            continue;
                                    }
                                    AudioEngineEvent::Paused { instance_id: id, position, duration: playing_sound.handle.duration }
                                },
                                AudioPlaybackState::HardStopping |
                                AudioPlaybackState::SoftStopping => {
                                    AudioEngineEvent::Stopping { instance_id: id, position, duration: playing_sound.handle.duration }
                                },
                                AudioPlaybackState::Stopped => {
                                    log::info!("STOP: id={}", id);
                                    AudioEngineEvent::Stopped { instance_id: id }
                                },
                                AudioPlaybackState::Completed => {
                                    log::info!("COMPLETE: id={}", id);
                                    AudioEngineEvent::Completed { instance_id: id }
                                },
                            }
                        };
                        if matches!(event, AudioEngineEvent::Progress {..}) {
                            if let Err(e) = self.event_tx.try_send(EngineEvent::Audio(event)) {
                                log::warn!("EngineEvent dropped: {:?}", e);
                            }
                        } else if self.event_tx.send(EngineEvent::Audio(event)).await.is_err() {
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
                    self.playing_sounds.retain(|_, value| !matches!(value.handle.state(), AudioPlaybackState::Stopped | AudioPlaybackState::Completed));
                },
                else => break
            }
        }
        log::info!("AudioEngine run loop finished.");
    }

    async fn handle_load(&mut self, id: Uuid, data: AudioCommandData) -> Result<()> {
        if self.loaded_sounds.contains_key(&id) {
            anyhow::bail!("Audio cue already loaded. id={}", id);
        }

        let mixer = self
            .output
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Audio Mixer is not available"))?;

        let mut settings = AudioSourceSettings::from(&data);
        settings.channel_mapping = ChannelMapping::from_pan(data.pan);

        let handle;
        let duration;

        let filepath_clone = data.filepath.clone();
        match data.sound_type {
            SoundType::Static => {
                let audio_source;
                (audio_source, handle) =
                    tokio::task::spawn_blocking(move || -> Result<(AudioSource<StaticSource>, AudioSourceHandle), anyhow::Error> {
                        let file = File::open(filepath_clone)?;
                        let len = file.metadata()?.len();
                        let decoder = Decoder::builder().with_data(file).with_byte_len(len).with_seekable(true).build()?;
                        Ok(AudioSource::new(StaticSource::new(decoder), settings))
                    })
                    .await?
                    .with_context(|| {
                        format!(
                            "Failed to load sound data from: {}",
                            data.filepath.display()
                        )
                    })?;

                duration = audio_source
                    .total_duration()
                    .map_or(0.0, |d| d.as_secs_f64());
                mixer.add(audio_source);
            }
            SoundType::Streaming => {
                let audio_source;
                (audio_source, handle) =
                    tokio::task::spawn_blocking(move || -> Result<(AudioSource<Decoder<File>>, AudioSourceHandle), anyhow::Error> {
                        let file = File::open(filepath_clone)?;
                        let len = file.metadata()?.len();
                        let decoder = Decoder::builder().with_data(file).with_byte_len(len).with_seekable(true).build()?;
                        Ok(AudioSource::new(decoder, settings))
                    })
                    .await?
                    .with_context(|| {
                        format!(
                            "Failed to load sound data from: {}",
                            data.filepath.display()
                        )
                    })?;

                duration = audio_source
                    .total_duration()
                    .map_or(0.0, |d| d.as_secs_f64());
                mixer.add(audio_source);
            }
        }

        log::info!(
            "LOAD: id={}, file={}, duration={}",
            id,
            data.filepath.display(),
            duration
        );

        self.event_tx
            .send(EngineEvent::Audio(AudioEngineEvent::Loaded {
                instance_id: id,
                position: handle.position(),
                duration,
            }))
            .await?;

        self.loaded_sounds.insert(id, handle);
        Ok(())
    }

    async fn handle_play(&mut self, id: Uuid, data: AudioCommandData) -> Result<()> {
        if self.playing_sounds.contains_key(&id) {
            anyhow::bail!("Audio cue already playing. id={}", id);
        }
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
                position: handle.position(),
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
            },
        );
        Ok(())
    }

    async fn handle_pause(&mut self, id: Uuid) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.pause();
            Ok(())
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_resume(&mut self, id: Uuid) -> Result<()> {
        log::info!("RESUME: id={}", id);
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            if matches!(
                playing_sound.handle.state(),
                AudioPlaybackState::Pausing | AudioPlaybackState::Paused
            ) {
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

    async fn handle_stop(&mut self, id: Uuid, is_hard: bool) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.stop(is_hard);
            Ok(())
        } else if let Some(mut loaded_sound) = self.loaded_sounds.remove(&id) {
            loaded_sound.stop(is_hard);
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Stopped {
                    instance_id: id,
                }))
                .await?;
            Ok(())
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_seek_to(&mut self, id: Uuid, position: f64) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            match playing_sound.handle.seek_to(position).await {
                Ok(seeked_position) => {
                    self.event_tx
                        .send(EngineEvent::Audio(AudioEngineEvent::Seeked {
                            instance_id: id,
                            position: seeked_position,
                        }))
                        .await?;
                    Ok(())
                }
                Err(e) => Err(anyhow::anyhow!("Failed to perform seek. e={}", e)),
            }
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            match loaded_handle.seek_to(position).await {
                Ok(seeked_position) => {
                    self.event_tx
                        .send(EngineEvent::Audio(AudioEngineEvent::Seeked {
                            instance_id: id,
                            position: seeked_position,
                        }))
                        .await?;
                    Ok(())
                }
                Err(e) => Err(anyhow::anyhow!("Failed to perform seek. e={}", e)),
            }
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_seek_by(&mut self, id: Uuid, amount: f64) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            match playing_sound.handle.seek_by(amount).await {
                Ok(seeked_position) => {
                    self.event_tx
                        .send(EngineEvent::Audio(AudioEngineEvent::Seeked {
                            instance_id: id,
                            position: seeked_position,
                        }))
                        .await?;
                    Ok(())
                }
                Err(e) => Err(anyhow::anyhow!("Failed to perform seek. e={}", e)),
            }
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            match loaded_handle.seek_by(amount).await {
                Ok(seeked_position) => {
                    self.event_tx
                        .send(EngineEvent::Audio(AudioEngineEvent::Seeked {
                            instance_id: id,
                            position: seeked_position,
                        }))
                        .await?;
                    Ok(())
                }
                Err(e) => Err(anyhow::anyhow!("Failed to perform seek. e={}", e)),
            }
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_fade_volume(
        &mut self,
        id: Uuid,
        volume: Decibels,
        param: FadeParam,
    ) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) {
            playing_sound.handle.set_fade(volume, param);
            Ok(())
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            loaded_handle.set_fade(volume, param);
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
