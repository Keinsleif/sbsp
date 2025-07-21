use serde::{Deserialize, Serialize};

use crate::model::{cue::Cue, settings::ShowSettings};

pub mod cue;
mod settings;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShowModel {
    pub name: String,
    pub cues: Vec<Cue>,
    pub settings: ShowSettings,
}
