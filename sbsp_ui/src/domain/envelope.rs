// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The volume envelope of an audio cue, as the waveform editor edits it.
//! Ported from `WaveformEditor.vue`.
//!
//! An envelope is a list of segments, each holding one volume (dB) for a
//! stretch of the played part of the file. Positions are fractions of that
//! played part (0 is the cue's start time, 1 its end time), so moving the
//! start or end marker does not disturb the envelope. The first segment
//! starts at 0 and the last ends at 1; between segments there may be gaps,
//! over which the backend ramps.

use sbsp_backend::model::cue::audio::{Decibels, EnvelopeSegment};

/// The least distance between two edges.
pub const MIN_GAP: f64 = 0.005;

/// The lowest volume the editor draws and sets.
pub const MIN_DB: f64 = -60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    /// dB.
    pub volume: f64,
}

impl From<&EnvelopeSegment> for Segment {
    fn from(segment: &EnvelopeSegment) -> Self {
        Self {
            start: segment.start,
            end: segment.end,
            volume: f64::from(f32::from(segment.volume)),
        }
    }
}

impl From<Segment> for EnvelopeSegment {
    fn from(segment: Segment) -> Self {
        Self {
            start: segment.start,
            end: segment.end,
            volume: Decibels::from(segment.volume as f32),
        }
    }
}

/// Sorted by start, the first starting at 0 and the last ending at 1.
pub fn normalize(segments: &[EnvelopeSegment]) -> Vec<Segment> {
    let mut result: Vec<Segment> = segments.iter().map(Segment::from).collect();
    result.sort_by(|a, b| a.start.total_cmp(&b.start));
    if let Some(first) = result.first_mut() {
        first.start = 0.0;
    }
    if let Some(last) = result.last_mut() {
        last.end = 1.0;
    }
    result
}

/// How far down a volume is drawn: 0 dB at the top, `MIN_DB` at the bottom,
/// evenly in between (in dB, so each step of the pointer is the same step of
/// loudness).
pub fn volume_to_y(volume: f64, height: f64) -> f64 {
    (volume.clamp(MIN_DB, 0.0) / MIN_DB * height).clamp(0.0, height)
}

pub fn y_to_volume(y: f64, height: f64) -> f64 {
    if height <= 0.0 {
        return 0.0;
    }
    (y / height * MIN_DB).clamp(MIN_DB, 0.0)
}

pub fn set_volume(segments: &[Segment], index: usize, volume: f64) -> Vec<Segment> {
    let mut result = segments.to_vec();
    if let Some(segment) = result.get_mut(index) {
        segment.volume = volume.clamp(MIN_DB, 0.0);
    }
    result
}

/// Moves the start of a segment (not the first one's: it stays at 0).
pub fn move_start(segments: &[Segment], index: usize, x: f64) -> Vec<Segment> {
    let mut result = segments.to_vec();
    if index == 0 {
        return result;
    }
    let Some(current) = result.get(index).copied() else {
        return result;
    };
    let min = result
        .get(index - 1)
        .map_or(0.0, |previous| previous.end + MIN_GAP);
    let max = current.end - MIN_GAP;
    if min <= max {
        result[index].start = x.clamp(min, max);
    }
    result
}

/// Moves the end of a segment (not the last one's: it stays at 1).
pub fn move_end(segments: &[Segment], index: usize, x: f64) -> Vec<Segment> {
    let mut result = segments.to_vec();
    if index + 1 >= result.len() {
        return result;
    }
    let current = result[index];
    let min = current.start + MIN_GAP;
    let max = result[index + 1].start - MIN_GAP;
    if min <= max {
        result[index].end = x.clamp(min, max);
    }
    result
}

/// A double click at `x`: the first segment of an empty envelope, a split
/// of the segment under the pointer, or a new segment in a gap. `None` when
/// there is no room.
pub fn add_or_split(segments: &[Segment], x: f64) -> Option<Vec<Segment>> {
    if segments.is_empty() {
        return Some(vec![Segment {
            start: 0.0,
            end: 1.0,
            volume: 0.0,
        }]);
    }

    let mut result = segments.to_vec();

    if let Some(index) = result.iter().position(|s| x >= s.start && x <= s.end) {
        let target = result[index];
        if target.end - target.start > MIN_GAP * 3.0 {
            let at = x.clamp(target.start + MIN_GAP, target.end - MIN_GAP);
            result[index].end = at - MIN_GAP / 2.0;
            result.insert(
                index + 1,
                Segment {
                    start: at + MIN_GAP / 2.0,
                    end: target.end,
                    volume: target.volume,
                },
            );
            return Some(result);
        }
    }

    for i in 0..result.len() - 1 {
        let gap_start = result[i].end;
        let gap_end = result[i + 1].start;
        if x > gap_start && x < gap_end && gap_end - gap_start > MIN_GAP * 3.0 {
            let start = (gap_start + MIN_GAP).max(x - 0.04);
            let end = (gap_end - MIN_GAP).min(x + 0.04);
            result.insert(
                i + 1,
                Segment {
                    start,
                    end,
                    volume: 0.0,
                },
            );
            return Some(result);
        }
    }
    None
}

/// What the "+" button does: starts an envelope, or splits the longest
/// segment in the middle, so it has room to act as long as there is any.
pub fn add_in_longest(segments: &[Segment]) -> Option<Vec<Segment>> {
    let Some(longest) = segments
        .iter()
        .max_by(|a, b| (a.end - a.start).total_cmp(&(b.end - b.start)))
    else {
        return add_or_split(segments, 0.5);
    };
    add_or_split(segments, (longest.start + longest.end) / 2.0)
}

/// Whether two envelopes are the same, allowing for the volume having been
/// through the backend's `f32`.
pub fn same(a: &[Segment], b: &[Segment]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            (a.start - b.start).abs() < 1e-9
                && (a.end - b.end).abs() < 1e-9
                && (a.volume - b.volume).abs() < 1e-3
        })
}

/// Removes a segment; the neighbour of an end segment grows to the edge.
pub fn remove(segments: &[Segment], index: usize) -> Vec<Segment> {
    let mut result = segments.to_vec();
    if index >= result.len() {
        return result;
    }
    result.remove(index);
    if index == 0 {
        if let Some(first) = result.first_mut() {
            first.start = 0.0;
        }
    } else if index == result.len()
        && let Some(last) = result.last_mut() {
            last.end = 1.0;
        }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: f64, end: f64, volume: f64) -> Segment {
        Segment { start, end, volume }
    }

    #[test]
    fn normalizing_sorts_and_pins_the_ends() {
        let raw = [
            EnvelopeSegment {
                start: 0.6,
                end: 0.9,
                volume: Decibels::from(-6.0),
            },
            EnvelopeSegment {
                start: 0.1,
                end: 0.4,
                volume: Decibels::from(0.0),
            },
        ];
        let result = normalize(&raw);
        assert_eq!(result, vec![seg(0.0, 0.4, 0.0), seg(0.6, 1.0, -6.0)]);
        assert!(normalize(&[]).is_empty());
    }

    #[test]
    fn volume_and_y_map_back_and_forth() {
        assert_eq!(volume_to_y(0.0, 100.0), 0.0);
        assert!((volume_to_y(-200.0, 100.0) - 100.0).abs() < 1e-6, "very quiet is the bottom");
        for volume in [-3.0, -12.0, -30.0] {
            let y = volume_to_y(volume, 120.0);
            assert!((y_to_volume(y, 120.0) - volume).abs() < 1e-9, "{volume}");
        }
        // The bottom edge is silence; above the top clamps to 0 dB.
        assert_eq!(y_to_volume(120.0, 120.0), MIN_DB);
        assert_eq!(y_to_volume(-10.0, 120.0), 0.0);
        assert_eq!(volume_to_y(6.0, 100.0), 0.0, "above 0 dB is the top");
    }

    #[test]
    fn a_volume_stays_between_silence_and_0_db() {
        let segments = [seg(0.0, 1.0, -3.0)];
        assert_eq!(set_volume(&segments, 0, -100.0)[0].volume, MIN_DB);
        assert_eq!(set_volume(&segments, 0, 5.0)[0].volume, 0.0);
        assert_eq!(set_volume(&segments, 3, -6.0), segments.to_vec());
    }

    #[test]
    fn edges_stay_between_their_neighbours() {
        let segments = [seg(0.0, 0.3, 0.0), seg(0.5, 0.8, 0.0), seg(0.9, 1.0, 0.0)];

        let moved = move_start(&segments, 1, 0.1);
        assert!((moved[1].start - (0.3 + MIN_GAP)).abs() < 1e-12);
        let moved = move_start(&segments, 1, 0.799);
        assert!((moved[1].start - (0.8 - MIN_GAP)).abs() < 1e-12);
        assert_eq!(move_start(&segments, 0, 0.2), segments.to_vec(), "the first start is pinned");

        let moved = move_end(&segments, 1, 0.95);
        assert!((moved[1].end - (0.9 - MIN_GAP)).abs() < 1e-12);
        let moved = move_end(&segments, 1, 0.5);
        assert!((moved[1].end - (0.5 + MIN_GAP)).abs() < 1e-12);
        assert_eq!(move_end(&segments, 2, 0.95), segments.to_vec(), "the last end is pinned");
    }

    #[test]
    fn a_double_click_adds_splits_or_fills_a_gap() {
        // First segment.
        assert_eq!(add_or_split(&[], 0.3), Some(vec![seg(0.0, 1.0, 0.0)]));

        // Split: both halves keep the volume, with a small gap between.
        let split = add_or_split(&[seg(0.0, 1.0, -6.0)], 0.5).unwrap();
        assert_eq!(split.len(), 2);
        assert!((split[0].end - (0.5 - MIN_GAP / 2.0)).abs() < 1e-12);
        assert!((split[1].start - (0.5 + MIN_GAP / 2.0)).abs() < 1e-12);
        assert_eq!((split[0].volume, split[1].volume), (-6.0, -6.0));
        assert_eq!(split[1].end, 1.0);

        // A gap gets a new segment around the click.
        let gap = add_or_split(&[seg(0.0, 0.2, 0.0), seg(0.8, 1.0, 0.0)], 0.5).unwrap();
        assert_eq!(gap.len(), 3);
        assert!((gap[1].start - 0.46).abs() < 1e-12 && (gap[1].end - 0.54).abs() < 1e-12);

        // No room: a tiny segment cannot be split, a tiny gap cannot be filled.
        assert_eq!(add_or_split(&[seg(0.0, 0.01, 0.0), seg(0.9, 1.0, 0.0)], 0.005), None);
        assert_eq!(add_or_split(&[seg(0.0, 0.5, 0.0), seg(0.51, 1.0, 0.0)], 0.505), None);
    }

    #[test]
    fn removing_an_end_segment_stretches_its_neighbour() {
        let segments = [seg(0.0, 0.3, 0.0), seg(0.5, 0.8, 0.0), seg(0.9, 1.0, 0.0)];

        let first_gone = remove(&segments, 0);
        assert_eq!(first_gone[0].start, 0.0);
        let last_gone = remove(&segments, 2);
        assert_eq!(last_gone[1].end, 1.0);
        let middle_gone = remove(&segments, 1);
        assert_eq!(middle_gone, vec![segments[0], segments[2]]);
        assert_eq!(remove(&segments, 5), segments.to_vec());
        assert!(remove(&[seg(0.0, 1.0, 0.0)], 0).is_empty());
    }

    #[test]
    fn plus_splits_the_longest_segment_again_and_again() {
        let mut segments = Vec::new();
        for expected in 1..=6 {
            segments = add_in_longest(&segments).expect("there is room");
            assert_eq!(segments.len(), expected);
        }
        // Still in order, inside 0..1.
        assert!(segments.windows(2).all(|pair| pair[0].end < pair[1].start));
        assert_eq!((segments[0].start, segments[5].end), (0.0, 1.0));

        let uneven = [seg(0.0, 0.1, 0.0), seg(0.2, 1.0, -3.0)];
        let result = add_in_longest(&uneven).unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[1].end - 0.6).abs() < 0.01);
    }

    #[test]
    fn envelopes_are_the_same_despite_f32_rounding() {
        let a = [seg(0.0, 1.0, -3.123_456_789)];
        let b = [seg(0.0, 1.0, f64::from(-3.123_456_7_f32))];
        assert_ne!(a, b);
        assert!(same(&a, &b));
        assert!(!same(&a, &[seg(0.0, 1.0, -4.0)]));
        assert!(!same(&a, &[]));
    }
}
