use std::{num::NonZero, ops::{Deref, DerefMut}, sync::{Arc, atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering}}, time::Duration};

use rodio::{ChannelCount, Sample, SampleRate, Source, source::{SeekError, SkipDuration, TakeDuration}};
use rtrb::{Consumer, Producer, RingBuffer};
use tokio::sync::oneshot;

use crate::{engine::audio_engine::AudioCommandData, model::cue::audio::{Decibels, Easing, FadeParam}};

const DEFAULT_FADE_PARAM: FadeParam = FadeParam {
    duration: 0.001,
    easing: Easing::Linear,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[repr(u8)]
pub enum AudioPlaybackState {
    Loaded,
    Playing,
    Pausing,
    Paused,
    Resuming,
    SoftStopping,
    HardStopping,
    Stopped,
    Completed,
}

impl TryFrom<u8> for AudioPlaybackState {
    type Error = ();
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            x if x == Self::Loaded as u8 => Ok(Self::Loaded),
            x if x == Self::Playing as u8 => Ok(Self::Playing),
            x if x == Self::Pausing as u8 => Ok(Self::Pausing),
            x if x == Self::Paused as u8 => Ok(Self::Paused),
            x if x == Self::Resuming as u8 => Ok(Self::Resuming),
            x if x == Self::SoftStopping as u8 => Ok(Self::SoftStopping),
            x if x == Self::HardStopping as u8 => Ok(Self::HardStopping),
            x if x == Self::Stopped as u8 => Ok(Self::Stopped),
            x if x == Self::Completed as u8 => Ok(Self::Completed),
            _ => Err(()),
        }
    }
}

impl PartialEq<u8> for AudioPlaybackState {
    fn eq(&self, other: &u8) -> bool {
        (*self as u8) == *other
    }
}

impl PartialEq<AudioPlaybackState> for u8 {
    fn eq(&self, other: &AudioPlaybackState) -> bool {
        *self == (*other as u8)
    }
}

enum AudioSourceControlCommand {
    Start,
    Pause,
    Resume,
    FadeOut,
    Stop,
    Seek {
        position: f64,
        result: oneshot::Sender<Result<(), SeekError>>
    },
    SetVolume {
        volume: Decibels,
        fade_param: FadeParam,
    }
}

struct AudioSourceShared {
    state: AtomicU8,
    position: AtomicU64,
    repeat: AtomicBool,
}

impl AudioSourceShared {
    fn new(repeat: bool) -> Self {
        Self {
            state: AtomicU8::new(AudioPlaybackState::Loaded as u8),
            position: AtomicU64::new(0),
            repeat: AtomicBool::new(repeat),
        }
    }
}

pub struct AudioSourceHandle {
    shared: Arc<AudioSourceShared>,
    control: Producer<AudioSourceControlCommand>,
}

impl AudioSourceHandle {
    pub fn state(&self) -> AudioPlaybackState {
        self.shared.state.load(Ordering::Acquire).try_into().unwrap()
    }

    pub fn position(&self) -> f64 {
        f64::from_bits(self.shared.position.load(Ordering::Acquire))
    }

    pub fn is_repeating(&self) -> bool {
        self.shared.repeat.load(Ordering::Acquire)
    }

    pub fn start(&mut self) {
        if self.state() == AudioPlaybackState::Loaded {
            let _ = self.control.push(AudioSourceControlCommand::Start);
        }
    }

    pub fn resume(&mut self) {
        if self.state() == AudioPlaybackState::Paused {
            let _ = self.control.push(AudioSourceControlCommand::Resume);
        }
    }

    pub fn pause(&mut self) {
        let _ = self.control.push(AudioSourceControlCommand::Pause);
    }

    pub fn stop(&mut self) {
        let _ = self.control.push(AudioSourceControlCommand::Stop);
    }

    pub fn fade_out(&mut self) {
        let _ = self.control.push(AudioSourceControlCommand::FadeOut);
    }

    pub async fn seek_to(&mut self, position: f64) -> Result<(), SeekError> {
        let (result_tx, result_rx) = oneshot::channel();
        let _ = self.control.push(AudioSourceControlCommand::Seek { position, result: result_tx });
        result_rx.await.unwrap_or(Ok(()))
    }

    pub async fn seek_by(&mut self, amount: f64) -> Result<(), SeekError> {
        let (result_tx, result_rx) = oneshot::channel();
        let _ = self.control.push(AudioSourceControlCommand::Seek { position: self.position() + amount, result: result_tx });
        result_rx.await.unwrap_or(Ok(()))
    }

    pub fn set_repeat(&self, value: bool) {
        self.shared.repeat.store(value, Ordering::Release);
    }

    pub fn set_volume(&mut self, volume: Decibels) {
        let _ = self.control.push(AudioSourceControlCommand::SetVolume { volume, fade_param: DEFAULT_FADE_PARAM });
    }

    pub fn set_fade(&mut self, volume: Decibels, fade_param: FadeParam) {
        let _ = self.control.push(AudioSourceControlCommand::SetVolume { volume, fade_param });
    }
}

struct VolumeFadeInfo {
    pub from: f32,
    pub to: f32,
    pub elapsed: f64,
    pub fade_param: FadeParam,
}

struct Volume {
    pub volume: f32,
    pub fade_info: Option<VolumeFadeInfo>,
}

impl Volume {
    pub fn new(volume: f32) -> Self {
        Self {
            volume,
            fade_info: None,
        }
    }

    pub fn new_with_fade(init: f32, target: f32, fade_param: FadeParam) -> Self {
        Self {
            volume: init,
            fade_info: Some(VolumeFadeInfo { from: init, to: target, elapsed: 0.0, fade_param })
        }
    }

    pub fn set_volume(&mut self, volume: f32, fade_param: FadeParam) {
        if let Some(info) = self.fade_info.as_mut() {
            info.from = self.volume;
            info.to = volume;
            info.elapsed = 0.0;
            info.fade_param = fade_param;
        } else {
            self.fade_info = Some(VolumeFadeInfo { from: self.volume, to: volume, elapsed: 0.0, fade_param });
        }
    }

    pub fn update(&mut self, dt: f64) -> bool {
        if let Some(info) = self.fade_info.as_mut() {
            if info.elapsed >= info.fade_param.duration {
                self.volume = info.to;
                return true;
            }

            self.volume = info.from + info.fade_param.easing.get_factor(info.elapsed / info.fade_param.duration) as f32 * (info.to - info.from);

            info.elapsed += dt;
        }
        false
    }
}

pub struct AudioSourceSettings {
    pub repeat: bool,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub fadeout_param: Option<FadeParam>,
    pub fadein_param: Option<FadeParam>,
    pub volume: Decibels,
    pub output_channels: u16,
}

impl From<&AudioCommandData> for AudioSourceSettings {
    fn from(value: &AudioCommandData) -> Self {
        Self { repeat: value.repeat, start_time: value.start_time, end_time: value.end_time, fadeout_param: value.fade_out_param, fadein_param: value.fade_in_param, volume: value.volume, output_channels: 2 }
    }
}

enum InnerSource<I> {
    Original(I),
    Skipped(SkipDuration<I>),
    Taken(TakeDuration<I>),
    Ranged(SkipDuration<TakeDuration<I>>),
}

impl<I> Deref for InnerSource<I> where I: Source + 'static {
    type Target = dyn Source;

    fn deref(&self) -> &Self::Target {
        match self {
            InnerSource::Original(inner) => inner as &dyn Source,
            InnerSource::Skipped(skip_duration) => skip_duration as &dyn Source,
            InnerSource::Taken(take_duration) => take_duration as &dyn Source,
            InnerSource::Ranged(skip_duration) => skip_duration as &dyn Source,
        }
    }
}

impl<I> DerefMut for InnerSource<I> where I: Source + 'static {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            InnerSource::Original(inner) => inner as &mut dyn Source,
            InnerSource::Skipped(skip_duration) => skip_duration as &mut dyn Source,
            InnerSource::Taken(take_duration) => take_duration as &mut dyn Source,
            InnerSource::Ranged(skip_duration) => skip_duration as &mut dyn Source,
        }
    }
}

pub struct AudioSource<I> where I: Source {
    input: InnerSource<I>,
    shared: Arc<AudioSourceShared>,
    control: Consumer<AudioSourceControlCommand>,
    settings: AudioSourceSettings,
    fadeout_param: FadeParam,
    current_channel: u16,
    samples_counted: usize,
    offset_duration: f64,
    current_span_channels: ChannelCount,
    current_span_sample_rate: SampleRate,
    first_channel_sample: Option<Sample>,
    control_volume: Volume,
    volume: Volume,
}

impl<I> AudioSource<I> where I: Source + 'static {
    pub fn new(input: I, settings: AudioSourceSettings) -> (Self, AudioSourceHandle)
    where I: Source {
        let channels = input.channels();
        let sample_rate = input.sample_rate();
        let fadeout_param = settings.fadeout_param.unwrap_or(DEFAULT_FADE_PARAM);
        let shared = Arc::new(AudioSourceShared::new(settings.repeat));
        let (control_pr, control_co) = RingBuffer::new(8);
        let volume = if let Some(fadein_param) = settings.fadein_param {
            Volume::new_with_fade(0.0, settings.volume.as_amplitude(), fadein_param)
        } else {
            Volume::new(settings.volume.as_amplitude())
        };

        let input = match (settings.start_time, settings.end_time) {
            (None, None) => InnerSource::Original(input),
            (None, Some(end)) => InnerSource::Taken(input.take_duration(Duration::from_secs_f64(end))),
            (Some(start), None) => InnerSource::Skipped(input.skip_duration(Duration::from_secs_f64(start))),
            (Some(start), Some(end)) => InnerSource::Ranged(input.take_duration(Duration::from_secs_f64(end)).skip_duration(Duration::from_secs_f64(start))),
        };

        (Self {
            input,
            shared: shared.clone(),
            control: control_co,
            settings,
            fadeout_param,
            current_channel: channels.get(),
            samples_counted: 0,
            offset_duration: 0.0,
            current_span_channels: channels,
            current_span_sample_rate: sample_rate,
            first_channel_sample: None,
            control_volume: Volume::new(1.0),
            volume,
        },
        AudioSourceHandle {
            shared,
            control: control_pr,
        })
    }
}

impl<I> Iterator for AudioSource<I> where I: Source + 'static {
    type Item = Sample;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut state = AudioPlaybackState::try_from(self.shared.state.load(Ordering::Acquire)).unwrap();
        
        if self.current_channel >= self.settings.output_channels {
            self.current_channel = 0;
            // Command Handling
            if let Ok(command) = self.control.pop() {
                match command {
                    AudioSourceControlCommand::Start => {
                        if matches!(state, AudioPlaybackState::Loaded) {
                            state = AudioPlaybackState::Playing;
                        }
                    },
                    AudioSourceControlCommand::Pause => {
                        if matches!(state, AudioPlaybackState::Playing | AudioPlaybackState::Resuming) {
                            state = AudioPlaybackState::Pausing;
                            self.control_volume.set_volume(0.0, DEFAULT_FADE_PARAM);
                        }
                    },
                    AudioSourceControlCommand::Resume => {
                        if matches!(state, AudioPlaybackState::Paused) {
                            state = AudioPlaybackState::Resuming;
                            self.control_volume.set_volume(1.0, DEFAULT_FADE_PARAM);
                        }
                    },
                    AudioSourceControlCommand::FadeOut => {
                        if matches!(state, AudioPlaybackState::Playing | AudioPlaybackState::Pausing | AudioPlaybackState::Paused | AudioPlaybackState::Resuming) {
                            if state == AudioPlaybackState::Paused {
                                state = AudioPlaybackState::Stopped;
                            } else {
                                state = AudioPlaybackState::SoftStopping;
                                self.control_volume.set_volume(0.0, self.fadeout_param);
                            }
                        }
                    },
                    AudioSourceControlCommand::Stop => {
                        if matches!(state, AudioPlaybackState::Playing | AudioPlaybackState::Pausing | AudioPlaybackState::Paused | AudioPlaybackState::Resuming | AudioPlaybackState::SoftStopping) {
                            if state == AudioPlaybackState::Paused {
                                state = AudioPlaybackState::Stopped;
                            } else {
                                state = AudioPlaybackState::HardStopping;
                                self.control_volume.set_volume(0.0, DEFAULT_FADE_PARAM);
                            }
                        }
                    },
                    AudioSourceControlCommand::Seek { position, result } => {
                        let _ = result.send(self.try_seek(Duration::from_secs_f64(position)));
                    },
                    AudioSourceControlCommand::SetVolume { volume, fade_param } => {
                        self.volume.set_volume(volume.as_amplitude(), fade_param);
                    }
                }
            }
            let dt = 1.0 / self.current_span_sample_rate.get() as f64;
            if self.control_volume.update(dt) {
                match state {
                    AudioPlaybackState::Pausing => {
                        state = AudioPlaybackState::Paused;
                    },
                    AudioPlaybackState::Resuming => {
                        state = AudioPlaybackState::Playing;
                    },
                    AudioPlaybackState::SoftStopping |
                    AudioPlaybackState::HardStopping => {
                        state = AudioPlaybackState::Stopped;
                    },
                    _ => {},
                }
            }
            self.volume.update(dt);
        }
        self.current_channel += 1;

        // return samples
        let sample = match state {
            AudioPlaybackState::Loaded => Some(0.0),
            AudioPlaybackState::Paused => Some(0.0),
            AudioPlaybackState::Stopped => None,
            AudioPlaybackState::Completed => None,
            _advancing => {
                let mut item;
                if self.current_channel == 0 {
                    if self.settings.output_channels < self.current_span_channels.get() {
                        for _ in self.settings.output_channels..self.current_span_channels.get() {
                            if self.input.next().is_none() {
                                break;
                            }
                        }
                        item = self.input.next();
                    } else if self.settings.output_channels > self.current_span_channels.get() {
                        item = self.input.next();
                        self.first_channel_sample = item;
                    } else {
                        item = self.input.next();
                    }
                } else if self.current_channel > self.current_span_channels.get() {
                    item = self.first_channel_sample;
                } else {
                    item = self.input.next();
                }
                let factor = self.control_volume.volume * self.volume.volume;

                if item.is_some() {
                    self.samples_counted += 1;
                    if Some(self.samples_counted) == self.current_span_len() {
                        self.offset_duration += self.samples_counted as f64
                            / self.current_span_sample_rate.get() as f64
                            / self.current_span_channels.get() as f64;

                        self.shared.position.store(self.offset_duration.to_bits(), Ordering::Release);

                        self.samples_counted = 0;

                        self.current_span_channels = self.channels();
                        self.current_span_sample_rate = self.sample_rate();
                    }
                } else if self.shared.repeat.load(Ordering::Acquire) {
                    let _ = self.try_seek(Duration::ZERO);
                    item = Some(0.0)
                } else {
                    state = AudioPlaybackState::Completed;
                }
                item.map(|s| s * factor)
            },
        };

        // State publish
        self.shared.state.store(state as u8, Ordering::Release);

        sample
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> Source for AudioSource<I> where I: Source + 'static {
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    #[inline]
    fn channels(&self) -> ChannelCount {
        NonZero::new(self.settings.output_channels).unwrap() as ChannelCount
    }

    #[inline]
    fn sample_rate(&self) -> SampleRate {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let result = self.input.try_seek(self.settings.start_time.map_or(pos, |st| Duration::from_secs_f64(st) + pos));
        if result.is_ok() {
            self.offset_duration = pos.as_secs_f64();
            self.shared.position.store(self.offset_duration.to_bits(), Ordering::Release);

            self.samples_counted = 0;
        }
        result
    }
}