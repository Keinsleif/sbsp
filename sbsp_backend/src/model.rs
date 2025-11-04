use serde::{Deserialize, Serialize};

use crate::model::{cue::Cue, settings::ShowSettings};

pub mod cue;
pub mod settings;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ShowModel {
    pub name: String,
    pub cues: Vec<Cue>,
    pub settings: ShowSettings,
}

impl Default for ShowModel {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            cues: Vec::new(),
            settings: ShowSettings::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ProjectType {
    #[default]
    SingleFile,
    ProjectFolder,
}
