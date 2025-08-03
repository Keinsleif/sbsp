use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{cue::Cue, settings::ShowSettings};

pub mod cue;
mod settings;

#[derive(Serialize, Deserialize, Debug, Clone, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShowModel {
    pub name: String,
    pub cues: Vec<Cue>,
    pub settings: ShowSettings,
}
