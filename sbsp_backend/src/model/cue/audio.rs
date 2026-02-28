use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioCueParam {
    pub target: PathBuf,
    pub start_time: Option<f64>,
    pub fade_in_param: Option<FadeParam>,
    pub end_time: Option<f64>,
    pub fade_out_param: Option<FadeParam>,
    pub volume: f32,
    pub pan: f32,
    pub repeat: bool,
    pub sound_type: SoundType,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, TS)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, TS)]
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

impl Easing {
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SoundType {
    Static,
    Streaming,
}
