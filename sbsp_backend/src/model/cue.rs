pub mod audio;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::cue::audio::AudioCueParam;

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
    Audio(AudioCueParam),
    Wait {
        duration: f64,
    },
}
