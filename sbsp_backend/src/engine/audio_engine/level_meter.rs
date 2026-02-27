use core::f32;
use std::{sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
}, time::Duration};

use rodio::{ChannelCount, SampleRate, Source, source::SeekError};

#[derive(Clone, Default)]
pub struct SharedLevel {
    pub left: Arc<AtomicU32>,
    pub right: Arc<AtomicU32>,
}

impl SharedLevel {
    pub fn set(&self, left: f32, right: f32) {
        self.left.store(left.to_bits(), Ordering::Relaxed);
        self.right.store(right.to_bits(), Ordering::Relaxed);
    }

    pub fn set_left(&self, left: f32) {
        self.left.store(left.to_bits(), Ordering::Relaxed);
    }

    pub fn set_right(&self, right: f32) {
        self.right.store(right.to_bits(), Ordering::Relaxed);
    }

    pub fn get(&self) -> (f32, f32) {
        let l = f32::from_bits(self.left.load(Ordering::Relaxed));
        let r = f32::from_bits(self.right.load(Ordering::Relaxed));
        (l, r)
    }
}

#[derive(Clone)]
pub struct LevelMeter<I> {
    input: I,
    shared: SharedLevel,
    current_channel: u16,
}

impl<I> LevelMeter<I>
where 
    I: Source,
{
    pub fn new(input: I, shared: SharedLevel) -> Self {
        let channels = input.channels().get();
        Self { input, shared, current_channel: channels }
    }

    /// Returns a reference to the inner source.
    #[inline]
    pub fn inner(&self) -> &I {
        &self.input
    }

    /// Returns a mutable reference to the inner source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Returns the inner source.
    #[inline]
    pub fn into_inner(self) -> I {
        self.input
    }
}

impl<I> Iterator for LevelMeter<I> 
where
    I: Source,
{
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        let channels = self.input.channels().get();
        let sample = self.input.next();
        if self.current_channel >= channels {
            self.current_channel = 0;
            if let Some(s) = sample {
                if self.current_channel == 0 {
                    self.shared.set_left(s as f32);
                } else if self.current_channel == 1 {
                    self.shared.set_right(s as f32);
                }
            }
        }
        self.current_channel += 1;
        sample
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> Source for LevelMeter<I>
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
