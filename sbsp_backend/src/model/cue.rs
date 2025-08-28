use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: Uuid,
    pub number: String,
    pub name: Option<String>,
    pub notes: String,
    pub pre_wait: f64,
    pub sequence: CueSequence,
    pub params: CueParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueSequence {
    #[default]
    DoNotContinue,
    AutoContinue {
        target_id: Option<Uuid>,
        post_wait: f64,
    },
    AutoFollow {
        target_id: Option<Uuid>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueParam {
    Audio {
        target: PathBuf,
        start_time: Option<f64>,
        fade_in_param: Option<AudioCueFadeParam>,
        end_time: Option<f64>,
        fade_out_param: Option<AudioCueFadeParam>,
        volume: f32,
        pan: f32,
        repeat: bool,
    },
    Wait {
        duration: f64,
    }, // TODO midi, osc wait, group cue
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AudioCueFadeParam {
    pub duration: f64,
    pub easing: Easing,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
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
