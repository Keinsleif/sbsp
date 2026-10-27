// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(feature = "backend")]
use std::ops::{Add, Mul, Sub};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Decibels(f32);

impl Default for Decibels {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Decibels {
    pub const MUTE: Self = Self(-60.0);
    pub const IDENTITY: Self = Self(0.0);

    #[cfg(feature = "backend")]
    #[inline]
    pub fn is_silent(self) -> bool { self.0 <= Self::MUTE.0 }

    #[cfg(feature = "backend")]
    #[inline]
    pub fn stack(self, rhs: Self) -> Self {
        let sum = self.0 + rhs.0;
        if self.is_silent() || rhs.is_silent() || sum <= Self::MUTE.0 {
            Self::MUTE
        } else {
            Self(sum)
        }
    }

    #[cfg(feature = "backend")]
    #[inline]
    pub fn as_amplitude(&self) -> f32 {
        if self.is_silent() {
            0.0
        } else {
            10.0f32.powf(self.0 / 20.0)
        }
    }
}

impl From<f32> for Decibels {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<Decibels> for f32 {
    fn from(value: Decibels) -> Self {
        value.0
    }
}

#[cfg(feature = "backend")]
impl Add for Decibels {
    type Output = Decibels;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

#[cfg(feature = "backend")]
impl Sub for Decibels {
    type Output = Decibels;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

#[cfg(feature = "backend")]
impl Mul<f32> for Decibels {
    type Output = Decibels;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioCueParam {
    pub target: PathBuf,
    pub start_time: Option<f64>,
    pub fade_in_param: Option<FadeParam>,
    pub end_time: Option<f64>,
    pub fade_out_param: Option<FadeParam>,
    pub volume: Decibels,
    pub pan: f32,
    pub repeat: bool,
    pub sound_type: SoundType,
    #[serde(default)]
    pub envelope: Vec<EnvelopeSegment>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FadeParam {
    pub duration: f64,
    pub easing: Easing,
}

impl Default for FadeParam {
    fn default() -> Self {
        Self {
            duration: 3.0,
            easing: Easing::InOutPow(2.0),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeSegment {
    pub start: f64,
    pub end: f64,
    pub volume: Decibels,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(
    tag = "type",
    content = "intensity",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Easing {
    Linear,
    InPow(f64),
    OutPow(f64),
    InOutPow(f64),
}

#[cfg(feature = "backend")]
impl Easing {
    #[inline]
    pub fn get_factor(&self, mut x: f64) -> f64 {
        match self {
            Easing::Linear => x,
            Easing::InPow(power) => x.powf(*power),
            Easing::OutPow(power) => 1.0 - Self::InPow(*power).get_factor(1.0 - x),
            Easing::InOutPow(power) => {
                x *= 2.0;
                if x < 1.0 {
                    0.5 * Self::InPow(*power).get_factor(x)
                } else {
                    x = 2.0 - x;
                    0.5 * (1.0 - Self::InPow(*power).get_factor(x)) + 0.5
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SoundType {
    Static,
    #[default]
    Streaming,
}

#[cfg(test)]
mod tests {
    use crate::model::cue::audio::Decibels;

    #[test]
    fn decibels_stack() {
        assert_eq!(Decibels::MUTE.stack(Decibels(6.0)), Decibels::MUTE);

        assert_eq!(Decibels(-40.0).stack(Decibels(-40.0)), Decibels::MUTE);

        assert_eq!(Decibels(-30.0).stack(Decibels(-20.0)), Decibels(-50.0));
    }

    #[test]
    fn as_amplitude_clamp() {
        assert_eq!(Decibels::MUTE.as_amplitude(), 0.0);

        assert_ne!(Decibels(-59.9).as_amplitude(), 0.0);
    }
}