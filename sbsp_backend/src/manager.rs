mod handle;
mod command;
mod project;

use anyhow::anyhow;
pub use handle::ShowModelHandle;
pub use command::{ModelCommand, InsertPosition};
pub use project::{ProjectStatus, ProjectFile};

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::{RwLock, broadcast, mpsc, watch};
use uuid::Uuid;

use crate::{
    BackendSettings, event::{UiError, UiEvent}, model::{
        ProjectType, ShowModel, cue::{Cue, CueParam}
    }
};

const DEFAULT_PROJECT_FOLDER_MODEL_FILENAME: &str = "model.sbsp";

pub struct ShowModelManager {
    model: Arc<RwLock<ShowModel>>,
    settings_rx: watch::Receiver<BackendSettings>,
    command_rx: mpsc::Receiver<ModelCommand>,
    event_tx: broadcast::Sender<UiEvent>,

    project_status: Arc<RwLock<ProjectStatus>>,
    modify_status: Arc<AtomicBool>,
}

impl ShowModelManager {
    pub fn new(event_tx: broadcast::Sender<UiEvent>, settings_rx: watch::Receiver<BackendSettings>) -> (Self, ShowModelHandle) {
        let (command_tx, command_rx) = mpsc::channel(32);
        let model = Arc::new(RwLock::new(ShowModel::default()));
        let project_status = Arc::new(RwLock::new(ProjectStatus::Unsaved));
        let modify_status = Arc::new(AtomicBool::new(false));
        let manager = Self {
            model: model.clone(),
            settings_rx,
            command_rx,
            event_tx,
            project_status: project_status.clone(),
            modify_status: modify_status.clone(),
        };
        let handle = ShowModelHandle::new(
            model,
            command_tx,
            project_status,
            modify_status,
        );

        (manager, handle)
    }

    pub async fn run(mut self) {
        while let Some(command) = self.command_rx.recv().await {
            if let Err(e) = self.process_command(command).await {
                log::error!("Failed modifying show model: {}", e);
            }
        }
    }

    async fn process_command(&self, command: ModelCommand) -> anyhow::Result<()> {
        log::info!("Model Manager received command: {:?}", command);
        match command {
            ModelCommand::UpdateCue(cue) => {
                let copy_assets_when_add = {
                    let settings = self.settings_rx.borrow();
                    settings.copy_assets_when_add
                };
                let model_path_option = self.project_status.read().await.to_model_path_option();
                let mut new_cue = cue.clone();
                if let CueParam::Audio(audio_param) = &mut new_cue.params
                    && let Some(model_path) = model_path_option.as_ref()
                    && copy_assets_when_add
                {
                    let import_destination = {
                        let model = self.model.read().await;
                        model.settings.general.copy_assets_destination.clone()
                    };

                    let new_target = Self::import_asset_file(&audio_param.target, model_path, &import_destination)
                        .await;
                    if let Ok(target) = new_target {
                        audio_param.target = target;
                    } // ignore failed to import asset. use absolute path
                }
                let event = if let Err(e) = self.update_cue_by_id(&cue.id, new_cue.clone()).await {
                    UiEvent::OperationFailed {
                        error: UiError::CueEdit {
                            message: format!("Failed to update cue. {}", e),
                        },
                    }
                } else {
                    self.modify_status.store(true, Ordering::Release);
                    UiEvent::CueListUpdated { cues: self.model.read().await.cues.clone() }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::AddCue { cue, position } => {
                let id_exists = {
                    let model = self.model.read().await;
                    model.cues.iter().any(|c| c.id == cue.id)
                };
                let copy_assets_when_add = {
                    let settings = self.settings_rx.borrow();
                    settings.copy_assets_when_add
                };
                let model_path_option = self.project_status.read().await.to_model_path_option();
                let event = if id_exists {
                    UiEvent::OperationFailed {
                        error: UiError::CueEdit {
                            message: format!("Cue already exist: cue_id={}", cue.id),
                        },
                    }
                } else {
                    let mut new_cue = cue.clone();
                    if let CueParam::Audio(audio_param) = &mut new_cue.params
                        && let Some(model_path) = model_path_option.as_ref()
                        && copy_assets_when_add
                    {
                        let import_destination = {
                            let model = self.model.read().await;
                            model.settings.general.copy_assets_destination.clone()
                        };
                        let new_target = Self::import_asset_file(&audio_param.target, model_path, &import_destination)
                            .await;
                        if let Ok(target) = new_target {
                            audio_param.target = target;
                        } // ignore failed to import asset. use absolute path
                    }
                    if let Err(e) = self.insert_cue_at_position(new_cue, position).await {
                        UiEvent::OperationFailed { error: UiError::CueEdit { message: format!("Failed to add cue. {}", e) } }
                    } else {
                        self.modify_status.store(true, Ordering::Release);
                        UiEvent::CueListUpdated { cues: self.model.read().await.cues.clone() }
                    }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::AddCues { cues, position } => {
                let copy_assets_when_add = {
                    let settings = self.settings_rx.borrow();
                    settings.copy_assets_when_add
                };
                let model_path_option = self.project_status.read().await.to_model_path_option();
                let mut added_cues: Vec<Cue> = Vec::new();
                for cue in cues.iter() {
                    let id_exists = {
                        let model = self.model.read().await;
                        model.cues.iter().any(|c| c.id == cue.id)
                    };
                    if id_exists {
                        self.event_tx.send(UiEvent::OperationFailed {
                            error: UiError::CueEdit {
                                message: format!("Cue already exist. cue_id={}", cue.id),
                            },
                        })?;
                    } else {
                        let mut new_cue = cue.clone();
                        if let CueParam::Audio(audio_param) = &mut new_cue.params
                            && let Some(model_path) = model_path_option.as_ref()
                            && copy_assets_when_add
                        {
                            let import_destination = {
                                let model = self.model.read().await;
                                model.settings.general.copy_assets_destination.clone()
                            };
                            let new_target = Self::import_asset_file(&audio_param.target, model_path, &import_destination)
                                .await;
                            if let Ok(target) = new_target {
                                audio_param.target = target;
                            } // ignore failed to import asset. use absolute path
                        }

                        let result = if added_cues.is_empty() {
                            self.insert_cue_at_position(new_cue.clone(), position.clone()).await
                        } else {
                            self.insert_cue_at_position(new_cue.clone(), InsertPosition::After { target: added_cues.last().unwrap().id }).await
                        };

                        if let Err(e) = result {
                            let _ = self.event_tx.send(UiEvent::OperationFailed { error: UiError::CueEdit { message: format!("Failed to add cue. {}", e) } });
                        } else {
                            added_cues.push(new_cue.clone());
                        }
                    }
                }
                if !added_cues.is_empty() {
                    self.modify_status.store(true, Ordering::Release);
                    self.event_tx.send(UiEvent::CueListUpdated { cues: self.model.read().await.cues.clone() })?;
                }
                Ok(())
            }
            ModelCommand::RemoveCue { cue_id } => {
                let event = if self.remove_cue_by_id(&cue_id).await.is_none() {
                    UiEvent::OperationFailed { error: UiError::CueEdit { message: "Failed to remove cue. id not found.".into() } }
                } else {
                    self.event_tx.send(UiEvent::CueRemoved { cue_id })?;
                    self.modify_status.store(true, Ordering::Release);
                    UiEvent::CueListUpdated { cues: self.model.read().await.cues.clone() }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::MoveCue { cue_id, position } => {
                let event = if let Some(cue) = self.remove_cue_by_id(&cue_id).await {
                    if let Err(e) = self.insert_cue_at_position(cue, position).await {
                        UiEvent::OperationFailed { error: UiError::CueEdit { message: format!("Failed to mov cue. {}", e) } }
                    } else {
                        self.modify_status.store(true, Ordering::Release);
                        UiEvent::CueListUpdated { cues: self.model.read().await.cues.clone() }
                    }
                } else {
                    UiEvent::OperationFailed {
                        error: UiError::CueEdit {
                            message: format!("Cue already exist: cue_id={}", cue_id),
                        },
                    }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::RenumberCues {
                cues,
                start_from,
                increment,
            } => {
                let mut model = self.model.write().await;
                let mut number = start_from;
                let targets: HashSet<Uuid> = cues.into_iter().collect();
                for cue in model.cues.iter_mut() {
                    if targets.contains(&cue.id) {
                        cue.number = number.to_string();
                        number += increment;
                    }
                }
                if number != start_from {
                    self.modify_status.store(true, Ordering::Release);
                    self.event_tx.send(UiEvent::CueListUpdated {
                        cues: model.cues.clone(),
                    })?;
                }
                Ok(())
            }
            ModelCommand::UpdateModelName(new_name) => {
                let mut model = self.model.write().await;
                model.name = new_name.clone();
                self.modify_status.store(true, Ordering::Release);
                self.event_tx.send(UiEvent::ModelNameUpdated { new_name })?;
                Ok(())
            }
            ModelCommand::UpdateSettings(new_settings) => {
                let mut model = self.model.write().await;
                // TODO setting validation
                model.settings = *new_settings.clone();
                self.modify_status.store(true, Ordering::Release);
                self.event_tx
                    .send(UiEvent::SettingsUpdated { new_settings })?;
                Ok(())
            }
            ModelCommand::Reset => {
                {
                    let mut model = self.model.write().await;
                    *model = ShowModel::default();
                }
                self.modify_status.store(false, Ordering::Release);
                {
                    let mut project_status_lock = self.project_status.write().await;
                    *project_status_lock = ProjectStatus::Unsaved;
                }
                self.event_tx.send(UiEvent::ShowModelReset {
                    model: self.read().await.clone(),
                })?;
                Ok(())
            }
            ModelCommand::Save => {
                let event = if let ProjectStatus::Saved { project_type, path } = &*self.project_status.read().await {
                    match self.save_to_file(path, project_type).await {
                        Err(error) => {
                            log::error!("Failed to save model file: {}", error);
                            UiEvent::OperationFailed {
                                error: UiError::FileSave {
                                    path: path.to_path_buf(),
                                    message: error.to_string(),
                                },
                            }
                        }
                        Ok(modified) => {
                            if modified {
                                let _ = self.event_tx.send(UiEvent::CueListUpdated { cues: self.model.read().await.cues.clone() });
                            }
                            self.modify_status.store(false, Ordering::Release);
                            UiEvent::ShowModelSaved {
                                project_type: project_type.clone(),
                                path: path.to_path_buf(),
                            }
                        }
                    }
                } else {
                    log::warn!(
                        "Save command issued, but no file path is set. Use SaveToFile first."
                    );
                    UiEvent::OperationFailed { error: UiError::FileSave { path: PathBuf::new(), message: "Save command issued, but no file path is set. Use SaveToFile first.".to_string() } }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::SaveToFile(path) => {
                let event = match self.save_to_file(&path, &ProjectType::SingleFile).await {
                    Err(error) => {
                        log::error!("Failed to save model file: {}", error);
                        UiEvent::OperationFailed {
                            error: UiError::FileSave {
                                path,
                                message: error.to_string(),
                            },
                        }
                    }
                    Ok(modified) => {
                        if modified {
                            let _ = self.event_tx.send(UiEvent::CueListUpdated { cues: self.model.read().await.cues.clone() });
                        }
                        self.modify_status.store(false, Ordering::Release);
                        {
                            let mut project_status = self.project_status.write().await;
                            *project_status = ProjectStatus::Saved { project_type: ProjectType::SingleFile, path: path.clone() };
                        }
                        UiEvent::ShowModelSaved { project_type: ProjectType::SingleFile, path }
                    }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::ExportToFolder(path) => {
                if !path.is_dir() {
                    anyhow::bail!("Failed to export to folder. path is not directory.");
                }
                let model_file_path = path.join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME);
                let event = match self.save_to_file(&model_file_path, &ProjectType::ProjectFolder).await {
                    Err(error) => {
                        log::error!("Failed to export model to folder: {}", error);
                        UiEvent::OperationFailed {
                            error: UiError::FileSave {
                                path: model_file_path.clone(),
                                message: error.to_string(),
                            },
                        }
                    }
                    Ok(modified) => {
                        if modified {
                            let _ = self.event_tx.send(UiEvent::CueListUpdated { cues: self.model.read().await.cues.clone() });
                        }
                        self.modify_status.store(false, Ordering::Release);
                        {
                            let mut project_status = self.project_status.write().await;
                            *project_status = ProjectStatus::Saved { project_type: ProjectType::ProjectFolder, path: model_file_path.clone() };
                        }
                        UiEvent::ShowModelSaved { project_type: ProjectType::ProjectFolder, path: model_file_path }
                    }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::LoadFromFile(path) => {
                let event = match self.load_from_file(path.as_path()).await {
                    Err(error) => {
                        log::error!("Failed to load model file: {}", error);
                        UiEvent::OperationFailed {
                            error: UiError::FileLoad {
                                path,
                                message: error.to_string(),
                            },
                        }
                    }
                    Ok(project_type) => {
                        self.modify_status.store(false, Ordering::Release);
                        {
                            let mut project_status = self.project_status.write().await;
                            *project_status = ProjectStatus::Saved { project_type: project_type.clone(), path: path.clone() };
                        }
                        let model = self.read().await.clone();
                        UiEvent::ShowModelLoaded { model, project_type, path }
                    }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
        }
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, ShowModel> {
        self.model.read().await
    }

    #[cfg(test)]
    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, ShowModel> {
        self.model.write().await
    }

    async fn remove_cue_by_id(&self, cue_id: &Uuid) -> Option<Cue> {
        let mut model = self.model.write().await;
        let mut queue: VecDeque<&mut Vec<Cue>> = VecDeque::from([&mut model.cues]);

        while let Some(cues) = queue.pop_front() {
            for (index, cue) in cues.iter().enumerate() {
                if cue.id == *cue_id {
                    return Some(cues.remove(index));
                }
            }

            for cue in cues.iter_mut() {
                if let CueParam::Group { children, .. } = &mut cue.params {
                    queue.push_back(children);
                }
            }
        }
        None
    }

    async fn update_cue_by_id(&self, cue_id: &Uuid, new_cue: Cue) -> Result<(), String> {
        let mut model = self.model.write().await;
        let mut queue: VecDeque<&mut Vec<Cue>> = VecDeque::from([&mut model.cues]);

        while let Some(cues) = queue.pop_front() {
            for cue in cues.iter_mut() {
                if cue.id == *cue_id {
                    *cue = new_cue;
                    return Ok(());
                }
            }

            for cue in cues.iter_mut() {
                if let CueParam::Group { children, .. } = &mut cue.params {
                    queue.push_back(children);
                }
            }
        }
        Err(format!("cue not found. id={}", cue_id))
    }

    async fn insert_cue_at_position(&self, insert_cue: Cue, position: InsertPosition) -> Result<(), String> {
        let mut model = self.model.write().await;
        match position {
            InsertPosition::Before { target } => {
                let mut queue: VecDeque<&mut Vec<Cue>> = VecDeque::from([&mut model.cues]);

                while let Some(cues) = queue.pop_front() {
                    for (index, cue) in cues.iter().enumerate() {
                        if cue.id == target {
                            cues.insert(index, insert_cue);
                            return Ok(())
                        }
                    }

                    for cue in cues.iter_mut() {
                        if let CueParam::Group { children, .. } = &mut cue.params {
                            queue.push_back(children);
                        }
                    }
                }
                Err("target id not found.".into())
            }
            InsertPosition::After { target } => {
                let mut queue: VecDeque<&mut Vec<Cue>> = VecDeque::from([&mut model.cues]);

                while let Some(cues) = queue.pop_front() {
                    for (index, cue) in cues.iter().enumerate() {
                        if cue.id == target {
                            cues.insert(index + 1, insert_cue);
                            return Ok(());
                        }
                    }

                    for cue in cues.iter_mut() {
                        if let CueParam::Group { children, .. } = &mut cue.params {
                            queue.push_back(children);
                        }
                    }
                }
                Err("target id not found.".into())
            },
            InsertPosition::Inside { target, index } => {
                if let Some(parent_id) = target {
                    let mut queue: VecDeque<&mut Vec<Cue>> = VecDeque::from([&mut model.cues]);

                    while let Some(cues) = queue.pop_front() {
                        for cue in cues.iter_mut() {
                            if let CueParam::Group { children, .. } = &mut cue.params {
                                if cue.id == parent_id {
                                    if index <= children.len() {
                                        children.insert(index, insert_cue);
                                        return Ok(());
                                    } else {
                                        return Err("insert index out of range.".into());
                                    }
                                } else {
                                    queue.push_back(children);
                                }
                            }
                        }
                    }
                    Err("target id not found.".into())
                } else if index <= model.cues.len() {
                    model.cues.insert(index, insert_cue);
                    Ok(())
                } else {
                    Err("insert index out of range.".into())
                }
            },
            InsertPosition::Last => {
                model.cues.push(insert_cue);
                Ok(())
            }
        }
    }

    pub async fn load_from_file(&self, path: &Path) -> Result<ProjectType, anyhow::Error> {
        let content = tokio::fs::read_to_string(path).await?;

        let project_file: ProjectFile =
            tokio::task::spawn_blocking(move || serde_json::from_str(&content)).await??;

        {
            let mut model = self.model.write().await;
            *model = project_file.model;
        }
        {
            let mut project_status = self.project_status.write().await;
            *project_status = ProjectStatus::Saved{project_type: project_file.project_type.clone(), path: path.to_path_buf()};
        }

        log::info!("Show loaded from: {}", path.display());
        Ok(project_file.project_type)
    }

    pub async fn export_to_folder(&self, folder_path: &Path) -> Result<bool, anyhow::Error> {
        if folder_path.is_dir() {
            self.save_to_file(&folder_path.join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME), &ProjectType::ProjectFolder).await
        } else {
            Err(anyhow!("path is not directory."))
        }
    }

    pub async fn save_to_file(&self, path: &PathBuf, project_type: &ProjectType) -> Result<bool, anyhow::Error> {
        let mut model_modified = false;
        let project_status = self.project_status.read().await;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        if project_type == &ProjectType::ProjectFolder {
            if let Some(project_dir) = path.parent() {
                let import_destination = {
                    let model = self.model.read().await;
                    model.settings.general.copy_assets_destination.clone()
                };

                if let ProjectStatus::Saved { project_type, path: saved_path } = &*project_status && *project_type == ProjectType::ProjectFolder && path != saved_path {
                    let mut model = self.model.write().await;
                    let mut queue: VecDeque<&mut Vec<Cue>> = VecDeque::from([&mut model.cues]);

                    while let Some(cues) = queue.pop_front() {
                        for cue in cues.iter_mut() {
                            if let CueParam::Audio(audio_param) = &mut cue.params && let Some(parent) = saved_path.parent() {
                                let asset_path = parent.join(&audio_param.target);
                                let new_path = Self::import_asset_file(&asset_path, project_dir, &import_destination).await?;
                                audio_param.target = new_path;
                            }
                            if let CueParam::Group { children, .. } = &mut cue.params {
                                queue.push_back(children);
                            }
                        }
                    }
                } else {
                    let mut model = self.model.write().await;
                    let mut queue: VecDeque<&mut Vec<Cue>> = VecDeque::from([&mut model.cues]);

                    while let Some(cues) = queue.pop_front() {
                        for cue in cues.iter_mut() {
                            if let CueParam::Audio(audio_param) = &mut cue.params
                            && audio_param.target.is_absolute() {
                                let new_path = Self::import_asset_file(&audio_param.target, project_dir, &import_destination).await?;
                                audio_param.target = new_path;
                            }
                            if let CueParam::Group { children, .. } = &mut cue.params {
                                queue.push_back(children);
                            }
                        }
                    }
                }
                model_modified = true;
            } else {
                return Err(anyhow!("Invalid project folder path."));
            }
        }

        let project_file = {
            let model = self.model.read().await;
            ProjectFile {
                project_type: ProjectType::ProjectFolder,
                model: model.clone()
            }
        };

        let content =
            tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&project_file))
                .await??;

        tokio::fs::write(&path, content).await?;
        log::info!("Show saved to: {}", path.display());
        Ok(model_modified)
    }

    async fn import_asset_file(
        asset_path: &PathBuf,
        model_path: &Path,
        import_destination: &String,
    ) -> anyhow::Result<PathBuf> {
        log::info!("Import asset file started. file={:?}", asset_path);
        let audio_dir = model_path.join(import_destination);
        if !audio_dir.exists() {
            tokio::fs::create_dir_all(&audio_dir).await?;
        } else if audio_dir.is_file() {
            anyhow::bail!("Failed to copy asset to destination. destination is not directory");
        }
        let asset_file_name = asset_path.file_name().unwrap().to_str().unwrap();
        let dest_path = audio_dir.join(asset_file_name);
        if !dest_path.exists() {
            tokio::fs::copy(asset_path, &dest_path).await?;
        }
        Ok([import_destination.clone(), asset_file_name.to_string()]
            .iter()
            .collect())
    }

    #[cfg(test)]
    pub async fn set_project_status(&self, new_project_status: ProjectStatus) {
        let mut project_status = self.project_status.write().await;
        *project_status = new_project_status;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        BackendSettings, event::UiEvent, manager::{ProjectStatus, ProjectType, command::InsertPosition}, model::{
            ShowModel,
            cue::{
                Cue, CueParam, CueSequence,
                audio::{AudioCueParam, SoundType},
            },
            settings::ShowSettings,
        }
    };
    use tempfile::{NamedTempFile, tempdir};
    use tokio::sync::{broadcast, watch};
    use uuid::Uuid;

    use super::{ShowModelHandle, ShowModelManager};

    async fn setup_manager(
        initial_model: Option<ShowModel>,
        project_status: ProjectStatus,
    ) -> (ShowModelHandle, broadcast::Receiver<UiEvent>) {
        let (event_tx, event_rx) = broadcast::channel::<UiEvent>(32);
        let (_, settings_rx) = watch::channel(BackendSettings {
            copy_assets_when_add: true,
            ..Default::default()
        });
        let (model_manager, model_handle) = ShowModelManager::new(event_tx.clone(), settings_rx);
        if let Some(inital) = initial_model {
            let mut model_lock = model_manager.write().await;
            *model_lock = inital;
            drop(model_lock);
        }
        model_manager.set_project_status(project_status).await;
        tokio::spawn(model_manager.run());
        (model_handle, event_rx)
    }

    #[tokio::test]
    async fn update_cue() {
        let temp_dir = tempdir().unwrap();
        let temp_target = NamedTempFile::with_suffix(".mp3").unwrap();
        let temp_target_after = NamedTempFile::with_suffix(".wav").unwrap();
        let cue_id = Uuid::new_v4();
        let (model_handle, mut event_rx) = setup_manager(
            Some(ShowModel {
                name: "test".into(),
                cues: vec![Cue {
                    id: cue_id,
                    number: "1".into(),
                    name: Some("test cue".into()),
                    notes: "note".into(),
                    pre_wait: 0.0,
                    sequence: CueSequence::DoNotContinue,
                    params: CueParam::Audio(AudioCueParam {
                        target: temp_target.path().to_path_buf(),
                        start_time: None,
                        fade_in_param: None,
                        end_time: None,
                        fade_out_param: None,
                        volume: 0.0,
                        pan: 0.0,
                        repeat: false,
                        sound_type: SoundType::Streaming,
                    }),
                }],
                settings: ShowSettings::default()
            }),
            ProjectStatus::Saved {project_type: ProjectType::ProjectFolder, path: temp_dir.path().to_path_buf()},
        )
        .await;

        let new_cue = Cue {
            id: cue_id,
            number: "1".into(),
            name: Some("test cue".into()),
            notes: "note".into(),
            pre_wait: 0.0,
            sequence: CueSequence::DoNotContinue,
            params: CueParam::Audio(AudioCueParam {
                target: temp_target_after.path().to_path_buf(),
                start_time: None,
                fade_in_param: None,
                end_time: None,
                fade_out_param: None,
                volume: 0.0,
                pan: 0.0,
                repeat: false,
                sound_type: SoundType::Streaming,
            }),
        };
        model_handle.update_cue(new_cue.clone()).await.unwrap();

        let estimated_audio_filename = temp_target_after
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        let estimated_audio_target = temp_dir.path().join(".").join(estimated_audio_filename);
        let mut estimated_new_cue = new_cue.clone();
        if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
            audio_param.target = [".", estimated_audio_filename].iter().collect();
        }

        let updated_cue;
        loop {
            if let Ok(UiEvent::CueListUpdated { cues }) = event_rx.recv().await {
                updated_cue = cues[0].clone();
                break;
            }
        }
        assert_eq!(updated_cue, estimated_new_cue);

        let model = model_handle.read().await;
        assert_eq!(model.cues[0], estimated_new_cue);
        assert!(estimated_audio_target.exists());
        drop(temp_target);
        drop(temp_dir);
    }

    #[tokio::test]
    async fn add_cue() {
        let temp_dir = tempdir().unwrap();
        let temp_target = NamedTempFile::with_suffix(".mp3").unwrap();
        let cue_id = Uuid::new_v4();
        let (model_handle, mut event_rx) = setup_manager(
            Some(ShowModel {
                name: "test".into(),
                cues: vec![],
                settings: ShowSettings::default(),
            }),
            ProjectStatus::Saved {project_type: ProjectType::ProjectFolder, path: temp_dir.path().to_path_buf()},
        )
        .await;

        let new_cue = Cue {
            id: cue_id,
            number: "1".into(),
            name: Some("test cue".into()),
            notes: "note".into(),
            pre_wait: 0.0,
            sequence: CueSequence::DoNotContinue,
            params: CueParam::Audio(AudioCueParam {
                target: temp_target.path().to_path_buf(),
                start_time: None,
                fade_in_param: None,
                end_time: None,
                fade_out_param: None,
                volume: 0.0,
                pan: 0.0,
                repeat: false,
                sound_type: SoundType::Streaming,
            }),
        };
        model_handle.add_cue(new_cue.clone(), InsertPosition::Inside { target: None, index: 0 }).await.unwrap();

        let estimated_audio_filename = temp_target.path().file_name().unwrap().to_str().unwrap();
        let estimated_audio_target = temp_dir.path().join(".").join(estimated_audio_filename);
        let mut estimated_new_cue = new_cue.clone();
        if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
            audio_param.target = [".", estimated_audio_filename].iter().collect();
        }

        let added_cue;
        loop {
            if let Ok(UiEvent::CueListUpdated { cues }) = event_rx.recv().await {
                added_cue = cues[0].clone();
                break;
            }
        }
        assert_eq!(added_cue, estimated_new_cue);

        let model = model_handle.read().await;
        assert_eq!(model.cues[0], estimated_new_cue);
        assert!(estimated_audio_target.exists());
        drop(temp_target);
        drop(temp_dir);
    }
}
