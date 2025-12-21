use std::{collections::VecDeque, path::PathBuf, sync::{Arc, atomic::{AtomicBool, Ordering}}};

use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    manager::{ModelCommand, ProjectStatus, command::InsertPosition},
    model::{ShowModel, cue::{Cue, CueParam, CueSequence, group::GroupMode}, settings::ShowSettings},
};

#[derive(Clone)]
pub struct ShowModelHandle {
    model: Arc<RwLock<ShowModel>>,
    command_tx: mpsc::Sender<ModelCommand>,
    project_status: Arc<RwLock<ProjectStatus>>,
    modify_status: Arc<AtomicBool>,
}

impl ShowModelHandle {
    pub fn new(model: Arc<RwLock<ShowModel>>, command_tx: mpsc::Sender<ModelCommand>, project_status: Arc<RwLock<ProjectStatus>>, modify_status: Arc<AtomicBool>) -> Self {
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

    pub async fn move_cue(&self, cue_id: Uuid, position: InsertPosition) -> anyhow::Result<()> {
        self.send_command(ModelCommand::MoveCue { cue_id, position })
            .await?;
        Ok(())
    }

    pub async fn renumber_cues(
        &self,
        cues: Vec<Uuid>,
        start_from: f64,
        increment: f64,
    ) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RenumberCues {
            cues,
            start_from,
            increment,
        })
        .await?;
        Ok(())
    }

    pub async fn update_model_name(&self, new_name: String) -> anyhow::Result<()> {
        self.send_command(ModelCommand::UpdateModelName(new_name)).await?;
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
        self.send_command(ModelCommand::ExportToFolder(folder_path)).await?;
        Ok(())
    }

    pub async fn load_from_file(&self, path: PathBuf) -> anyhow::Result<()> {
        self.send_command(ModelCommand::LoadFromFile(path)).await?;
        Ok(())
    }

    pub async fn get_cue_by_id(&self, cue_id: &Uuid) -> Option<Cue> {
        let model = self.read().await;
        let mut queue: VecDeque<&Cue> = model.cues.iter().collect();

        while let Some(cue) = queue.pop_front() {
            if cue.id == *cue_id {
                return Some(cue.clone())
            }

            if let CueParam::Group { children, .. } = &cue.params {
                for child in children.iter() {
                    queue.push_back(child);
                }
            }
        }
        None
    }

    pub async fn get_cue_and_parent_by_id(&self, cue_id: &Uuid) -> Option<(Cue, Option<Cue>)> {
        let model = self.read().await;
        let mut queue: VecDeque<(&Vec<Cue>, Option<&Cue>)> = VecDeque::from([(&model.cues, None)]);

        while let Some((cues, parent)) = queue.pop_front() {
            for cue in cues {
                if cue.id == *cue_id {
                    return Some((cue.clone(), parent.cloned()));
                }

                if let CueParam::Group { children, .. } = &cue.params {
                    queue.push_back((children, Some(cue)));
                }
            }
        }
        None
    }

    pub async fn get_all_children_by_id(&self, cue_id: &Uuid) -> Vec<Cue> {
        let mut result = Vec::new();
        let target_cue = self.get_cue_by_id(cue_id).await;
        if let Some(target) = target_cue && let CueParam::Group { children, .. } = &target.params {
            let mut queue: VecDeque<&Vec<Cue>> = VecDeque::from([children.as_ref()]);
            while let Some(cues) = queue.pop_front() {
                for cue in cues {
                    if let CueParam::Group { children, .. } = &cue.params {
                        queue.push_back(children);
                    } else {
                        result.push(cue.clone());
                    }
                }
            }
        }
        result
    }

    pub async fn get_next_cue_id_by_id(&self, cue_id: &Uuid) -> Option<Uuid> {
        let model = self.read().await;
        let mut queue: VecDeque<(&Vec<Cue>, Option<&Cue>)> = VecDeque::from([(&model.cues, None)]);

        while let Some((cues, _parent)) = queue.pop_front() {
            for (index, cue) in cues.iter().enumerate() {
                if cue.id == *cue_id {
                    if index + 1 < cues.len() {
                        return Some(cues[index + 1].id);
                    } else {
                        return None;
                    }
                }

                if let CueParam::Group { children, .. } = &cue.params {
                    queue.push_back((children, Some(cue)));
                }
            }
        }
        None
    }

    pub async fn get_cue_sequence_by_id(&self, cue_id: &Uuid) -> Option<CueSequence> {
        let model = self.read().await;
        let mut queue: VecDeque<(&Vec<Cue>, Option<&Cue>)> = VecDeque::from([(&model.cues, None)]);

        while let Some((cues, parent)) = queue.pop_front() {
            for (index, cue) in cues.iter().enumerate() {
                if cue.id == *cue_id {
                    if let Some(parent_cue) = parent {
                        if let CueParam::Group { mode, .. } = &parent_cue.params {
                            match mode {
                                GroupMode::Playlist { repeat } => {
                                    if (index + 1) == cues.len() && let Some(first_cue) = cues.first() {
                                        if *repeat {
                                            return Some(CueSequence::AutoFollow { target_id: Some(first_cue.id) });
                                        } else {
                                            return Some(CueSequence::DoNotContinue);
                                        }
                                    } else {
                                        return Some(CueSequence::AutoFollow { target_id: None })
                                    }
                                }
                                GroupMode::Concurrency => {
                                    return Some(CueSequence::DoNotContinue);
                                }
                            }
                        }
                    } else {
                        return Some(cue.sequence.clone())
                    }
                }

                if let CueParam::Group { children, .. } = &cue.params {
                    queue.push_back((children, Some(cue)));
                }
            }
        }
        None
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
            let asset_dir = self.model.read().await.settings.general.copy_assets_destination.clone();
            path.parent().map(|path| path.to_path_buf().join(asset_dir))
        } else {
            None
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
