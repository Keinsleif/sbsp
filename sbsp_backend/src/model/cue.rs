// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

pub mod audio;
pub mod group;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
pub use uuid::Uuid;

#[cfg(feature = "backend")]
use crate::manager::project::{ProjectCue, ProjectCueParam};
use crate::model::cue::{
    audio::{AudioCueParam, Decibels, FadeParam},
    group::GroupCueParamBase,
};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CueList {
    pub cues: HashMap<Uuid, Cue>,
    pub root_ids: Vec<Uuid>,
}

#[cfg(feature = "backend")]
impl CueList {
    fn flatten_cue(cue: ProjectCue, parent_id: Option<Uuid>, flat_list: &mut HashMap<Uuid, Cue>) -> Result<(), anyhow::Error> {
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
                    Self::flatten_cue(child, Some(cue.id), flat_list)?;
                }

                CueParam::Group { base, children: child_ids }
            },
        };
        let flat_cue = Cue { id: cue.id, number: cue.number, name: cue.name, notes: cue.notes, color: cue.color, pre_wait: cue.pre_wait, chain: cue.chain, parent_id, params: flat_params };

        if flat_list.contains_key(&cue.id) {
            return Err(anyhow::anyhow!("Duplicate key found."));
        }
        flat_list.insert(cue.id, flat_cue);
        Ok(())
    }

    fn reconstruct_cue(flat_list: &HashMap<Uuid, Cue>, cue_ids: &[Uuid], cue_list: &mut Vec<ProjectCue>) {
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
                        Self::reconstruct_cue(flat_list, children, &mut child_cues);
                        ProjectCueParam::Group { base: base.clone(), children: Box::new(child_cues) }
                    }
                };
                cue_list.push(ProjectCue { id: flat_cue.id, number: flat_cue.number.clone(), name: flat_cue.name.clone(), notes: flat_cue.notes.clone(), color: flat_cue.color, pre_wait: flat_cue.pre_wait, chain: flat_cue.chain, params: cue_params });
            }
        }
    }
}

#[cfg(feature = "backend")]
impl TryFrom<Vec<ProjectCue>> for CueList {
    type Error = anyhow::Error;
    fn try_from(value: Vec<ProjectCue>) -> Result<Self, Self::Error> {
        let mut flat_list = CueList::default();
        for cue in value {
            flat_list.root_ids.push(cue.id);
            Self::flatten_cue(cue, None, &mut flat_list.cues)?;
        }
        Ok(flat_list)
    }
}

#[cfg(feature = "backend")]
impl From<CueList> for Vec<ProjectCue> {
    fn from(value: CueList) -> Self {
        let mut cue_list = Vec::with_capacity(value.root_ids.len());
        CueList::reconstruct_cue(&value.cues, &value.root_ids, &mut cue_list);
        cue_list
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: Uuid,
    pub number: String,
    pub name: Option<String>,
    pub notes: String,
    pub color: CueColor,
    pub pre_wait: f64,
    pub chain: CueChain,
    pub parent_id: Option<Uuid>,
    pub params: CueParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CueColor {
    #[default]
    None,
    Red,
    Purple,
    Blue,
    Cyan,
    Green,
    Yellow,
    Orange,
    Grey,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueChain {
    #[default]
    DoNotChain,
    AfterStart {
        target_id: Option<Uuid>,
    },
    AfterComplete {
        target_id: Option<Uuid>,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueParam {
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
        children: Vec<Uuid>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct WaitCueParam {
    pub duration: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct FadeCueParam {
    pub target: Uuid,
    pub volume: Decibels,
    pub fade_param: FadeParam,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartCueParam {
    pub target: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct StopCueParam {
    pub target: Uuid,
    #[serde(default)]
    pub hard: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct PauseCueParam {
    pub target: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct LoadCueParam {
    pub target: Uuid,
}