use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioCueParam {
    pub target: PathBuf,
    pub start_time: Option<f64>,
    pub fade_in_param: Option<AudioFadeParam>,
    pub end_time: Option<f64>,
    pub fade_out_param: Option<AudioFadeParam>,
    pub volume: f32,
    pub pan: f32,
    pub repeat: bool,
    pub sound_type: SoundType,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioFadeParam {
    pub duration: f64,
    pub easing: Easing,
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
    InPowi(i32),
    OutPowi(i32),
    InOutPowi(i32),
    InPowf(f64),
    OutPowf(f64),
    InOutPowf(f64),
}

#[cfg(feature = "backend")]
impl From<kira::Easing> for Easing {
    fn from(value: kira::Easing) -> Self {
        match value {
            kira::Easing::Linear => Self::Linear,
            kira::Easing::InPowi(i) => Self::InPowi(i),
            kira::Easing::OutPowi(i) => Self::OutPowi(i),
            kira::Easing::InOutPowi(i) => Self::InOutPowi(i),
            kira::Easing::InPowf(f) => Self::InPowf(f),
            kira::Easing::OutPowf(f) => Self::OutPowf(f),
            kira::Easing::InOutPowf(f) => Self::InOutPowf(f),
        }
    }
}

#[cfg(feature = "backend")]
impl From<Easing> for kira::Easing {
    fn from(val: Easing) -> Self {
        match val {
            Easing::Linear => kira::Easing::Linear,
            Easing::InPowi(i) => kira::Easing::InPowi(i),
            Easing::OutPowi(i) => kira::Easing::OutPowi(i),
            Easing::InOutPowi(i) => kira::Easing::InOutPowi(i),
            Easing::InPowf(f) => kira::Easing::InPowf(f),
            Easing::OutPowf(f) => kira::Easing::OutPowf(f),
            Easing::InOutPowf(f) => kira::Easing::InOutPowf(f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SoundType {
    Static,
    Streaming,
}
