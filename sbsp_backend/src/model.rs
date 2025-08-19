use serde::{Deserialize, Serialize};

use crate::model::{cue::Cue, settings::ShowSettings};

pub mod cue;
pub mod settings;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ShowModel {
    pub name: String,
    pub cues: Vec<Cue>,
    pub settings: ShowSettings,
}
