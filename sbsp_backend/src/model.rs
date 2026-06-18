// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use serde::{Deserialize, Serialize};

use crate::model::{cue::CueList, settings::ShowSettings};

pub mod cue;
pub mod settings;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ShowModel {
    pub name: String,
    #[serde(flatten)]
    pub cue_list: CueList,
    pub settings: ShowSettings,
}

impl Default for ShowModel {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            cue_list: CueList::default(),
            settings: ShowSettings::default(),
        }
    }
}
