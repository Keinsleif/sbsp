// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod command;
mod handle;
pub mod project;
mod guard;

use anyhow::anyhow;
pub use command::{InsertPosition, ModelCommand};
pub use handle::ShowModelHandle;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::{RwLock, broadcast, mpsc, watch};
use uuid::Uuid;

use crate::manager::guard::RollbackGuard;
use crate::manager::project::ProjectFile;
use crate::manager::project::ProjectStatus;
use crate::manager::project::ProjectType;
use crate::{
    BackendSettings,
    event::{BackendError, BackendEvent},
    model::{
        ShowModel,
        cue::{Cue, CueParam},
    },
};

const DEFAULT_PROJECT_FOLDER_MODEL_FILENAME: &str = "model.sbsp";

pub struct ShowModelManager {
    model: Arc<RwLock<ShowModel>>,
    settings_rx: watch::Receiver<BackendSettings>,
    command_rx: mpsc::Receiver<ModelCommand>,
    event_tx: broadcast::Sender<BackendEvent>,

    copy_assets_when_add: bool,
    project_status: Arc<RwLock<ProjectStatus>>,
    modify_status: Arc<AtomicBool>,
}

impl ShowModelManager {
    pub fn new(
        event_tx: broadcast::Sender<BackendEvent>,
        settings_rx: watch::Receiver<BackendSettings>,
    ) -> (Self, ShowModelHandle) {
        let (command_tx, command_rx) = mpsc::channel(32);
        let model = Arc::new(RwLock::new(ShowModel::default()));
        let project_status = Arc::new(RwLock::new(ProjectStatus::Unsaved));
        let modify_status = Arc::new(AtomicBool::new(false));
        let copy_assets_when_add = settings_rx.borrow().copy_assets_when_add;
        let manager = Self {
            model: model.clone(),
            settings_rx,
            command_rx,
            event_tx,
            copy_assets_when_add,
            project_status: project_status.clone(),
            modify_status: modify_status.clone(),
        };
        let handle = ShowModelHandle::new(model, command_tx, project_status, modify_status);

        (manager, handle)
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => self.process_command(command).await,
                Ok(_) = self.settings_rx.changed() => {
                    self.copy_assets_when_add = self.settings_rx.borrow().copy_assets_when_add;
                }
                else => break,
            }
        }
    }

    async fn process_command(&self, command: ModelCommand) {
        log::debug!("Model Manager received command: {:?}", command);
        match command {
            ModelCommand::UpdateCue(mut cue) => {
                let model_path_option = self.project_status.read().await.to_model_path_option();
                if let CueParam::Audio(audio_param) = &mut cue.params
                    && let Some(model_path) = model_path_option.as_ref()
                    && self.copy_assets_when_add
                {
                    let import_destination = {
                        let model = self.model.read().await;
                        model.settings.general.copy_assets_destination.clone()
                    };

                    let new_target = Self::import_asset_file(
                        &audio_param.target,
                        model_path,
                        &import_destination,
                    )
                    .await;
                    if let Ok(target) = new_target {
                        audio_param.target = target;
                    } // ignore failed to import asset. use absolute path
                }
                if let Err(e) = self.update_cue_by_id(&cue.id, cue.clone()).await {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::CueEdit {
                            message: format!("Failed to update cue, {}.", e),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    return;
                }
                self.modify_status.store(true, Ordering::Release);
                if let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                    cue_list: self.model.read().await.cue_list.clone(),
                }) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::AddCue { mut cue, position } => {
                let model_path_option = self.project_status.read().await.to_model_path_option();
                if self.is_cue_exists(&cue.id).await {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::CueEdit {
                            message: "Failed to add cue, id already exists.".into(),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    return;
                }
                if let CueParam::Audio(audio_param) = &mut cue.params
                    && let Some(model_path) = model_path_option.as_ref()
                    && self.copy_assets_when_add
                {
                    let import_destination = {
                        let model = self.model.read().await;
                        model.settings.general.copy_assets_destination.clone()
                    };
                    let new_target = Self::import_asset_file(
                        &audio_param.target,
                        model_path,
                        &import_destination,
                    )
                    .await;
                    if let Ok(target) = new_target {
                        audio_param.target = target;
                    } // ignore failed to import asset. use absolute path
                }
                if let Err(e) = self.insert_cues_at_position(vec![cue], position).await {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::CueEdit {
                            message: format!("Failed to add cue, {}.", e),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    return;
                }
                self.modify_status.store(true, Ordering::Release);
                if let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                    cue_list: self.model.read().await.cue_list.clone(),
                }) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::AddCues { cues, position } => {
                let model_path_option = self.project_status.read().await.to_model_path_option();
                let mut valid_cues = Vec::new();
                let mut valid_cue_ids = HashSet::new();

                for mut cue in cues {
                    if self.is_cue_exists(&cue.id).await {
                        if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                            error: BackendError::CueEdit {
                                message: "Failed to add cue, id already exists.".into(),
                            },
                        }) {
                            log::warn!("Failed to send event, {}", e);
                        }
                        continue;
                    } else if !valid_cue_ids.insert(cue.id) {
                        if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                            error: BackendError::CueEdit {
                                message: "Failed to add cue, duplicate id found.".into(),
                            },
                        }) {
                            log::warn!("Failed to send event, {}", e);
                        }
                        continue;
                    }
                    if let CueParam::Audio(audio_param) = &mut cue.params
                        && let Some(model_path) = model_path_option.as_ref()
                        && self.copy_assets_when_add
                    {
                        let import_destination = {
                            let model = self.model.read().await;
                            model.settings.general.copy_assets_destination.clone()
                        };
                        let new_target = Self::import_asset_file(
                            &audio_param.target,
                            model_path,
                            &import_destination,
                        )
                        .await;
                        if let Ok(target) = new_target {
                            audio_param.target = target;
                        } // ignore failed to import asset. use absolute path
                    }
                    valid_cues.push(cue);
                }
                if valid_cues.is_empty() {
                    return;
                }
                if let Err(e) = self
                    .insert_cues_at_position(valid_cues, position.clone())
                    .await
                {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::CueEdit {
                            message: format!("Failed to add cues, {}.", e),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    return;
                }

                self.modify_status.store(true, Ordering::Release);
                if let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                    cue_list: self.model.read().await.cue_list.clone(),
                }) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::RemoveCue { cue_id } => {
                let removed_ids = self.remove_cues_by_id(HashSet::from([cue_id])).await;
                if removed_ids.is_empty() {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::CueEdit {
                            message: "Failed to remove cue, id not found.".to_string(),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                } else {
                    if let Err(e) = self.event_tx.send(BackendEvent::CueRemoved {
                        cue_ids: removed_ids,
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    self.modify_status.store(true, Ordering::Release);
                    if let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                        cue_list: self.model.read().await.cue_list.clone(),
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                }
            }
            ModelCommand::RemoveCues { cue_ids } => {
                let removed_ids = self.remove_cues_by_id(cue_ids.clone()).await;
                if removed_ids.is_empty() {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::CueEdit {
                            message: "Failed to remove cues, id not found.".to_string(),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                } else {
                    if let Err(e) = self.event_tx.send(BackendEvent::CueRemoved {
                        cue_ids: removed_ids,
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    self.modify_status.store(true, Ordering::Release);
                    if let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                        cue_list: self.model.read().await.cue_list.clone(),
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                }
            }
            ModelCommand::MoveCue { cue_id, position } => {
                if let Err(e) = self.move_cues_at_position(HashSet::from([cue_id]), position).await {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::CueEdit {
                            message: format!("Failed to move cue, {}.", e),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    return;
                }
                self.modify_status.store(true, Ordering::Release);
                if let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                    cue_list: self.model.read().await.cue_list.clone(),
                }) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::MoveCues { cue_ids, position } => {
                if let Err(e) = self.move_cues_at_position(cue_ids, position).await {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::CueEdit {
                            message: format!("Failed to move cues, {}.", e),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    return;
                }
                self.modify_status.store(true, Ordering::Release);

                if let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                    cue_list: self.model.read().await.cue_list.clone(),
                }) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::RenumberCues {
                cues,
                start_from,
                increment,
                prefix,
                suffix,
            } => {
                let mut model = self.model.write().await;
                let mut targets: HashSet<Uuid> = cues.into_iter().collect();
                let mut number = start_from;
                let prefix = prefix.unwrap_or_default();
                let suffix = suffix.unwrap_or_default();

                let mut queue: VecDeque<Vec<Uuid>> = VecDeque::from([model.cue_list.root_ids.clone()]);

                'outer: while let Some(cue_ids) = queue.pop_front() {
                    for cue_id in cue_ids {
                        if let Some(cue) = model.cue_list.cues.get_mut(&cue_id) {
                            if targets.remove(&cue_id) {
                                cue.number = format!("{}{}{}", &prefix, number, &suffix);
                                number += increment;
                                if targets.is_empty() {
                                    break 'outer;
                                }
                            }
                            if let CueParam::Group { children, .. } = &cue.params {
                                queue.push_back(children.clone());
                            }
                        }
                    }

                    if targets.is_empty() {
                        break;
                    }
                }

                if number != start_from {
                    self.modify_status.store(true, Ordering::Release);
                    if let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                        cue_list: model.cue_list.clone(),
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                }
            }
            ModelCommand::UpdateModelName(new_name) => {
                let mut model = self.model.write().await;
                model.name = new_name.clone();
                self.modify_status.store(true, Ordering::Release);
                if let Err(e) = self
                    .event_tx
                    .send(BackendEvent::ModelNameUpdated { new_name })
                {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::UpdateSettings(new_settings) => {
                let mut model = self.model.write().await;
                // TODO setting validation
                model.settings = *new_settings.clone();
                self.modify_status.store(true, Ordering::Release);
                if let Err(e) = self
                    .event_tx
                    .send(BackendEvent::SettingsUpdated { new_settings })
                {
                    log::warn!("Failed to send event, {}", e);
                }
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
                if let Err(e) = self.event_tx.send(BackendEvent::ShowModelReset {
                    model: self.read().await.clone(),
                }) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::Save => {
                let event = if let ProjectStatus::Saved { project_type, path } =
                    &*self.project_status.read().await
                {
                    match self.save_to_file(path, project_type).await {
                        Err(error) => {
                            log::error!("Failed to save model file: {}", error);
                            BackendEvent::OperationFailed {
                                error: BackendError::SaveToFile {
                                    path: path.to_path_buf(),
                                    message: error.to_string(),
                                },
                            }
                        }
                        Ok(modified) => {
                            if modified {
                                let _ = self.event_tx.send(BackendEvent::CueListUpdated {
                                    cue_list: self.model.read().await.cue_list.clone(),
                                });
                            }
                            self.modify_status.store(false, Ordering::Release);
                            BackendEvent::ShowModelSaved {
                                project_type: project_type.clone(),
                                path: path.to_path_buf(),
                            }
                        }
                    }
                } else {
                    log::warn!(
                        "Save command issued, but no file path is set. Use SaveToFile first."
                    );
                    BackendEvent::OperationFailed { error: BackendError::SaveToFile { path: PathBuf::new(), message: "Save command issued, but no file path is set. Use SaveToFile first.".to_string() } }
                };
                if let Err(e) = self.event_tx.send(event) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::SaveToFile(path) => {
                let event = match self.save_to_file(&path, &ProjectType::SingleFile).await {
                    Err(error) => {
                        log::error!("Failed to save model file: {}", error);
                        BackendEvent::OperationFailed {
                            error: BackendError::SaveToFile {
                                path,
                                message: error.to_string(),
                            },
                        }
                    }
                    Ok(modified) => {
                        if modified
                            && let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                                cue_list: self.model.read().await.cue_list.clone(),
                            })
                        {
                            log::warn!("Failed to send event, {}", e);
                        }
                        self.modify_status.store(false, Ordering::Release);
                        {
                            let mut project_status = self.project_status.write().await;
                            *project_status = ProjectStatus::Saved {
                                project_type: ProjectType::SingleFile,
                                path: path.clone(),
                            };
                        }
                        BackendEvent::ShowModelSaved {
                            project_type: ProjectType::SingleFile,
                            path,
                        }
                    }
                };
                if let Err(e) = self.event_tx.send(event) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::ExportToFolder(path) => {
                if !path.is_dir() {
                    if let Err(e) = self.event_tx.send(BackendEvent::OperationFailed {
                        error: BackendError::ExportToFolder {
                            path,
                            message: "Failed to export to folder. path is not directory."
                                .to_string(),
                        },
                    }) {
                        log::warn!("Failed to send event, {}", e);
                    }
                    return;
                }
                let model_file_path = path.join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME);
                let event = match self
                    .save_to_file(&model_file_path, &ProjectType::ProjectFolder)
                    .await
                {
                    Err(error) => {
                        log::error!("Failed to export model to folder: {}", error);
                        BackendEvent::OperationFailed {
                            error: BackendError::SaveToFile {
                                path: model_file_path.clone(),
                                message: error.to_string(),
                            },
                        }
                    }
                    Ok(modified) => {
                        if modified
                            && let Err(e) = self.event_tx.send(BackendEvent::CueListUpdated {
                                cue_list: self.model.read().await.cue_list.clone(),
                            })
                        {
                            log::warn!("Failed to send event, {}", e);
                        }
                        self.modify_status.store(false, Ordering::Release);
                        {
                            let mut project_status = self.project_status.write().await;
                            *project_status = ProjectStatus::Saved {
                                project_type: ProjectType::ProjectFolder,
                                path: model_file_path.clone(),
                            };
                        }
                        BackendEvent::ShowModelSaved {
                            project_type: ProjectType::ProjectFolder,
                            path: model_file_path,
                        }
                    }
                };
                if let Err(e) = self.event_tx.send(event) {
                    log::warn!("Failed to send event, {}", e);
                }
            }
            ModelCommand::LoadFromFile(path) => {
                let event = match self.load_from_file(path.as_path()).await {
                    Err(error) => {
                        log::error!("Failed to load model file: {}", error);
                        BackendEvent::OperationFailed {
                            error: BackendError::LoadFromFile {
                                path,
                                message: error.to_string(),
                            },
                        }
                    }
                    Ok(project_type) => {
                        self.modify_status.store(false, Ordering::Release);
                        {
                            let mut project_status = self.project_status.write().await;
                            *project_status = ProjectStatus::Saved {
                                project_type: project_type.clone(),
                                path: path.clone(),
                            };
                        }
                        let model = self.read().await.clone();
                        BackendEvent::ShowModelLoaded {
                            model,
                            project_type,
                            path,
                        }
                    }
                };
                if let Err(e) = self.event_tx.send(event) {
                    log::warn!("Failed to send event, {}", e);
                }
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

    async fn is_cue_exists(&self, cue_id: &Uuid) -> bool {
        self.read().await.cue_list.cues.contains_key(cue_id)
    }

    async fn remove_cues_by_id(&self, cue_ids: HashSet<Uuid>) -> HashSet<Uuid> {
        let mut model = self.model.write().await;
        let mut removed_cues = HashSet::new();
        let mut queue: VecDeque<_> = cue_ids.into_iter().collect();

        while let Some(target_id) = queue.pop_front() {
            if let Some(parent_id) = model.cue_list.cues.get(&target_id).and_then(|cue| cue.parent_id) {
                if let Some(parent) = model.cue_list.cues.get_mut(&parent_id) && let CueParam::Group { children, .. } = &mut parent.params {
                    children.retain(|&id| id != target_id);
                }
            } else {
                model.cue_list.root_ids.retain(|&id| id != target_id);
            }
            let Some(target_cue) = model.cue_list.cues.remove(&target_id) else {
                continue;
            };
            if let CueParam::Group { children, .. } = &target_cue.params {
                queue.extend(children);
            }
            removed_cues.insert(target_id);
        }
        removed_cues
    }

    async fn update_cue_by_id(&self, cue_id: &Uuid, new_cue: Cue) -> anyhow::Result<()> {
        let mut model = self.model.write().await;

        if let Some(cue) = model.cue_list.cues.get_mut(cue_id) {
            // id, parent_id, params.children(group) is not modifiable by Update Command
            cue.number = new_cue.number;
            cue.name = new_cue.name;
            cue.notes = new_cue.notes;
            cue.color = new_cue.color;
            cue.pre_wait = new_cue.pre_wait;
            cue.chain = new_cue.chain;
            match (&mut cue.params, new_cue.params) {
                (CueParam::Audio(p), CueParam::Audio(new_p)) => {
                    *p = new_p;
                }
                (CueParam::Wait(p), CueParam::Wait(new_p)) => {
                    *p = new_p;
                }
                (CueParam::Fade(p), CueParam::Fade(new_p)) => {
                    *p = new_p;
                }
                (CueParam::Start(p), CueParam::Start(new_p)) => {
                    *p = new_p;
                }
                (CueParam::Stop(p), CueParam::Stop(new_p)) => {
                    *p = new_p;
                }
                (CueParam::Pause(p), CueParam::Pause(new_p)) => {
                    *p = new_p;
                }
                (CueParam::Load(p), CueParam::Load(new_p)) => {
                    *p = new_p;
                }
                (CueParam::Group { base, .. }, CueParam::Group { base: new_base, .. }) => {
                    // modify only base on Group cue param modify. 
                    *base = new_base;
                }
                _ => {}
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("cue not found. id={}", cue_id))
        }
    }

    async fn move_cues_at_position(&self, mut cue_ids: HashSet<Uuid>, position: InsertPosition) -> anyhow::Result<()> {
        let mut model_guard = self.model.write().await;
        let mut model = RollbackGuard::from(&mut model_guard.cue_list);
        let mut move_ids = Vec::new();

        let mut queue: VecDeque<_> = model.cue_list.root_ids.iter().rev().copied().collect();
        while let Some(target_id) = queue.pop_back() {
            if cue_ids.remove(&target_id) {
                move_ids.push(target_id);

                if cue_ids.is_empty() {
                    break;
                }
            } else if let Some(cue) = model.cue_list.cues.get(&target_id) && let CueParam::Group { children, .. } = &cue.params {
                queue.extend(children.iter().rev());
            }
        }
        if move_ids.is_empty() {
            return Err(anyhow::anyhow!("No valid cues found to move."));
        }

        let move_set: HashSet<Uuid> = move_ids.iter().copied().collect();

        for id in &move_ids {
            if let Some(parent_id) = model.cue_list.cues.get(id).and_then(|cue| cue.parent_id) {
                if let Some(parent) = model.cue_list.cues.get_mut(&parent_id) && let CueParam::Group { children, .. } = &mut parent.params {
                    children.retain(|x| !move_set.contains(x));
                }
            } else {
                model.cue_list.root_ids.retain(|x| !move_set.contains(x));
            }
        }

        let (new_parent_id, start_idx) = match position {
            InsertPosition::Before { target } => {
                if let Some(parent_id) = model.cue_list.cues.get(&target).and_then(|cue| cue.parent_id) {
                    if let Some(parent) = model.cue_list.cues.get(&parent_id) && let CueParam::Group { children, .. } = &parent.params
                    && let Some(index) = children.iter().position(|&id| id == target) {
                        (Some(parent_id), index)
                    } else {
                        return Err(anyhow::anyhow!("Invalid tree structure"));
                    }
                } else {
                    if let Some(index) = model.cue_list.root_ids.iter().position(|&id| id == target) {
                        (None, index)
                    } else {
                        return Err(anyhow::anyhow!("Invalid tree structure"));
                    }
                }
            }
            InsertPosition::After { target } => {
                if let Some(parent_id) = model.cue_list.cues.get(&target).and_then(|cue| cue.parent_id) {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id) && let CueParam::Group { children, .. } = &mut parent.params
                    && let Some(mut index) = children.iter().position(|&id| id == target) {
                        index += 1;
                        (Some(parent_id), index)
                    } else {
                        return Err(anyhow::anyhow!("Invalid tree structure"));
                    }
                } else {
                    if let Some(mut index) = model.cue_list.root_ids.iter().position(|&id| id == target) {
                        index += 1;
                        (None, index)
                    } else {
                        return Err(anyhow::anyhow!("Invalid tree structure"));
                    }
                }
            }
            InsertPosition::Inside { target, index } => {
                if let Some(parent_id) = target {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id) && let CueParam::Group { children, .. } = &mut parent.params {
                        if index <= children.len() {
                            (Some(parent_id), index)
                        } else {
                            return Err(anyhow::anyhow!("insert index out of range."))
                        }
                    } else {
                        return Err(anyhow::anyhow!("target id not found."))
                    }
                } else if index <= model.cue_list.root_ids.len() {
                    (None, index)
                } else {
                    return Err(anyhow::anyhow!("insert index out of range."));
                }
            }
            InsertPosition::Last => {
                (None, model.cue_list.root_ids.len())
            }
        };

        let mut ancestor_id = new_parent_id;
        while let Some(ancestor) = ancestor_id {
            if move_set.contains(&ancestor) {
                return Err(anyhow::anyhow!("Cannot move a cue into its own descendant."));
            }
            ancestor_id = model.cue_list.cues.get(&ancestor).and_then(|cue| cue.parent_id);
        }

        for id in &move_ids {
            if let Some(cue) = model.cue_list.cues.get_mut(id) {
                cue.parent_id = new_parent_id;
            }
        }

        if let Some(pid) = new_parent_id {
            if let Some(parent) = model.cue_list.cues.get_mut(&pid) && let CueParam::Group { children, .. } = &mut parent.params {
                children.splice(start_idx..start_idx, move_ids);
            }
        } else {
            model.cue_list.root_ids.splice(start_idx..start_idx, move_ids);
        }

        model.success = true;
        Ok(())
    }

    async fn insert_cues_at_position(
        &self,
        insert_cues: Vec<Cue>,
        position: InsertPosition,
    ) -> anyhow::Result<()> {
        let insert_ids = insert_cues.iter().map(|cue| cue.id);
        let mut model = self.model.write().await;
        match position {
            InsertPosition::Before { target } => {
                if let Some(parent_id) = model.cue_list.cues.get(&target).and_then(|cue| cue.parent_id) {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id) && let CueParam::Group { children, .. } = &mut parent.params
                        && let Some(index) = children.iter().position(|&id| id == target) {
                            children.splice(index..index, insert_ids);
                            model.cue_list.cues.extend(insert_cues.into_iter().map(|mut cue| {
                                cue.parent_id = Some(parent_id);
                                if let CueParam::Group { children, .. } = &mut cue.params {
                                    children.clear();
                                }
                                (cue.id, cue)
                            }));
                            return Ok(());
                        }
                } else {
                    if let Some(index) = model.cue_list.root_ids.iter().position(|&id| id == target) {
                        model.cue_list.root_ids.splice(index..index, insert_ids);
                            model.cue_list.cues.extend(insert_cues.into_iter().map(|mut cue| {
                                cue.parent_id = None;
                                if let CueParam::Group { children, .. } = &mut cue.params {
                                    children.clear();
                                }
                                (cue.id, cue)
                            }));
                        return Ok(());
                    }
                }

                Err(anyhow::anyhow!("target id not found."))
            }
            InsertPosition::After { target } => {
                if let Some(parent_id) = model.cue_list.cues.get(&target).and_then(|cue| cue.parent_id) {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id) && let CueParam::Group { children, .. } = &mut parent.params
                        && let Some(mut index) = children.iter().position(|&id| id == target) {
                            index += 1;
                            children.splice(index..index, insert_ids);
                            model.cue_list.cues.extend(insert_cues.into_iter().map(|mut cue| {
                                cue.parent_id = Some(parent_id);
                                if let CueParam::Group { children, .. } = &mut cue.params {
                                    children.clear();
                                }
                                (cue.id, cue)
                            }));
                            return Ok(());
                        }
                } else {
                    if let Some(mut index) = model.cue_list.root_ids.iter().position(|&id| id == target) {
                        index += 1;
                        model.cue_list.root_ids.splice(index..index, insert_ids);
                            model.cue_list.cues.extend(insert_cues.into_iter().map(|mut cue| {
                                cue.parent_id = None;
                                if let CueParam::Group { children, .. } = &mut cue.params {
                                    children.clear();
                                }
                                (cue.id, cue)
                            }));
                        return Ok(());
                    }
                }
                Err(anyhow::anyhow!("target id not found."))
            }
            InsertPosition::Inside { target, index } => {
                if let Some(parent_id) = target {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id) && let CueParam::Group { children, .. } = &mut parent.params {
                        if index <= children.len() {
                            children.splice(index..index, insert_ids);
                            model.cue_list.cues.extend(insert_cues.into_iter().map(|mut cue| {
                                cue.parent_id = Some(parent_id);
                                if let CueParam::Group { children, .. } = &mut cue.params {
                                    children.clear();
                                }
                                (cue.id, cue)
                            }));
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!("insert index out of range."))
                        }
                    } else {
                        Err(anyhow::anyhow!("target id not found."))
                    }
                } else if index <= model.cue_list.root_ids.len() {
                    model.cue_list.root_ids.splice(index..index, insert_ids);
                    model.cue_list.cues.extend(insert_cues.into_iter().map(|mut cue| {
                        cue.parent_id = None;
                        if let CueParam::Group { children, .. } = &mut cue.params {
                            children.clear();
                        }
                        (cue.id, cue)
                    }));
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("insert index out of range."))
                }
            }
            InsertPosition::Last => {
                model.cue_list.root_ids.extend(insert_ids);
                model.cue_list.cues.extend(insert_cues.into_iter().map(|mut cue| {
                    cue.parent_id = None;
                    if let CueParam::Group { children, .. } = &mut cue.params {
                        children.clear();
                    }
                    (cue.id, cue)
                }));
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
            *model = project_file.model.try_into()?;
        }
        {
            let mut project_status = self.project_status.write().await;
            *project_status = ProjectStatus::Saved {
                project_type: project_file.project_type.clone(),
                path: path.to_path_buf(),
            };
        }

        log::info!("Show loaded from: {}", path.display());
        Ok(project_file.project_type)
    }

    pub async fn export_to_folder(&self, folder_path: &Path) -> Result<bool, anyhow::Error> {
        if folder_path.is_dir() {
            self.save_to_file(
                &folder_path.join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME),
                &ProjectType::ProjectFolder,
            )
            .await
        } else {
            Err(anyhow!("path is not directory."))
        }
    }

    pub async fn save_to_file(
        &self,
        path: &PathBuf,
        project_type: &ProjectType,
    ) -> Result<bool, anyhow::Error> {
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

                if let ProjectStatus::Saved {
                    project_type,
                    path: saved_path,
                } = &*project_status
                    && *project_type == ProjectType::ProjectFolder
                    && path != saved_path
                {
                    let mut model = self.model.write().await;

                    for cue in model.cue_list.cues.values_mut() {
                        if let CueParam::Audio(audio_param) = &mut cue.params
                            && let Some(parent) = saved_path.parent()
                        {
                            let asset_path = parent.join(&audio_param.target);
                            let new_path = Self::import_asset_file(
                                &asset_path,
                                project_dir,
                                &import_destination,
                            )
                            .await?;
                            audio_param.target = new_path;
                        }
                    }
                } else {
                    let mut model = self.model.write().await;

                    for cue in model.cue_list.cues.values_mut() {
                        if let CueParam::Audio(audio_param) = &mut cue.params
                            && audio_param.target.is_absolute()
                        {
                            let new_path = Self::import_asset_file(
                                &audio_param.target,
                                project_dir,
                                &import_destination,
                            )
                            .await?;
                            audio_param.target = new_path;
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
                model: model.clone().into(),
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
    use std::collections::HashMap;

use crate::{
        BackendSettings,
        event::BackendEvent,
        manager::{ProjectStatus, ProjectType, command::InsertPosition},
        model::{
            ShowModel,
            cue::{
                Cue, CueChain, CueColor, CueList, CueParam, audio::{AudioCueParam, Decibels, SoundType}
            },
            settings::ShowSettings,
        },
    };
    use tempfile::{NamedTempFile, tempdir};
    use tokio::sync::{broadcast, watch};
    use uuid::Uuid;

    use super::{ShowModelHandle, ShowModelManager};

    async fn setup_manager(
        initial_model: Option<ShowModel>,
        project_status: ProjectStatus,
    ) -> (ShowModelHandle, broadcast::Receiver<BackendEvent>) {
        let (event_tx, event_rx) = broadcast::channel::<BackendEvent>(32);
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
                cue_list: CueList { 
                    cues: HashMap::from([(cue_id,
                        Cue {
                            id: cue_id,
                            number: "1".into(),
                            name: Some("test cue".into()),
                            notes: "note".into(),
                            color: CueColor::None,
                            pre_wait: 0.0,
                            chain: CueChain::DoNotChain,
                            parent_id: None,
                            params: CueParam::Audio(AudioCueParam {
                                target: temp_target.path().to_path_buf(),
                                start_time: None,
                                fade_in_param: None,
                                end_time: None,
                                fade_out_param: None,
                                volume: Decibels::IDENTITY,
                                pan: 0.0,
                                repeat: false,
                                sound_type: SoundType::Streaming,
                                envelope: Vec::new(),
                            }),
                        })]
                    ),
                    root_ids: vec![cue_id],
                },
                settings: ShowSettings::default(),
            }),
            ProjectStatus::Saved {
                project_type: ProjectType::ProjectFolder,
                path: temp_dir.path().to_path_buf(),
            },
        )
        .await;

        let new_cue = Cue {
            id: cue_id,
            number: "1".into(),
            name: Some("test cue".into()),
            notes: "note".into(),
            color: CueColor::None,
            pre_wait: 0.0,
            chain: CueChain::DoNotChain,
            parent_id: None,
            params: CueParam::Audio(AudioCueParam {
                target: temp_target_after.path().to_path_buf(),
                start_time: None,
                fade_in_param: None,
                end_time: None,
                fade_out_param: None,
                volume: Decibels::IDENTITY,
                pan: 0.0,
                repeat: false,
                sound_type: SoundType::Streaming,
                envelope: Vec::new(),
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

        loop {
            if let Ok(BackendEvent::CueListUpdated { cue_list }) = event_rx.recv().await {
                assert_eq!(*cue_list.cues.get(&cue_id).unwrap(), estimated_new_cue);
                break;
            }
        }

        let model = model_handle.read().await;
        assert_eq!(*model.cue_list.cues.get(&cue_id).unwrap(), estimated_new_cue);
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
                cue_list: CueList::default(),
                settings: ShowSettings::default(),
            }),
            ProjectStatus::Saved {
                project_type: ProjectType::ProjectFolder,
                path: temp_dir.path().to_path_buf(),
            },
        )
        .await;

        let new_cue = Cue {
            id: cue_id,
            number: "1".into(),
            name: Some("test cue".into()),
            notes: "note".into(),
            color: CueColor::None,
            pre_wait: 0.0,
            chain: CueChain::DoNotChain,
            parent_id: None,
            params: CueParam::Audio(AudioCueParam {
                target: temp_target.path().to_path_buf(),
                start_time: None,
                fade_in_param: None,
                end_time: None,
                fade_out_param: None,
                volume: Decibels::IDENTITY,
                pan: 0.0,
                repeat: false,
                sound_type: SoundType::Streaming,
                envelope: Vec::new(),
            }),
        };
        model_handle
            .add_cue(
                new_cue.clone(),
                InsertPosition::Inside {
                    target: None,
                    index: 0,
                },
            )
            .await
            .unwrap();

        let estimated_audio_filename = temp_target.path().file_name().unwrap().to_str().unwrap();
        let estimated_audio_target = temp_dir.path().join(".").join(estimated_audio_filename);
        let mut estimated_new_cue = new_cue.clone();
        if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
            audio_param.target = [".", estimated_audio_filename].iter().collect();
        }

        loop {
            if let Ok(BackendEvent::CueListUpdated { cue_list }) = event_rx.recv().await {
                assert_eq!(*cue_list.cues.get(&cue_id).unwrap(), estimated_new_cue);
                break;
            }
        }

        let model = model_handle.read().await;
        assert_eq!(*model.cue_list.cues.get(&cue_id).unwrap(), estimated_new_cue);
        assert!(estimated_audio_target.exists());
        drop(temp_target);
        drop(temp_dir);
    }
}
