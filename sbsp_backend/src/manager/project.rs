use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::{ProjectType, ShowModel};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ProjectStatus {
    Unsaved,
    Saved{
        project_type: ProjectType,
        path: PathBuf,
    },
}

impl ProjectStatus {
    pub fn to_model_path_option(&self) -> Option<PathBuf> {
        if let Self::Saved { path, .. } = self {
            Some(path.to_path_buf())
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
pub struct ProjectFile {
    pub project_type: ProjectType,
    pub model: ShowModel,
}
