use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::{cue::Cue, settings::ShowSettings};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InsertPosition {
    Before {
        target: Uuid
    },
    After {
        target: Uuid
    },
    Inside {
        target: Option<Uuid>,
        index: usize
    },
    Last,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "command",
    content = "params",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ModelCommand {
    UpdateCue(Cue),
    AddCue {
        cue: Cue,
        position: InsertPosition,
    },
    AddCues {
        cues: Vec<Cue>,
        position: InsertPosition,
    },
    RemoveCue {
        cue_id: Uuid,
    },
    MoveCue {
        cue_id: Uuid,
        position: InsertPosition,
    },

    RenumberCues {
        cues: Vec<Uuid>,
        start_from: f64,
        increment: f64,
    },

    UpdateModelName(String),
    UpdateSettings(Box<ShowSettings>),

    Reset,
    Save,
    SaveToFile(PathBuf),
    ExportToFolder(PathBuf),
    LoadFromFile(PathBuf),
}