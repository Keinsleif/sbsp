// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The flashing of the transport buttons.

use std::f32::consts::TAU;
use std::time::Duration;

/// How strongly a flashing button is lit `elapsed` after the flashing began:
/// one flash a second, fading from fully lit to unlit and back.
pub fn level(elapsed: Duration) -> f32 {
    let phase = (elapsed.as_millis() % 1000) as f32 / 1000.0;
    0.5 + 0.5 * (TAU * phase).cos()
}

/// How a button is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lit {
    Off,
    On,
    Flashing,
}

impl Lit {
    /// The strength of the light at the current flashing `level`.
    pub fn strength(self, level: f32) -> Strength {
        match self {
            Lit::Off => Strength(0.0),
            Lit::On => Strength(1.0),
            Lit::Flashing => Strength(level),
        }
    }
}

/// How lit a button is, from 0.0 (not at all) to 1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strength(pub f32);

impl Strength {
    pub fn is_lit(self) -> bool {
        self.0 > 0.0
    }

    /// Whether the light is strong enough for the icon to take the color
    /// made for the lit background.
    pub fn shows_accent(self) -> bool {
        self.0 >= 0.5
    }
}

impl From<bool> for Strength {
    fn from(lit: bool) -> Self {
        Strength(if lit { 1.0 } else { 0.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flash_takes_a_second_and_dims_halfway() {
        let at = |ms| level(Duration::from_millis(ms));
        assert!((at(0) - 1.0).abs() < 1e-6);
        assert!(at(500).abs() < 1e-6);
        assert!((at(1000) - 1.0).abs() < 1e-6);
        assert!((at(2250) - at(250)).abs() < 1e-6);
    }

    #[test]
    fn only_a_flashing_light_follows_the_level() {
        assert_eq!(Lit::Off.strength(0.3), Strength(0.0));
        assert_eq!(Lit::On.strength(0.3), Strength(1.0));
        assert_eq!(Lit::Flashing.strength(0.3), Strength(0.3));
        assert_eq!(Strength::from(true), Strength(1.0));
        assert!(!Strength(0.0).is_lit());
        assert!(Strength(0.5).shows_accent());
        assert!(!Strength(0.4).shows_accent());
    }
}
