use std::{
    f32::consts::SQRT_2,
    num::NonZero,
    ops::{Deref, DerefMut},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    },
    time::Duration,
};

use rodio::{
    ChannelCount, Sample, SampleRate, Source,
    source::{SeekError, TakeDuration},
};
use rtrb::{Consumer, Producer, RingBuffer};
use tokio::sync::oneshot;

use crate::{
    engine::audio_engine::AudioCommandData,
    model::cue::audio::{Decibels, Easing, FadeParam},
};

use super::lowcost_skip::SkipDuration;

const MAX_CHANNELS: u16 = 128;

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
        result: oneshot::Sender<Result<(), anyhow::Error>>,
    },
    SetVolume {
        volume: Decibels,
        fade_param: FadeParam,
    },
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
    pub duration: f64,
    volume: Decibels,
    fade_volume: Decibels,
}

impl AudioSourceHandle {
    pub fn state(&self) -> AudioPlaybackState {
        self.shared
            .state
            .load(Ordering::Acquire)
            .try_into()
            .unwrap()
    }

    pub fn position(&self) -> f64 {
        f64::from_bits(self.shared.position.load(Ordering::Acquire))
    }

    pub fn is_repeating(&self) -> bool {
        self.shared.repeat.load(Ordering::Acquire)
    }

    pub fn get_volume(&self) -> Decibels {
        self.volume
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
        let state = self.state();
        if state == AudioPlaybackState::Stopped || state == AudioPlaybackState::Completed {
            return;
        }
        if self.state().eq(&AudioPlaybackState::SoftStopping) {
            // Hard Stop
            let _ = self.control.push(AudioSourceControlCommand::Stop);
        } else {
            let _ = self.control.push(AudioSourceControlCommand::FadeOut);
        }
    }

    pub async fn seek_to(&mut self, position: f64) -> Result<f64, anyhow::Error> {
        let (result_tx, result_rx) = oneshot::channel();
        let position = position.clamp(0.0, self.duration);
        let _ = self.control.push(AudioSourceControlCommand::Seek {
            position,
            result: result_tx,
        });
        match result_rx.await {
            Ok(Ok(_)) => Ok(position),
            Ok(Err(err)) => Err(err),
            Err(err) => Err(anyhow::anyhow!("failed to retrieve seek result. {}", err)),
        }
    }

    pub async fn seek_by(&mut self, amount: f64) -> Result<f64, anyhow::Error> {
        let (result_tx, result_rx) = oneshot::channel();
        let position = (self.position() + amount).clamp(0.0, self.duration);
        let _ = self.control.push(AudioSourceControlCommand::Seek {
            position,
            result: result_tx,
        });
        match result_rx.await {
            Ok(Ok(_)) => Ok(position),
            Ok(Err(err)) => Err(err),
            Err(err) => Err(anyhow::anyhow!("failed to retrieve seek result. {}", err)),
        }
    }

    pub fn set_repeat(&self, value: bool) {
        self.shared.repeat.store(value, Ordering::Release);
    }

    pub fn set_volume(&mut self, volume: Decibels) {
        self.volume = volume;

        let _ = self.control.push(AudioSourceControlCommand::SetVolume {
            volume: self.volume + self.fade_volume,
            fade_param: DEFAULT_FADE_PARAM,
        });
    }

    pub fn set_fade(&mut self, volume: Decibels, fade_param: FadeParam) {
        self.fade_volume = volume;

        let _ = self.control.push(AudioSourceControlCommand::SetVolume {
            volume: self.volume + self.fade_volume,
            fade_param,
        });
    }
}

struct VolumeFadeInfo {
    pub from: Decibels,
    pub to: Decibels,
    pub elapsed: f64,
    pub fade_param: FadeParam,
}

struct Volume {
    pub volume: Decibels,
    pub fade_info: Option<VolumeFadeInfo>,
}

impl Volume {
    pub fn new(volume: Decibels) -> Self {
        Self {
            volume,
            fade_info: None,
        }
    }

    pub fn new_with_fade(init: Decibels, target: Decibels, fade_param: FadeParam) -> Self {
        Self {
            volume: init,
            fade_info: Some(VolumeFadeInfo {
                from: init,
                to: target,
                elapsed: 0.0,
                fade_param,
            }),
        }
    }

    pub fn set_volume(&mut self, volume: Decibels, fade_param: FadeParam) {
        if let Some(info) = self.fade_info.as_mut() {
            info.from = self.volume;
            info.to = volume;
            info.elapsed = 0.0;
            info.fade_param = fade_param;
        } else {
            self.fade_info = Some(VolumeFadeInfo {
                from: self.volume,
                to: volume,
                elapsed: 0.0,
                fade_param,
            });
        }
    }

    pub fn update(&mut self, dt: f64) -> bool {
        if let Some(info) = self.fade_info.as_mut() {
            if info.elapsed >= info.fade_param.duration {
                self.volume = info.to;
                return true;
            }

            let progress =
                info.fade_param
                    .easing
                    .get_factor(info.elapsed / info.fade_param.duration) as f32;

            self.volume = info.from + (info.to - info.from) * progress;

            info.elapsed += dt;
        }
        false
    }
}

pub struct ChannelMapping {
    input_channels: usize,
    output_channels: usize,
    map: Box<[f32]>,
}

impl ChannelMapping {
    pub fn auto_map(in_n: usize, out_n: usize) -> Self {
        let mut map = vec![0.0; in_n * out_n].into_boxed_slice();
        match (in_n, out_n) {
            (1, 2) => {
                map[0] = 1.0;
                map[1] = 1.0;
            }
            (2, 1) => {
                map[0] = 0.5;
                map[1] = 0.5;
            }
            (in_n, out_n) if in_n == out_n => {
                for i in 0..out_n {
                    map[i * out_n + i] = 1.0;
                }
            }
            (in_n, out_n) => {
                for i in 0..in_n.min(out_n) {
                    map[i * out_n + i] = 1.0;
                }
            }
        }
        Self {
            input_channels: in_n,
            output_channels: out_n,
            map,
        }
    }

    /// generate channel mapping from pannig (-1.0..1.0 mapped to L..R). other channels are ignored.
    pub fn from_pan(pan: f32) -> Self {
        let mut map = vec![0.0; 4].into_boxed_slice();
        let pan = pan.clamp(-1.0, 1.0);
        let right_amount = (pan + 1.0) * 0.5;
        map[0] = (1.0 - right_amount).sqrt() * SQRT_2;
        map[3] = right_amount.sqrt() * SQRT_2;
        Self {
            input_channels: 2,
            output_channels: 2,
            map,
        }
    }

    pub fn get_factor(&self, in_n: usize, out_n: usize) -> f32 {
        if in_n < self.input_channels && out_n < self.output_channels {
            self.map[in_n * self.output_channels + out_n]
        } else {
            0.0
        }
    }
}

pub struct AudioSourceSettings {
    pub repeat: bool,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub fadeout_param: Option<FadeParam>,
    pub fadein_param: Option<FadeParam>,
    pub volume: Decibels,
    pub channel_mapping: ChannelMapping,
}

impl From<&AudioCommandData> for AudioSourceSettings {
    fn from(value: &AudioCommandData) -> Self {
        Self {
            repeat: value.repeat,
            start_time: value.start_time,
            end_time: value.end_time,
            fadeout_param: value.fade_out_param,
            fadein_param: value.fade_in_param,
            volume: value.volume,
            channel_mapping: ChannelMapping::auto_map(2, 2),
        }
    }
}

enum InnerSource<I> {
    Original(I),
    Skipped(SkipDuration<I>),
    Taken(TakeDuration<I>),
    Ranged(SkipDuration<TakeDuration<I>>),
}

impl<I> Deref for InnerSource<I>
where
    I: Source + 'static,
{
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

impl<I> DerefMut for InnerSource<I>
where
    I: Source + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            InnerSource::Original(inner) => inner as &mut dyn Source,
            InnerSource::Skipped(skip_duration) => skip_duration as &mut dyn Source,
            InnerSource::Taken(take_duration) => take_duration as &mut dyn Source,
            InnerSource::Ranged(skip_duration) => skip_duration as &mut dyn Source,
        }
    }
}

pub struct AudioSource<I>
where
    I: Source,
{
    input: InnerSource<I>,
    shared: Arc<AudioSourceShared>,
    control: Consumer<AudioSourceControlCommand>,
    settings: AudioSourceSettings,
    fadeout_param: FadeParam,
    current_channel: u16,
    offset_position: f64,
    frames_counted: usize,
    playing_frames_counted: usize,
    update_interval: usize,
    current_span_channels: ChannelCount,
    current_span_sample_rate: SampleRate,
    control_volume: Volume,
    volume: Volume,
    output_buffer: Box<[Sample]>,
}

impl<I> AudioSource<I>
where
    I: Source + 'static,
{
    pub fn new(input: I, settings: AudioSourceSettings) -> (Self, AudioSourceHandle)
    where
        I: Source,
    {
        let channels = input.channels();
        let sample_rate = input.sample_rate();
        let fadeout_param = settings.fadeout_param.unwrap_or(DEFAULT_FADE_PARAM);
        let shared = Arc::new(AudioSourceShared::new(settings.repeat));
        let (control_pr, control_co) = RingBuffer::new(8);
        let control_volume = if let Some(fadein_param) = settings.fadein_param {
            Volume::new_with_fade(Decibels::MUTE, Decibels::IDENTITY, fadein_param)
        } else {
            Volume::new(Decibels::IDENTITY)
        };
        let volume_db = settings.volume;
        let output_buffer = vec![0.0; settings.channel_mapping.output_channels].into_boxed_slice();
        let update_interval = Self::calculate_interval(&sample_rate);

        let input = match (settings.start_time, settings.end_time) {
            (None, None) => InnerSource::Original(input),
            (None, Some(end)) => {
                InnerSource::Taken(input.take_duration(Duration::from_secs_f64(end)))
            }
            (Some(start), None) => {
                InnerSource::Skipped(SkipDuration::new(input, Duration::from_secs_f64(start)))
            }
            (Some(start), Some(end)) => InnerSource::Ranged(SkipDuration::new(
                input.take_duration(Duration::from_secs_f64(end)),
                Duration::from_secs_f64(start),
            )),
        };

        let duration = input
            .total_duration()
            .map(|duration| duration.as_secs_f64())
            .unwrap_or(0.0);

        (
            Self {
                input,
                shared: shared.clone(),
                control: control_co,
                settings,
                fadeout_param,
                current_channel: channels.get(),
                output_buffer,
                current_span_channels: channels,
                current_span_sample_rate: sample_rate,
                offset_position: 0.0,
                frames_counted: update_interval,
                playing_frames_counted: 0,
                update_interval,
                control_volume,
                volume: Volume::new(volume_db),
            },
            AudioSourceHandle {
                shared,
                control: control_pr,
                duration,
                volume: volume_db,
                fade_volume: Decibels::IDENTITY,
            },
        )
    }

    fn calculate_interval(sample_rate: &NonZero<u32>) -> usize {
        sample_rate.get() as usize / 1_000
    }

    fn is_advancing(state: AudioPlaybackState) -> bool {
        match state {
            AudioPlaybackState::Loaded
            | AudioPlaybackState::Paused
            | AudioPlaybackState::Stopped
            | AudioPlaybackState::Completed => false,
            AudioPlaybackState::Playing
            | AudioPlaybackState::Pausing
            | AudioPlaybackState::Resuming
            | AudioPlaybackState::SoftStopping
            | AudioPlaybackState::HardStopping => true,
        }
    }
}

impl<I> Iterator for AudioSource<I>
where
    I: Source + 'static,
{
    type Item = Sample;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut state =
            AudioPlaybackState::try_from(self.shared.state.load(Ordering::Acquire)).unwrap();

        if self.current_channel >= self.settings.channel_mapping.output_channels as u16 {
            self.current_channel = 0;

            if self.frames_counted >= self.update_interval {
                self.frames_counted = 0;

                if Self::is_advancing(state) {
                    self.shared.position.store(
                        (self.offset_position
                            + self.playing_frames_counted as f64
                                / self.current_span_sample_rate.get() as f64)
                            .to_bits(),
                        Ordering::Release,
                    );
                }

                // Command Handling
                if let Ok(command) = self.control.pop() {
                    match command {
                        AudioSourceControlCommand::Start => {
                            if matches!(state, AudioPlaybackState::Loaded) {
                                state = AudioPlaybackState::Playing;
                            }
                        }
                        AudioSourceControlCommand::Pause => {
                            if matches!(
                                state,
                                AudioPlaybackState::Playing | AudioPlaybackState::Resuming
                            ) {
                                state = AudioPlaybackState::Pausing;
                                self.control_volume
                                    .set_volume(Decibels::MUTE, DEFAULT_FADE_PARAM);
                            }
                        }
                        AudioSourceControlCommand::Resume => {
                            if matches!(state, AudioPlaybackState::Paused) {
                                state = AudioPlaybackState::Resuming;
                                self.control_volume
                                    .set_volume(Decibels::IDENTITY, DEFAULT_FADE_PARAM);
                            }
                        }
                        AudioSourceControlCommand::FadeOut => {
                            if matches!(
                                state,
                                AudioPlaybackState::Playing
                                    | AudioPlaybackState::Pausing
                                    | AudioPlaybackState::Paused
                                    | AudioPlaybackState::Resuming
                            ) {
                                if state == AudioPlaybackState::Paused {
                                    state = AudioPlaybackState::Stopped;
                                } else {
                                    state = AudioPlaybackState::SoftStopping;
                                    self.control_volume
                                        .set_volume(Decibels::MUTE, self.fadeout_param);
                                }
                            }
                        }
                        AudioSourceControlCommand::Stop => {
                            if matches!(
                                state,
                                AudioPlaybackState::Playing
                                    | AudioPlaybackState::Pausing
                                    | AudioPlaybackState::Paused
                                    | AudioPlaybackState::Resuming
                                    | AudioPlaybackState::SoftStopping
                            ) {
                                if state == AudioPlaybackState::Paused {
                                    state = AudioPlaybackState::Stopped;
                                } else {
                                    state = AudioPlaybackState::HardStopping;
                                    self.control_volume
                                        .set_volume(Decibels::MUTE, DEFAULT_FADE_PARAM);
                                }
                            }
                        }
                        AudioSourceControlCommand::Seek { position, result } => {
                            let _ = result.send(match Duration::try_from_secs_f64(position) {
                                Ok(duration) => self.try_seek(duration).map_err(|err| err.into()),
                                Err(err) => Err(anyhow::anyhow!("Invalid position. {}", err)),
                            });
                        }
                        AudioSourceControlCommand::SetVolume { volume, fade_param } => {
                            self.volume.set_volume(volume, fade_param);
                        }
                    }

                    // State publish
                    self.shared.state.store(state as u8, Ordering::Release);
                }
            }

            let dt = 1.0 / self.current_span_sample_rate.get() as f64;
            if self.control_volume.update(dt) {
                match state {
                    AudioPlaybackState::Pausing => {
                        state = AudioPlaybackState::Paused;
                    }
                    AudioPlaybackState::Resuming => {
                        state = AudioPlaybackState::Playing;
                    }
                    AudioPlaybackState::SoftStopping | AudioPlaybackState::HardStopping => {
                        state = AudioPlaybackState::Stopped;
                    }
                    _ => {}
                }
                // State publish
                self.shared.state.store(state as u8, Ordering::Release);
            }
            self.volume.update(dt);

            if Self::is_advancing(state) {
                let factor = self.control_volume.volume + self.volume.volume;

                let mut completed = false;
                let mut inputs = [0.0; MAX_CHANNELS as usize];
                for i in 0..self.current_span_channels.get().min(MAX_CHANNELS) {
                    let sample = self.input.next();
                    if let Some(s) = sample {
                        inputs[i as usize] = s;
                    } else {
                        completed = true;
                        break;
                    }
                }

                if !completed {
                    for out_n in 0..self.settings.channel_mapping.output_channels {
                        let mut out = 0.0;
                        for (in_n, src) in inputs
                            .iter()
                            .take(self.settings.channel_mapping.input_channels)
                            .enumerate()
                        {
                            out += self.settings.channel_mapping.get_factor(in_n, out_n) * src;
                        }
                        self.output_buffer[out_n] = out * factor.as_amplitude();
                    }
                } else if self.shared.repeat.load(Ordering::Acquire) {
                    let _ = self.try_seek(Duration::ZERO);
                } else {
                    state = match state {
                        AudioPlaybackState::SoftStopping | AudioPlaybackState::HardStopping => {
                            AudioPlaybackState::Stopped
                        }
                        _ => AudioPlaybackState::Completed,
                    };
                    // State publish
                    self.shared.state.store(state as u8, Ordering::Release);
                };

                self.playing_frames_counted += 1;
            }

            self.frames_counted += 1;
        }

        // return samples
        let sample = match state {
            AudioPlaybackState::Loaded | AudioPlaybackState::Paused => Some(0.0),
            AudioPlaybackState::Stopped | AudioPlaybackState::Completed => None,
            _advancing => Some(self.output_buffer[self.current_channel as usize]),
        };

        self.current_channel += 1;

        sample
    }
}

impl<I> Source for AudioSource<I>
where
    I: Source + 'static,
{
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> ChannelCount {
        NonZero::new(self.settings.channel_mapping.output_channels as u16).unwrap() as ChannelCount
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
        let result = self.input.try_seek(pos);
        if result.is_ok() {
            self.offset_position = pos.as_secs_f64();
            self.shared
                .position
                .store(self.offset_position.to_bits(), Ordering::Release);
            self.playing_frames_counted = 0;
            self.frames_counted = 0;
            self.current_channel = 0;
        }
        result
    }
}
