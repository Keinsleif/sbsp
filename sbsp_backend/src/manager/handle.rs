// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    manager::{ModelCommand, project::ProjectStatus, command::InsertPosition},
    model::{
        ShowModel,
        cue::{Cue, CueChain, CueParam, group::GroupMode},
        settings::ShowSettings,
    },
};

#[derive(Clone)]
pub struct ShowModelHandle {
    model: Arc<RwLock<ShowModel>>,
    command_tx: mpsc::Sender<ModelCommand>,
    project_status: Arc<RwLock<ProjectStatus>>,
    modify_status: Arc<AtomicBool>,
}

impl ShowModelHandle {
    pub fn new(
        model: Arc<RwLock<ShowModel>>,
        command_tx: mpsc::Sender<ModelCommand>,
        project_status: Arc<RwLock<ProjectStatus>>,
        modify_status: Arc<AtomicBool>,
    ) -> Self {
        Self {
            model,
            command_tx,
            project_status,
            modify_status,
        }
    }

    pub async fn send_command(&self, command: ModelCommand) -> anyhow::Result<()> {
        self.command_tx.send(command).await?;
        Ok(())
    }

    pub async fn update_cue(&self, cue: Cue) -> anyhow::Result<()> {
        self.send_command(ModelCommand::UpdateCue(cue)).await?;
        Ok(())
    }

    pub async fn add_cue(&self, cue: Cue, position: InsertPosition) -> anyhow::Result<()> {
        self.send_command(ModelCommand::AddCue { cue, position })
            .await?;
        Ok(())
    }

    pub async fn add_cues(&self, cues: Vec<Cue>, position: InsertPosition) -> anyhow::Result<()> {
        self.send_command(ModelCommand::AddCues { cues, position })
            .await?;
        Ok(())
    }

    pub async fn remove_cue(&self, cue_id: Uuid) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RemoveCue { cue_id })
            .await?;
        Ok(())
    }

    pub async fn remove_cues(&self, cue_ids: HashSet<Uuid>) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RemoveCues { cue_ids })
            .await?;
        Ok(())
    }

    pub async fn move_cue(&self, cue_id: Uuid, position: InsertPosition) -> anyhow::Result<()> {
        self.send_command(ModelCommand::MoveCue { cue_id, position })
            .await?;
        Ok(())
    }

    pub async fn move_cues(
        &self,
        cue_ids: HashSet<Uuid>,
        position: InsertPosition,
    ) -> anyhow::Result<()> {
        self.send_command(ModelCommand::MoveCues { cue_ids, position })
            .await?;
        Ok(())
    }

    pub async fn renumber_cues(
        &self,
        cues: Vec<Uuid>,
        start_from: usize,
        increment: usize,
        prefix: Option<String>,
        suffix: Option<String>,
    ) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RenumberCues {
            cues,
            start_from,
            increment,
            prefix,
            suffix,
        })
        .await?;
        Ok(())
    }

    pub async fn update_model_name(&self, new_name: String) -> anyhow::Result<()> {
        self.send_command(ModelCommand::UpdateModelName(new_name))
            .await?;
        Ok(())
    }

    pub async fn update_settings(&self, new_settings: ShowSettings) -> anyhow::Result<()> {
        self.send_command(ModelCommand::UpdateSettings(Box::new(new_settings)))
            .await?;
        Ok(())
    }

    pub async fn reset(&self) -> anyhow::Result<()> {
        self.send_command(ModelCommand::Reset).await?;
        Ok(())
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        self.send_command(ModelCommand::Save).await?;
        Ok(())
    }

    pub async fn save_as(&self, path: PathBuf) -> anyhow::Result<()> {
        self.send_command(ModelCommand::SaveToFile(path)).await?;
        Ok(())
    }

    pub async fn export_to_folder(&self, folder_path: PathBuf) -> anyhow::Result<()> {
        self.send_command(ModelCommand::ExportToFolder(folder_path))
            .await?;
        Ok(())
    }

    pub async fn load_from_file(&self, path: PathBuf) -> anyhow::Result<()> {
        self.send_command(ModelCommand::LoadFromFile(path)).await?;
        Ok(())
    }

    pub async fn is_cue_exists(&self, cue_id: &Uuid) -> bool {
        let model = self.read().await;
        model.cue_list.cues.contains_key(cue_id)
    }

    pub async fn get_cue_by_id(&self, cue_id: &Uuid) -> Option<Cue> {
        self.read().await.cue_list.cues.get(cue_id).cloned()
    }

    pub async fn get_parent_by_id(&self, cue_id: &Uuid) -> Option<Cue> {
        let model = self.read().await;

        model.cue_list.cues.get(cue_id).and_then(|cue| {
            cue.parent_id.and_then(|parent_id| {
                model.cue_list.cues.get(&parent_id).cloned()
            })
        })
    }

    pub async fn get_all_children_by_id(&self, cue_id: &Uuid) -> Vec<Cue> {
        let model = self.model.read().await;
        let mut result = Vec::new();
        let target_cue = model.cue_list.cues.get(cue_id);
        if let Some(target) = target_cue
            && let CueParam::Group { children, .. } = &target.params
        {
            let mut queue: VecDeque<&Vec<Uuid>> = VecDeque::from([children]);
            while let Some(cue_ids) = queue.pop_front() {
                for id in cue_ids {
                    if let Some(cue) = model.cue_list.cues.get(id) {
                        if let CueParam::Group { children, .. } = &cue.params {
                            queue.push_back(children);
                        } else {
                            result.push(cue.clone());
                        }
                    }
                }
            }
        }
        result
    }

    pub async fn get_next_cue_id_by_id(&self, cue_id: &Uuid) -> Option<Uuid> {
        let model = self.model.read().await;
        let mut current_id = *cue_id;

        loop {
            let cue = model.cue_list.cues.get(&current_id)?;

            if let Some(parent_id) = cue.parent_id {
                let parent = model.cue_list.cues.get(&parent_id)?;
                if let CueParam::Group { children, .. } = &parent.params
                    && let Some(idx) = children.iter().position(|id| *id == current_id)
                {
                    if let Some(next_id) = children.get(idx + 1) {
                        return Some(*next_id);
                    }

                    current_id = parent_id;
                    continue;
                }

                return None;
            }

            if let Some(idx) = model.cue_list.root_ids.iter().position(|id| *id == current_id) {
                return model.cue_list.root_ids.get(idx + 1).copied();
            }

            return None;
        }
    }

    pub async fn get_cue_chain_by_id(&self, cue_id: &Uuid) -> Option<CueChain> {
        let model = self.read().await;

        if let Some(cue) = model.cue_list.cues.get(cue_id) {
            if let Some(parent_id) = cue.parent_id {
                if let Some(parent) = model.cue_list.cues.get(&parent_id) {
                    if let CueParam::Group { base, children } = &parent.params {
                        match base.mode {
                            GroupMode::Playlist { repeat } => {
                                if children.last() == Some(cue_id)
                                    && let Some(first_id) = children.first()
                                {
                                    if repeat {
                                        return Some(CueChain::AfterComplete {
                                            target_id: Some(*first_id),
                                        });
                                    } else {
                                        return Some(CueChain::DoNotChain);
                                    }
                                } else {
                                    return Some(CueChain::AfterComplete { target_id: None });
                                }
                            },
                            GroupMode::Concurrency |
                            GroupMode::StartFirst { .. } => return Some(cue.chain),
                        }
                    } else {
                        log::warn!("broken cues, invalid parent_id.");
                    }
                }
            } else {
                return Some(cue.chain)
            }
        }
        None
    }

    pub async fn get_all_asset_paths(&self) -> HashSet<PathBuf> {
        let targets: HashSet<_> = {
            let model = self.read().await;
            model.cue_list.cues.values().filter_map(|cue| {
                if let CueParam::Audio(params) = &cue.params {
                    Some(params.target.clone())
                } else {
                    None
                }
            }).collect()
        };
        let mut result = HashSet::new();

        for target in targets {
            if let Ok(path) = self.get_asset_standard_path(&target).await {
                result.insert(path);
            }
        }

        result
    }

    pub async fn get_current_file_path(&self) -> Option<PathBuf> {
        if let ProjectStatus::Saved { path, .. } = self.project_status.read().await.to_owned() {
            Some(path)
        } else {
            None
        }
    }

    pub async fn get_asset_folder_path(&self) -> Option<PathBuf> {
        if let ProjectStatus::Saved { path, .. } = self.project_status.read().await.to_owned() {
            let asset_dir = self
                .model
                .read()
                .await
                .settings
                .general
                .copy_assets_destination
                .clone();
            path.parent().map(|path| path.to_path_buf().join(asset_dir))
        } else {
            None
        }
    }

    pub async fn get_asset_standard_path(&self, path: &PathBuf) -> anyhow::Result<PathBuf> {
        if let Some(model_path) = self.get_current_file_path().await
            && let Some(parent) = model_path.parent()
        {
            Ok(tokio::fs::canonicalize(parent.join(path)).await?)
        } else {
            Ok(tokio::fs::canonicalize(path).await?)
        }
    }

    pub async fn get_project_state(&self) -> tokio::sync::RwLockReadGuard<'_, ProjectStatus> {
        self.project_status.read().await
    }

    pub fn is_modified(&self) -> bool {
        self.modify_status.load(Ordering::Acquire)
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, ShowModel> {
        self.model.read().await
    }
}
