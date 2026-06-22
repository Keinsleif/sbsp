// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "backend")]
use crate::model::ShowModel;
use crate::model::{cue::{CueChain, CueColor, FadeCueParam, LoadCueParam, PauseCueParam, StartCueParam, StopCueParam, WaitCueParam, audio::AudioCueParam, group::GroupCueParamBase}, settings::ShowSettings};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ProjectType {
    #[default]
    SingleFile,
    ProjectFolder,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProjectStatus {
    Unsaved,
    Saved {
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
pub struct ProjectFile {
    pub project_type: ProjectType,
    pub model: ProjectShowModel,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectShowModel {
    name: String,
    cues: Vec<ProjectCue>,
    settings: ShowSettings,
}

impl Default for ProjectShowModel {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            cues: Vec::new(),
            settings: ShowSettings::default(),
        }
    }
}

#[cfg(feature = "backend")]
impl From<ShowModel> for ProjectShowModel {
    fn from(value: ShowModel) -> Self {
        Self { name: value.name, cues: value.cue_list.into(), settings: value.settings }
    }
}

#[cfg(feature = "backend")]
impl TryFrom<ProjectShowModel> for ShowModel {
    type Error = anyhow::Error;

    fn try_from(value: ProjectShowModel) -> Result<Self, Self::Error> {
        Ok(Self { name: value.name, cue_list: value.cues.try_into()?, settings: value.settings })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCue {
    pub id: Uuid,
    pub number: String,
    pub name: Option<String>,
    pub notes: String,
    #[serde(default)]
    pub color: CueColor,
    pub pre_wait: f64,
    #[serde(default)]
    pub chain: CueChain,
    pub params: ProjectCueParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProjectCueParam {
    Audio(AudioCueParam),
    Wait (WaitCueParam),
    Fade (FadeCueParam),
    Start (StartCueParam),
    Stop (StopCueParam),
    Pause (PauseCueParam),
    Load (LoadCueParam),
    Group {
        #[serde(flatten)]
        base: GroupCueParamBase,
        children: Box<Vec<ProjectCue>>,
    },
}
