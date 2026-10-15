// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The master level meter's display state: a peak-hold-and-decay value per
//! channel plus a short "clipped" flash. Ported from `LevelMeter.vue`'s
//! `decayLoop`; the drawing is `features::side_bar`'s.

/// Quietest level shown; also what a channel decays to.
pub const FLOOR_DB: f32 = -60.0;
/// How fast a peak falls back, in dB per second.
const DECAY_DB_PER_SEC: f32 = 30.0;
/// How long the clip indicator stays lit after the last clipped sample.
const CLIP_HOLD_SECS: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Channel {
    level_db: f32,
    clip_remaining: f32,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            level_db: FLOOR_DB,
            clip_remaining: 0.0,
        }
    }
}

impl Channel {
    /// The displayed level in dB, `FLOOR_DB..`; 0 dB and above is clipping.
    pub fn level_db(&self) -> f32 {
        self.level_db
    }

    pub fn is_clipping(&self) -> bool {
        self.clip_remaining > 0.0
    }

    /// How much of the bar is empty, 0 (full, at 0 dB or above) to 1
    /// (empty, at the floor): the original covers the bar from the top by
    /// this fraction.
    pub fn empty_fraction(&self) -> f32 {
        (self.level_db.min(0.0) / FLOOR_DB).clamp(0.0, 1.0)
    }

    fn push(&mut self, amplitude: f32) {
        self.level_db = self.level_db.max(to_db(amplitude));
    }

    fn step(&mut self, dt_secs: f32) {
        if self.level_db >= 0.0 {
            self.clip_remaining = CLIP_HOLD_SECS;
        } else if self.clip_remaining > 0.0 {
            self.clip_remaining = (self.clip_remaining - dt_secs).max(0.0);
        }

        if self.level_db > FLOOR_DB {
            self.level_db = (self.level_db - DECAY_DB_PER_SEC * dt_secs).max(FLOOR_DB);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LevelMeterState {
    pub left: Channel,
    pub right: Channel,
}

impl LevelMeterState {
    /// Feeds one reading of the backend's peak amplitudes (linear, 0..=1+).
    /// A channel only ever goes up here; falling is [`step`](Self::step)'s.
    pub fn push(&mut self, left: f32, right: f32) {
        self.left.push(left);
        self.right.push(right);
    }

    /// Advances the decay and the clip flash by `dt_secs`.
    pub fn step(&mut self, dt_secs: f32) {
        self.left.step(dt_secs);
        self.right.step(dt_secs);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

fn to_db(sample: f32) -> f32 {
    if sample.is_finite() && sample > 0.0 {
        20.0 * sample.log10()
    } else {
        FLOOR_DB
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_silent() {
        let meter = LevelMeterState::default();
        assert_eq!(meter.left.level_db(), FLOOR_DB);
        assert_eq!(meter.left.empty_fraction(), 1.0);
        assert!(!meter.left.is_clipping());
    }

    #[test]
    fn a_louder_reading_raises_the_level_and_a_quieter_one_does_not_lower_it() {
        let mut meter = LevelMeterState::default();

        meter.push(0.5, 0.1);
        assert!((meter.left.level_db() - -6.0206).abs() < 0.01);
        assert!((meter.right.level_db() - -20.0).abs() < 0.01);

        meter.push(0.1, 0.5);
        assert!((meter.left.level_db() - -6.0206).abs() < 0.01);
        assert!((meter.right.level_db() - -6.0206).abs() < 0.01);
    }

    #[test]
    fn the_level_decays_at_thirty_db_per_second_down_to_the_floor() {
        let mut meter = LevelMeterState::default();
        meter.push(0.1, 0.1); // -20 dB

        meter.step(0.5);
        assert!((meter.left.level_db() - -35.0).abs() < 0.01);

        meter.step(10.0);
        assert_eq!(meter.left.level_db(), FLOOR_DB);
    }

    #[test]
    fn clipping_lights_the_indicator_for_half_a_second() {
        let mut meter = LevelMeterState::default();
        meter.push(1.0, 0.1); // 0 dB is clipping, on the left only

        meter.step(0.1);
        assert!(meter.left.is_clipping());
        assert!(!meter.right.is_clipping());

        // Still held a moment later, then released.
        meter.step(0.3);
        assert!(meter.left.is_clipping());
        meter.step(0.3);
        assert!(!meter.left.is_clipping());
    }

    #[test]
    fn empty_fraction_runs_from_full_at_zero_db_to_empty_at_the_floor() {
        let mut meter = LevelMeterState::default();
        meter.push(1.0, 0.1);
        assert_eq!(meter.left.empty_fraction(), 0.0);
        assert!((meter.right.empty_fraction() - (20.0 / 60.0)).abs() < 0.01);

        // Above 0 dB does not go past full.
        meter.push(2.0, 0.1);
        assert_eq!(meter.left.empty_fraction(), 0.0);
    }

    #[test]
    fn silence_and_garbage_readings_stay_at_the_floor() {
        let mut meter = LevelMeterState::default();
        meter.push(0.0, f32::NAN);
        assert_eq!(meter.left.level_db(), FLOOR_DB);
        assert_eq!(meter.right.level_db(), FLOOR_DB);
    }

    #[test]
    fn reset_clears_everything() {
        let mut meter = LevelMeterState::default();
        meter.push(1.0, 1.0);
        meter.step(0.01);
        meter.reset();
        assert_eq!(meter, LevelMeterState::default());
    }
}
