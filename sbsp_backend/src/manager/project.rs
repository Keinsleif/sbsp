// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::cue::{Cue, CueList};
use crate::model::{ShowModel, cue::CueParam};
use crate::model::{
    cue::{
        CueChain, CueColor, CueCursorAdvanceTriggerOverride, FadeCueParam, LoadCueParam,
        PauseCueParam, StartCueParam, StopCueParam, WaitCueParam, audio::AudioCueParam,
        group::GroupCueParamBase,
    },
    settings::ShowSettings,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProjectType {
    #[default]
    SingleFile,
    ProjectFolder,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
impl TryFrom<ShowModel> for ProjectShowModel {
    type Error = anyhow::Error;

    fn try_from(value: ShowModel) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            cues: value.cue_list.try_into()?,
            settings: value.settings,
        })
    }
}

#[cfg(feature = "backend")]
impl TryFrom<ProjectShowModel> for ShowModel {
    type Error = anyhow::Error;

    fn try_from(value: ProjectShowModel) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            cue_list: value.cues.try_into()?,
            settings: value.settings,
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCue {
    pub id: Uuid,
    #[serde(default)]
    pub number: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub color: CueColor,
    #[serde(default)]
    pub pre_wait: f64,
    #[serde(default)]
    pub chain: CueChain,
    #[serde(default)]
    pub treat_stop_as_completed: bool,
    #[serde(default)]
    pub cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride,
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
    Wait(WaitCueParam),
    Fade(FadeCueParam),
    Start(StartCueParam),
    Stop(StopCueParam),
    Pause(PauseCueParam),
    Load(LoadCueParam),
    Group {
        #[serde(flatten)]
        base: GroupCueParamBase,
        children: Box<Vec<ProjectCue>>,
    },
}

fn flatten_cue(
    cue: ProjectCue,
    parent_id: Option<Uuid>,
    flat_list: &mut HashMap<Uuid, Cue>,
) -> Result<(), anyhow::Error> {
    let flat_params = match cue.params {
        ProjectCueParam::Audio(audio_cue_param) => CueParam::Audio(audio_cue_param),
        ProjectCueParam::Wait(wait_cue_param) => CueParam::Wait(wait_cue_param),
        ProjectCueParam::Fade(fade_cue_param) => CueParam::Fade(fade_cue_param),
        ProjectCueParam::Start(start_cue_param) => CueParam::Start(start_cue_param),
        ProjectCueParam::Stop(stop_cue_param) => CueParam::Stop(stop_cue_param),
        ProjectCueParam::Pause(pause_cue_param) => CueParam::Pause(pause_cue_param),
        ProjectCueParam::Load(load_cue_param) => CueParam::Load(load_cue_param),
        ProjectCueParam::Group { base, children } => {
            let child_ids = children.iter().map(|child| child.id).collect();
            for child in *children {
                flatten_cue(child, Some(cue.id), flat_list)?;
            }

            CueParam::Group {
                base,
                children: child_ids,
            }
        }
    };
    let flat_cue = Cue {
        id: cue.id,
        number: cue.number,
        name: cue.name,
        notes: cue.notes,
        color: cue.color,
        pre_wait: cue.pre_wait,
        chain: cue.chain,
        treat_stop_as_completed: cue.treat_stop_as_completed,
        cursor_advance_trigger_override: cue.cursor_advance_trigger_override,
        parent_id,
        params: flat_params,
    };

    if flat_list.contains_key(&cue.id) {
        return Err(anyhow::anyhow!("Duplicate key found."));
    }
    flat_list.insert(cue.id, flat_cue);
    Ok(())
}

fn reconstruct_cue(
    flat_list: &HashMap<Uuid, Cue>,
    cue_ids: &[Uuid],
    cue_list: &mut Vec<ProjectCue>,
) -> Result<(), anyhow::Error> {
    for cue_id in cue_ids {
        if let Some(flat_cue) = flat_list.get(cue_id) {
            let cue_params = match &flat_cue.params {
                CueParam::Audio(audio_cue_param) => ProjectCueParam::Audio(audio_cue_param.clone()),
                CueParam::Wait(wait_cue_param) => ProjectCueParam::Wait(wait_cue_param.clone()),
                CueParam::Fade(fade_cue_param) => ProjectCueParam::Fade(fade_cue_param.clone()),
                CueParam::Start(start_cue_param) => ProjectCueParam::Start(start_cue_param.clone()),
                CueParam::Stop(stop_cue_param) => ProjectCueParam::Stop(stop_cue_param.clone()),
                CueParam::Pause(pause_cue_param) => ProjectCueParam::Pause(pause_cue_param.clone()),
                CueParam::Load(load_cue_param) => ProjectCueParam::Load(load_cue_param.clone()),
                CueParam::Group { base, children } => {
                    let mut child_cues = Vec::with_capacity(children.len());
                    reconstruct_cue(flat_list, children, &mut child_cues)?;
                    ProjectCueParam::Group {
                        base: base.clone(),
                        children: Box::new(child_cues),
                    }
                }
            };
            cue_list.push(ProjectCue {
                id: flat_cue.id,
                number: flat_cue.number.clone(),
                name: flat_cue.name.clone(),
                notes: flat_cue.notes.clone(),
                color: flat_cue.color,
                pre_wait: flat_cue.pre_wait,
                chain: flat_cue.chain,
                treat_stop_as_completed: flat_cue.treat_stop_as_completed,
                cursor_advance_trigger_override: flat_cue.cursor_advance_trigger_override,
                params: cue_params,
            });
        } else {
            anyhow::bail!("cue_id {} not found in flatten cue list.", cue_id);
        }
    }
    Ok(())
}

impl TryFrom<Vec<ProjectCue>> for CueList {
    type Error = anyhow::Error;
    fn try_from(value: Vec<ProjectCue>) -> Result<Self, Self::Error> {
        let mut flat_list = CueList::default();
        for cue in value {
            flat_list.root_ids.push(cue.id);
            flatten_cue(cue, None, &mut flat_list.cues)?;
        }
        Ok(flat_list)
    }
}

impl TryFrom<CueList> for Vec<ProjectCue> {
    type Error = anyhow::Error;
    fn try_from(value: CueList) -> Result<Self, Self::Error> {
        let mut cue_list = Vec::with_capacity(value.root_ids.len());
        reconstruct_cue(&value.cues, &value.root_ids, &mut cue_list)?;
        Ok(cue_list)
    }
}
