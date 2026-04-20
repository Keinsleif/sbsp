use crate::model::cue::audio::{Decibels, EnvelopeParam};

pub struct Envelope {
    segments: Vec<EnvelopeParam>,
    current_idx: usize,
    duration: f64,
}

impl Envelope {
    pub fn new(mut segments: Vec<EnvelopeParam>, duration: f64) -> Self {
        segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        Self {
            segments,
            current_idx: 0,
            duration,
        }
    }

    pub fn update(&mut self, pos: f64) -> Decibels {
        let pos = pos / self.duration;
        let n = self.segments.len();

        if n == 0 {
            return Decibels::IDENTITY;
        }

        if n == 1 {
            return self.segments[0].volume;
        }

        while self.current_idx < n - 1 && pos >= self.segments[self.current_idx + 1].start {
            self.current_idx += 1;
        }

        let seg = &self.segments[self.current_idx];

        if pos >= seg.start && pos <= seg.end {
            return seg.volume;
        }

        if self.current_idx == 0 && pos < seg.start {
            return seg.volume;
        }

        if self.current_idx < n - 1 {
            let next = &self.segments[self.current_idx + 1];
            
            if pos > seg.end {
                let gap_duration = next.start - seg.end;
                if gap_duration > 0.0 {
                    let t = (pos - seg.end) / gap_duration;
                    return seg.volume + (next.volume - seg.volume) * t as f32;
                }
                return next.volume;
            }
        }

        seg.volume
    }

    pub fn seek(&mut self, pos: f64) {
        if self.segments.is_empty() {
            return;
        }
        let pos = pos / self.duration;

        self.current_idx = match self.segments.binary_search_by(|s| s.start.partial_cmp(&pos).unwrap()) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
    }
}
