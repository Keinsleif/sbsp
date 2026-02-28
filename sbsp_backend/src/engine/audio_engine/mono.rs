use std::{sync::{Arc, atomic::{AtomicBool, Ordering}},  time::Duration};

use rodio::{ChannelCount, Sample, SampleRate, Source, source::SeekError};

#[derive(Clone, Debug)]
pub struct Mono<I>
where
    I: Source,
{
    input: I,
    is_enabled: Arc<AtomicBool>,
    current_channel: u16,
    current_sample: Option<Sample>,
}

impl<I> Mono<I>
where
    I: Source,
{
    pub fn new(input: I, is_enabled: Arc<AtomicBool>) -> Self {
        let channels = input.channels().get();
        Self {
            input,
            is_enabled,
            current_channel: channels,
            current_sample: None,
        }
    }
}

impl<I> Iterator for Mono<I>
where
    I: Source,
{
    type Item = Sample;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let channels = self.input.channels().get();
        if self.is_enabled.load(Ordering::Acquire) {
            if self.current_channel >= channels {
                self.current_channel = 0;
                self.current_sample = None;
                for _ in 0..channels {
                    if let Some(s) = self.input.next() {
                        self.current_sample = Some(self.current_sample.unwrap_or(0.0) + s);
                    }
                }
                self.current_sample.map(|s| s / channels as Self::Item);
            }
            self.current_channel += 1;
            self.current_sample
        } else if self.current_channel < channels {
            self.current_channel += 1;
            self.current_sample
        } else {
            self.input.next()
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for Mono<I> where I: Source + ExactSizeIterator {}

impl<I> Source for Mono<I>
where
    I: Source,
{
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    #[inline]
    fn channels(&self) -> ChannelCount {
        self.input.channels() as ChannelCount
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
        self.input.try_seek(pos)
    }
}