use std::time::Duration;

use rodio::{ChannelCount, SampleRate, Source, source::SeekError};

use crate::engine::audio_engine::NANOS_PER_SEC;

#[derive(Clone, Debug)]
pub struct SkipDuration<I> {
    input: I,
    skipped_duration: Duration,
}

impl<I> SkipDuration<I>
where
    I: Source,
{
    pub fn new(mut input: I, duration: Duration) -> Self
    where
        I: Source,
    {
        if input.try_seek(duration).is_err() {
            // fallback to original implementation
            Self::do_skip_duration(&mut input, duration);
        }
        SkipDuration {
            input,
            skipped_duration: duration,
        }
    }

    fn do_skip_duration(input: &mut I, mut duration: Duration)
    where
        I: Source,
    {
        while duration > Duration::ZERO {
            if input.current_span_len().is_none() {
                Self::do_skip_duration_unchecked(input, duration);
                return;
            }

            let span_len: usize = input.current_span_len().unwrap();
            if span_len == 0 {
                return;
            }

            let sample_rate = input.sample_rate().get() as u128;
            let channels = input.channels().get() as u128;

            let samples_per_channel = duration.as_nanos() * sample_rate / NANOS_PER_SEC as u128;
            let samples_to_skip: u128 = samples_per_channel * channels;

            if span_len as u128 > samples_to_skip {
                Self::skip_samples(input, samples_to_skip as usize);
                return;
            }

            duration -= Duration::from_nanos(
                (NANOS_PER_SEC as u128 * span_len as u128 / channels / sample_rate) as u64,
            );
            Self::skip_samples(input, span_len);
        }
    }

    fn do_skip_duration_unchecked(input: &mut I, duration: Duration)
    where
        I: Source,
    {
        let samples_per_channel: u128 =
            duration.as_nanos() * input.sample_rate().get() as u128 / NANOS_PER_SEC as u128;
        let samples_to_skip: u128 = samples_per_channel * input.channels().get() as u128;

        Self::skip_samples(input, samples_to_skip as usize);
    }

    fn skip_samples(input: &mut I, n: usize)
    where
        I: Source,
    {
        for _ in 0..n {
            if input.next().is_none() {
                break;
            }
        }
    }
}

impl<I> Iterator for SkipDuration<I>
where
    I: Source,
{
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.input.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> Source for SkipDuration<I>
where
    I: Source,
{
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    #[inline]
    fn channels(&self) -> ChannelCount {
        self.input.channels()
    }

    #[inline]
    fn sample_rate(&self) -> SampleRate {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration().map(|val| {
            val.checked_sub(self.skipped_duration)
                .unwrap_or_else(|| Duration::from_secs(0))
        })
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.input.try_seek(pos + self.skipped_duration)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rodio::Sample;
    use rodio::buffer::SamplesBuffer;
    use rodio::math::nz;
    use rodio::source::Source;
    use rodio::{ChannelCount, SampleRate};

    fn test_skip_duration_samples_left(
        channels: ChannelCount,
        sample_rate: SampleRate,
        seconds: u32,
        seconds_to_skip: u32,
    ) {
        let buf_len = (sample_rate.get() * channels.get() as u32 * seconds) as usize;
        assert!(buf_len < 10 * 1024 * 1024);
        let data: Vec<Sample> = vec![0.0; buf_len];
        let test_buffer = SamplesBuffer::new(channels, sample_rate, data);
        let seconds_left = seconds.saturating_sub(seconds_to_skip);

        let samples_left_expected =
            (sample_rate.get() * channels.get() as u32 * seconds_left) as usize;
        let samples_left = test_buffer
            .skip_duration(Duration::from_secs(seconds_to_skip as u64))
            .count();

        assert_eq!(samples_left, samples_left_expected);
    }

    macro_rules! skip_duration_test_block {
        ($(channels: $ch:expr, sample rate: $sr:expr, seconds: $sec:expr, seconds to skip: $sec_to_skip:expr;)+) => {
            $(
                test_skip_duration_samples_left(nz!($ch), nz!($sr), $sec, $sec_to_skip);
            )+
        }
    }

    #[test]
    fn skip_duration_shorter_than_source() {
        skip_duration_test_block! {
            channels: 1, sample rate: 44100, seconds: 5, seconds to skip: 3;
            channels: 1, sample rate: 96000, seconds: 5, seconds to skip: 3;

            channels: 2, sample rate: 44100, seconds: 5, seconds to skip: 3;
            channels: 2, sample rate: 96000, seconds: 5, seconds to skip: 3;

            channels: 4, sample rate: 44100, seconds: 5, seconds to skip: 3;
            channels: 4, sample rate: 96000, seconds: 5, seconds to skip: 3;
        }
    }

    #[test]
    fn skip_duration_zero_duration() {
        skip_duration_test_block! {
            channels: 1, sample rate: 44100, seconds: 5, seconds to skip: 0;
            channels: 1, sample rate: 96000, seconds: 5, seconds to skip: 0;

            channels: 2, sample rate: 44100, seconds: 5, seconds to skip: 0;
            channels: 2, sample rate: 96000, seconds: 5, seconds to skip: 0;

            channels: 4, sample rate: 44100, seconds: 5, seconds to skip: 0;
            channels: 4, sample rate: 96000, seconds: 5, seconds to skip: 0;
        }
    }

    #[test]
    fn skip_duration_longer_than_source() {
        skip_duration_test_block! {
            channels: 1, sample rate: 44100, seconds: 1, seconds to skip: 5;
            channels: 1, sample rate: 96000, seconds: 10, seconds to skip: 11;

            channels: 2, sample rate: 44100, seconds: 1, seconds to skip: 5;
            channels: 2, sample rate: 96000, seconds: 10, seconds to skip: 11;

            channels: 4, sample rate: 44100, seconds: 1, seconds to skip: 5;
            channels: 4, sample rate: 96000, seconds: 10, seconds to skip: 11;
        }
    }

    #[test]
    fn skip_duration_equal_to_source_length() {
        skip_duration_test_block! {
            channels: 1, sample rate: 44100, seconds: 1, seconds to skip: 1;
            channels: 1, sample rate: 96000, seconds: 10, seconds to skip: 10;

            channels: 2, sample rate: 44100, seconds: 1, seconds to skip: 1;
            channels: 2, sample rate: 96000, seconds: 10, seconds to skip: 10;

            channels: 4, sample rate: 44100, seconds: 1, seconds to skip: 1;
            channels: 4, sample rate: 96000, seconds: 10, seconds to skip: 10;
        }
    }
}
