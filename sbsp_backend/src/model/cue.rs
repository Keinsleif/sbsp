use std::path::PathBuf;

use kira::sound::{IntoOptionalRegion, Region};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: Uuid,
    pub number: String,
    pub name: String,
    pub notes: String,
    pub pre_wait: f64,
    pub sequence: CueSequence,
    pub params: CueParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, TS)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CueSequence {
    #[default]
    DoNotContinue,
    AutoFollow {
        target_id: Option<Uuid>,
        post_wait: f64,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CueParam {
    Audio {
        target: PathBuf,
        start_time: Option<f64>,
        fade_in_param: Option<AudioCueFadeParam>,
        end_time: Option<f64>,
        fade_out_param: Option<AudioCueFadeParam>,
        levels: AudioCueLevels,
        loop_region: Option<LoopRegion>,
    },
    Wait {
        duration: f64,
    }, // TODO midi, osc wait, group cue
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioCueLevels {
    pub master: f64, // decibels
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioCueFadeParam {
    pub duration: f64,
    pub easing: Easing,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, TS)]
pub struct LoopRegion{
    pub start: Option<f64>,
    pub end: Option<f64>,
}

impl From<(Option<f64>, Option<f64>)> for LoopRegion {
    fn from(value: (Option<f64>, Option<f64>)) -> Self {
        Self {
            start: value.0,
            end: value.1,
        }
    }
}

impl From<LoopRegion> for Option<Region> {
    fn from(val: LoopRegion) -> Self {
        match (val.start, val.end) {
            (None, None) => None,
            (None, Some(end)) => Some(Region { start: kira::sound::PlaybackPosition::Seconds(0.0), end: kira::sound::EndPosition::Custom(kira::sound::PlaybackPosition::Seconds(end)) }),
            (Some(start), None) => Some(Region { start: kira::sound::PlaybackPosition::Seconds(start), end: kira::sound::EndPosition::EndOfAudio }),
            (Some(start), Some(end)) => Some(Region { start: kira::sound::PlaybackPosition::Seconds(start), end: kira::sound::EndPosition::Custom(kira::sound::PlaybackPosition::Seconds(end)) }),
        }
    }
}

impl IntoOptionalRegion for LoopRegion {
    fn into_optional_region(self) -> Option<Region> {
        self.into()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, TS)]
pub enum Easing {
	Linear,
	InPowi(i32),
	OutPowi(i32),
	InOutPowi(i32),
	InPowf(f64),
	OutPowf(f64),
	InOutPowf(f64),
}

impl From <kira::Easing> for Easing {
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