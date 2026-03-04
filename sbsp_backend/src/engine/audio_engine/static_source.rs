use std::time::Duration;

use rodio::{ChannelCount, Sample, SampleRate, Source, source::SeekError};

use crate::engine::audio_engine::NANOS_PER_SEC;

/// A buffer of samples treated as a source.
#[derive(Clone)]
pub struct StaticSource {
    data: Box<[f32]>,
    index: usize,
    channels: ChannelCount,
    sample_rate: SampleRate,
    duration: Duration,
}

impl StaticSource {
    /// Builds a new `StaticSamplesBuffer`.
    ///
    /// # Panic
    ///
    /// - Panics if the number of channels is zero.
    /// - Panics if the samples rate is zero.
    /// - Panics if the length of the buffer is larger than approximately 16 billion elements.
    ///   This is because the calculation of the duration would overflow.
    ///
    pub fn new<I>(input: I) -> StaticSource
    where
        I: Source,
    {
        let channels = input.channels();
        let sample_rate = input.sample_rate();
        let data: Box<[f32]> = input.collect();
        let duration_ns = NANOS_PER_SEC.checked_mul(data.len() as u64).unwrap()
            / sample_rate.get() as u64
            / channels.get() as u64;
        let duration = Duration::new(
            duration_ns / NANOS_PER_SEC,
            (duration_ns % NANOS_PER_SEC) as u32,
        );

        StaticSource {
            data,
            index: 0,
            channels,
            sample_rate,
            duration,
        }
    }
}

impl Source for StaticSource {
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        Some(self.data.len())
    }

    #[inline]
    fn channels(&self) -> ChannelCount {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        Some(self.duration)
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let pos = pos.min(self.duration);
        let target_sample = pos.as_nanos() as usize
            * self.channels.get() as usize
            * self.sample_rate.get() as usize
            / NANOS_PER_SEC as usize;
        self.index = target_sample;
        Ok(())
    }
}

impl Iterator for StaticSource {
    type Item = Sample;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.data.get(self.index).cloned();
        self.index += 1;
        sample
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.data.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}
