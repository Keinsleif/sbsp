use std::path::PathBuf;

use kira::{Easing, sound::Region};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: Uuid,
    pub number: String,
    pub name: String,
    pub notes: String,
    pub pre_wait: f64,
    pub post_wait: f64,
    pub sequence: CueSequence,
    pub param: CueParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CueSequence {
    #[default]
    DoNotContinue,
    AutoContinue,
    AutoFollow,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "params", rename_all = "camelCase")]
pub enum CueParam {
    Audio {
        target: PathBuf,
        start_time: Option<f64>,
        fade_in_param: Option<AudioCueFadeParam>,
        end_time: Option<f64>,
        fade_out_param: Option<AudioCueFadeParam>,
        levels: AudioCueLevels,
        loop_region: Option<Region>,
    },
    Wait {
        duration: f64,
    }, // TODO midi, osc wait, group cue
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioCueLevels {
    pub master: f64, // decibels
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioCueFadeParam {
    pub duration: f64,
    pub easing: Easing,
}
