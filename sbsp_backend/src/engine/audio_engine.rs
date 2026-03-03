mod command;
mod event;
pub mod level_meter;
mod mono;
mod audio_soucre;
mod lowcost_skip;
mod static_source;

pub use command::{AudioCommand, AudioCommandData};
pub use event::AudioEngineEvent;

use anyhow::{Context, Result};
use std::{collections::HashMap, fs::File, num::NonZero, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};
use tokio::{sync::mpsc, time};
use uuid::Uuid;
use rodio::{Decoder, Source, mixer::Mixer, source::{SeekError, Zero}, stream::{DeviceSinkBuilder, MixerDeviceSink}};

use super::EngineEvent;
use crate::{
    action::AudioAction,
    controller::state::AudioStateParam,
    engine::audio_engine::{audio_soucre::{AudioPlaybackState, AudioSource, AudioSourceHandle, AudioSourceSettings, ChannelMapping}, level_meter::{LevelMeter, SharedLevel}, static_source::StaticSource},
    model::{
        cue::audio::{Decibels, FadeParam, SoundType},
        settings::ShowAudioSettings,
    },
};

const NANOS_PER_SEC: u64 = 1_000_000_000;

struct PlaybackHandle {
    handle: AudioSourceHandle,
    duration: f64,
    volume: Decibels,
    fade_volume: Decibels,
}

impl PlaybackHandle {
    fn state(&self) -> AudioPlaybackState {
        self.handle.state()
    }

    fn is_repeating(&self) -> bool {
        self.handle.is_repeating()
    }

    fn get_volume(&self) -> Decibels {
        self.volume
    }

    fn position(&self) -> f64 {
        self.handle.position()
    }

    fn start(&mut self) {
        self.handle.start();
    }

    fn pause(&mut self) {
        self.handle.pause();
    }

    fn resume(&mut self) {
        self.handle.resume();
    }

    fn stop(&mut self) {
        let state = self.state();
        if state == AudioPlaybackState::Stopped || state == AudioPlaybackState::Completed {
            return;
        }
        if self.state().eq(&AudioPlaybackState::SoftStopping) {
            // Hard Stop
            self.handle.stop();
        } else {
            self.handle.fade_out();
        }
    }

    async fn seek_to(&mut self, position: f64) -> Result<(), SeekError> {
        self.handle.seek_to(position).await
    }

    async fn seek_by(&mut self, amount: f64) -> Result<(), SeekError> {
        self.handle.seek_by(amount).await
    }

    fn set_repeat(&mut self, repeat: bool) {
        self.handle.set_repeat(repeat);
    }

    fn set_volume(&mut self, volume: Decibels) {
        self.volume = volume;
        self.handle
            .set_volume(self.volume + self.fade_volume);
    }

    fn set_fade(&mut self, volume: Decibels, fade_param: FadeParam) {
        self.fade_volume = volume;
        self.handle
            .set_fade(self.volume + self.fade_volume, fade_param);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LastStatus {
    state: AudioPlaybackState,
    position: f64,
    duration: f64,
}

struct PlayingSound {
    handle: PlaybackHandle,
    last_status: Option<LastStatus>,
}

pub struct AudioEngine {
    _output: Option<MixerDeviceSink>,
    audio_mixer: Option<Mixer>,
    is_mono: Arc<AtomicBool>,
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
        let is_mono = Arc::new(AtomicBool::new(settings.mono_output));
        let builder = DeviceSinkBuilder::from_default_device()?;
        let output = builder.with_channels(NonZero::new(settings.output_channels).ok_or(anyhow::anyhow!("output_hannels must be NonZero value."))?).open_stream()?;
        let (channel_count, sample_rate) = {
            let output_config = output.config();
            (output_config.channel_count(), output_config.sample_rate())
        };
        
        let (main_mixer, mixer_source) = rodio::mixer::mixer(channel_count, sample_rate);
        main_mixer.add(Zero::new(channel_count, sample_rate));

        let main_source = mono::Mono::new(mixer_source, is_mono.clone());

        output.mixer().add(main_source);

        Ok(Self {
            _output: Some(output),
            audio_mixer: Some(main_mixer),
            is_mono,
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
        let is_mono = Arc::new(AtomicBool::new(settings.mono_output));
        let builder = DeviceSinkBuilder::from_default_device()?;
        let output = builder.with_channels(NonZero::new(settings.output_channels).ok_or(anyhow::anyhow!("output_hannels must be NonZero value."))?).open_stream()?;
        let (channel_count, sample_rate) = {
            let output_config = output.config();
            (output_config.channel_count(), output_config.sample_rate())
        };
        
        let (main_mixer, mixer_source) = rodio::mixer::mixer(channel_count, sample_rate);
        main_mixer.add(Zero::new(channel_count, sample_rate));

        let main_source = LevelMeter::new(mono::Mono::new(mixer_source, is_mono.clone()), shared_level.clone());

        output.mixer().add(main_source);

        Ok((
            Self {
                _output: Some(output),
                audio_mixer: Some(main_mixer),
                is_mono,
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
                                self.is_mono.store(settings.mono_output, Ordering::Release);
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
                                    if let Some(last_state) = &playing_sound.last_status && last_state.state == playback_state {
                                        AudioEngineEvent::Progress { instance_id: id, position, duration: playing_sound.handle.duration }
                                    } else {
                                        AudioEngineEvent::Stopping { instance_id: id, position, duration: playing_sound.handle.duration }
                                    }
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
        let mixer = self
            .audio_mixer
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

                duration = audio_source.total_duration().map_or(0.0, |d| d.as_secs_f64());
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

                duration = audio_source.total_duration().map_or(0.0, |d| d.as_secs_f64());
                mixer.add(audio_source);
            }
        }

        log::info!("LOAD: id={}, file={}, duration={}", id, data.filepath.display(), duration);

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
                duration,
                volume: data.volume,
                fade_volume: Decibels::IDENTITY,
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
            if matches!(playing_sound
                .handle
                .state(), AudioPlaybackState::Pausing | AudioPlaybackState::Paused)
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
            Ok(())
        } else if let Some(mut loaded_sound) = self.loaded_sounds.remove(&id) {
            loaded_sound.stop();
            Ok(())
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_seek_to(&mut self, id: Uuid, position: f64) -> Result<()> {
        if let Some(playing_sound) = self.playing_sounds.get_mut(&id) && playing_sound.handle.seek_to(position).await.is_ok() {
            self.event_tx
                .send(EngineEvent::Audio(AudioEngineEvent::Seeked {
                    instance_id: id,
                    position,
                }))
                .await?;
            Ok(())
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) && loaded_handle.seek_to(position).await.is_ok() {
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
            if let Err(e) = playing_sound.handle.seek_by(amount).await {
                Err(anyhow::anyhow!("Failed to perform seek. e={}", e))
            } else {
                self.event_tx
                    .send(EngineEvent::Audio(AudioEngineEvent::Seeked {
                        instance_id: id,
                        position,
                    }))
                    .await?;
                Ok(())
            }
        } else if let Some(loaded_handle) = self.loaded_sounds.get_mut(&id) {
            let position = loaded_handle.position() + amount;
            if let Err(e) = loaded_handle.seek_by(amount).await {
                Err(anyhow::anyhow!("Failed to perform seek. e={}", e))
            } else {
                self.event_tx
                    .send(EngineEvent::Audio(AudioEngineEvent::Loaded {
                        instance_id: id,
                        position,
                        duration: loaded_handle.duration,
                    }))
                    .await?;
                Ok(())
            }
        } else {
            anyhow::bail!("unknown instance_id. id={}", id);
        }
    }

    async fn handle_fade_volume(&mut self, id: Uuid, volume: Decibels, param: FadeParam) -> Result<()> {
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
