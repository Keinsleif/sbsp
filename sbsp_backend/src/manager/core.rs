// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use sha2::Digest;
use tokio::sync::{RwLock, broadcast, mpsc, watch};
use uuid::Uuid;

use super::{
    InsertPosition, ModelCommand, ShowModelHandle,
    guard::RollbackGuard,
    project::{ProjectFile, ProjectStatus, ProjectType},
};
use crate::{
    BackendSettings,
    event::{BackendError, BackendEvent},
    model::{
        ShowModel,
        cue::{Cue, CueParam},
    },
};

pub const DEFAULT_PROJECT_FOLDER_MODEL_FILENAME: &str = "model.sbsp";

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
                self.import_cue_asset(&mut cue, model_path_option.as_deref())
                    .await;
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
                self.import_cue_asset(&mut cue, model_path_option.as_deref())
                    .await;

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
                    self.import_cue_asset(&mut cue, model_path_option.as_deref())
                        .await;

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
                if let Err(e) = self
                    .move_cues_at_position(HashSet::from([cue_id]), position)
                    .await
                {
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
                let mut renumbered = false;
                let prefix = prefix.unwrap_or_default();
                let suffix = suffix.unwrap_or_default();

                let mut stack: VecDeque<Uuid> = VecDeque::from(model.cue_list.root_ids.clone());
                let mut first_found_parent = None;

                'outer: while let Some(cue_id) = stack.pop_front() {
                    if let Some(cue) = model.cue_list.cues.get_mut(&cue_id) {
                        if let Some(first_found) = first_found_parent
                            && first_found != cue.parent_id
                        {
                            continue;
                        }
                        if targets.remove(&cue_id) {
                            if first_found_parent.is_none() {
                                first_found_parent = Some(cue.parent_id);
                            }
                            let new_number = format!("{}{}{}", prefix, number, suffix);
                            if cue.number != new_number {
                                cue.number = new_number;
                                renumbered = true;
                            }
                            number += increment;
                            if targets.is_empty() {
                                break 'outer;
                            }
                        }
                        if let CueParam::Group { children, .. } = &cue.params {
                            for child in children.iter().rev() {
                                stack.push_front(*child);
                            }
                        }
                    }

                    if targets.is_empty() {
                        break;
                    }
                }

                if renumbered {
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
                                project_type: *project_type,
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
                                project_type,
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

    async fn import_cue_asset(&self, cue: &mut Cue, model_path: Option<&Path>) {
        if let CueParam::Audio(audio_param) = &mut cue.params
            && let Some(model_path) = model_path
            && let Some(model_dir) = model_path.parent()
            && self.copy_assets_when_add
        {
            let import_destination = {
                let model = self.model.read().await;
                model.settings.general.copy_assets_destination.clone()
            };
            if let Ok(target) = import_asset_file(
                audio_param.target.clone(),
                model_dir.to_path_buf(),
                import_destination,
            )
            .await
            {
                audio_param.target = target; // ignore import failure: keep original path
            }
        }
    }

    async fn is_cue_exists(&self, cue_id: &Uuid) -> bool {
        self.read().await.cue_list.cues.contains_key(cue_id)
    }

    async fn remove_cues_by_id(&self, cue_ids: HashSet<Uuid>) -> HashSet<Uuid> {
        let mut model = self.model.write().await;
        let mut removed_cues = HashSet::new();
        let mut queue: VecDeque<_> = cue_ids.into_iter().collect();

        while let Some(target_id) = queue.pop_front() {
            if let Some(parent_id) = model
                .cue_list
                .cues
                .get(&target_id)
                .and_then(|cue| cue.parent_id)
            {
                if let Some(parent) = model.cue_list.cues.get_mut(&parent_id)
                    && let CueParam::Group { children, .. } = &mut parent.params
                {
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
            cue.cursor_advance_trigger_override = new_cue.cursor_advance_trigger_override;
            cue.treat_stop_as_completed = new_cue.treat_stop_as_completed;
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

    async fn move_cues_at_position(
        &self,
        mut cue_ids: HashSet<Uuid>,
        position: InsertPosition,
    ) -> anyhow::Result<()> {
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
            } else if let Some(cue) = model.cue_list.cues.get(&target_id)
                && let CueParam::Group { children, .. } = &cue.params
            {
                queue.extend(children.iter().rev());
            }
        }
        if move_ids.is_empty() {
            return Err(anyhow::anyhow!("No valid cues found to move."));
        }

        let move_set: HashSet<Uuid> = move_ids.iter().copied().collect();

        for id in &move_ids {
            if let Some(parent_id) = model.cue_list.cues.get(id).and_then(|cue| cue.parent_id) {
                if let Some(parent) = model.cue_list.cues.get_mut(&parent_id)
                    && let CueParam::Group { children, .. } = &mut parent.params
                {
                    children.retain(|x| !move_set.contains(x));
                }
            } else {
                model.cue_list.root_ids.retain(|x| !move_set.contains(x));
            }
        }

        let (new_parent_id, start_idx) = match position {
            InsertPosition::Before { target } => {
                if let Some(parent_id) = model
                    .cue_list
                    .cues
                    .get(&target)
                    .and_then(|cue| cue.parent_id)
                {
                    if let Some(parent) = model.cue_list.cues.get(&parent_id)
                        && let CueParam::Group { children, .. } = &parent.params
                        && let Some(index) = children.iter().position(|&id| id == target)
                    {
                        (Some(parent_id), index)
                    } else {
                        return Err(anyhow::anyhow!("Invalid tree structure"));
                    }
                } else {
                    if let Some(index) = model.cue_list.root_ids.iter().position(|&id| id == target)
                    {
                        (None, index)
                    } else {
                        return Err(anyhow::anyhow!("Invalid tree structure"));
                    }
                }
            }
            InsertPosition::After { target } => {
                if let Some(parent_id) = model
                    .cue_list
                    .cues
                    .get(&target)
                    .and_then(|cue| cue.parent_id)
                {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id)
                        && let CueParam::Group { children, .. } = &mut parent.params
                        && let Some(mut index) = children.iter().position(|&id| id == target)
                    {
                        index += 1;
                        (Some(parent_id), index)
                    } else {
                        return Err(anyhow::anyhow!("Invalid tree structure"));
                    }
                } else {
                    if let Some(mut index) =
                        model.cue_list.root_ids.iter().position(|&id| id == target)
                    {
                        index += 1;
                        (None, index)
                    } else {
                        return Err(anyhow::anyhow!("Invalid tree structure"));
                    }
                }
            }
            InsertPosition::Inside { target, index } => {
                if let Some(parent_id) = target {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id)
                        && let CueParam::Group { children, .. } = &mut parent.params
                    {
                        let idx = match index {
                            Some(idx) if idx <= children.len() => idx,
                            Some(_) => return Err(anyhow::anyhow!("insert index out of range.")),
                            None => children.len(),
                        };
                        (Some(parent_id), idx)
                    } else {
                        return Err(anyhow::anyhow!("target id not found."));
                    }
                } else {
                    let idx = match index {
                        Some(idx) if idx <= model.cue_list.root_ids.len() => idx,
                        Some(_) => return Err(anyhow::anyhow!("insert index out of range.")),
                        None => model.cue_list.root_ids.len(),
                    };
                    (None, idx)
                }
            }
        };

        let mut ancestor_id = new_parent_id;
        while let Some(ancestor) = ancestor_id {
            if move_set.contains(&ancestor) {
                return Err(anyhow::anyhow!(
                    "Cannot move a cue into its own descendant."
                ));
            }
            ancestor_id = model
                .cue_list
                .cues
                .get(&ancestor)
                .and_then(|cue| cue.parent_id);
        }

        for id in &move_ids {
            if let Some(cue) = model.cue_list.cues.get_mut(id) {
                cue.parent_id = new_parent_id;
            }
        }

        if let Some(pid) = new_parent_id {
            if let Some(parent) = model.cue_list.cues.get_mut(&pid)
                && let CueParam::Group { children, .. } = &mut parent.params
            {
                children.splice(start_idx..start_idx, move_ids);
            }
        } else {
            model
                .cue_list
                .root_ids
                .splice(start_idx..start_idx, move_ids);
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
                if let Some(parent_id) = model
                    .cue_list
                    .cues
                    .get(&target)
                    .and_then(|cue| cue.parent_id)
                {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id)
                        && let CueParam::Group { children, .. } = &mut parent.params
                        && let Some(index) = children.iter().position(|&id| id == target)
                    {
                        children.splice(index..index, insert_ids);
                        model
                            .cue_list
                            .cues
                            .extend(insert_cues.into_iter().map(|mut cue| {
                                cue.parent_id = Some(parent_id);
                                if let CueParam::Group { children, .. } = &mut cue.params {
                                    children.clear();
                                }
                                (cue.id, cue)
                            }));
                        return Ok(());
                    }
                } else {
                    if let Some(index) = model.cue_list.root_ids.iter().position(|&id| id == target)
                    {
                        model.cue_list.root_ids.splice(index..index, insert_ids);
                        model
                            .cue_list
                            .cues
                            .extend(insert_cues.into_iter().map(|mut cue| {
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
                if let Some(parent_id) = model
                    .cue_list
                    .cues
                    .get(&target)
                    .and_then(|cue| cue.parent_id)
                {
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id)
                        && let CueParam::Group { children, .. } = &mut parent.params
                        && let Some(mut index) = children.iter().position(|&id| id == target)
                    {
                        index += 1;
                        children.splice(index..index, insert_ids);
                        model
                            .cue_list
                            .cues
                            .extend(insert_cues.into_iter().map(|mut cue| {
                                cue.parent_id = Some(parent_id);
                                if let CueParam::Group { children, .. } = &mut cue.params {
                                    children.clear();
                                }
                                (cue.id, cue)
                            }));
                        return Ok(());
                    }
                } else {
                    if let Some(mut index) =
                        model.cue_list.root_ids.iter().position(|&id| id == target)
                    {
                        index += 1;
                        model.cue_list.root_ids.splice(index..index, insert_ids);
                        model
                            .cue_list
                            .cues
                            .extend(insert_cues.into_iter().map(|mut cue| {
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
                    if let Some(parent) = model.cue_list.cues.get_mut(&parent_id)
                        && let CueParam::Group { children, .. } = &mut parent.params
                    {
                        let idx = match index {
                            Some(idx) if idx <= children.len() => idx,
                            Some(_) => return Err(anyhow::anyhow!("insert index out of range.")),
                            None => children.len(),
                        };
                        children.splice(idx..idx, insert_ids);
                        model
                            .cue_list
                            .cues
                            .extend(insert_cues.into_iter().map(|mut cue| {
                                cue.parent_id = Some(parent_id);
                                if let CueParam::Group { children, .. } = &mut cue.params {
                                    children.clear();
                                }
                                (cue.id, cue)
                            }));
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!("target id not found."))
                    }
                } else {
                    let idx = match index {
                        Some(idx) if idx <= model.cue_list.root_ids.len() => idx,
                        Some(_) => return Err(anyhow::anyhow!("insert index out of range.")),
                        None => model.cue_list.root_ids.len(),
                    };
                    model.cue_list.root_ids.splice(idx..idx, insert_ids);
                    model
                        .cue_list
                        .cues
                        .extend(insert_cues.into_iter().map(|mut cue| {
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
                project_type: project_file.project_type,
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
                    let Some(parent) = saved_path.parent() else {
                        return Err(anyhow!("Invalid project folder path."));
                    };
                    let mut targets: HashMap<_, _> = {
                        let model = self.read().await;
                        model
                            .cue_list
                            .cues
                            .values()
                            .filter_map(|cue| {
                                if let CueParam::Audio(params) = &cue.params {
                                    Some((cue.id, params.target.clone()))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    };

                    for target in targets.values_mut() {
                        let asset_path = parent.join(&*target);
                        let new_path = import_asset_file(
                            asset_path,
                            project_dir.to_path_buf(),
                            import_destination.clone(),
                        )
                        .await?;
                        *target = new_path;
                    }

                    let mut model = self.model.write().await;
                    for (id, target) in targets {
                        if let Some(cue) = model.cue_list.cues.get_mut(&id)
                            && let CueParam::Audio(params) = &mut cue.params
                        {
                            params.target = target;
                        }
                    }
                } else {
                    let mut targets: HashMap<_, _> = {
                        let model = self.read().await;
                        model
                            .cue_list
                            .cues
                            .values()
                            .filter_map(|cue| {
                                if let CueParam::Audio(params) = &cue.params
                                    && params.target.is_absolute()
                                {
                                    Some((cue.id, params.target.clone()))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    };

                    for target in targets.values_mut() {
                        let new_path = import_asset_file(
                            target.clone(),
                            project_dir.to_path_buf(),
                            import_destination.clone(),
                        )
                        .await?;
                        *target = new_path;
                    }

                    let mut model = self.model.write().await;
                    for (id, target) in targets {
                        if let Some(cue) = model.cue_list.cues.get_mut(&id)
                            && let CueParam::Audio(params) = &mut cue.params
                        {
                            params.target = target;
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
                project_type: *project_type,
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

    #[cfg(test)]
    pub async fn set_project_status(&self, new_project_status: ProjectStatus) {
        let mut project_status = self.project_status.write().await;
        *project_status = new_project_status;
    }
}

async fn import_asset_file(
    asset_path: PathBuf,
    model_dir: PathBuf,
    import_destination: String,
) -> anyhow::Result<PathBuf> {
    tokio::task::spawn_blocking(move || {
        log::info!("Import asset file started. file={:?}", asset_path);
        let audio_dir = model_dir.join(import_destination);
        if !audio_dir.exists() {
            std::fs::create_dir_all(&audio_dir)?;
        } else if audio_dir.is_file() {
            anyhow::bail!("Failed to copy asset to destination. destination is not directory");
        }
        let asset_file_name = asset_path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid asset path. path={:?}", asset_path))?;
        let dest_path = audio_dir.join(asset_file_name);
        let copied_path = resolve_dest_path(&asset_path, &dest_path)?;
        Ok(copied_path.strip_prefix(audio_dir)?.to_path_buf())
    })
    .await?
}

fn resolve_dest_path(src: &Path, dest: &Path) -> std::io::Result<PathBuf> {
    match create_new_copy(src, dest) {
        Ok(()) => return Ok(dest.to_path_buf()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }

    let src_hash = hash_file(src)?;
    if src_hash == hash_file(dest)? {
        return Ok(dest.to_path_buf());
    }

    let hashed_path = filename_with_suffix(dest, &src_hash[..8]);

    match create_new_copy(src, &hashed_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }

    Ok(hashed_path)
}

fn create_new_copy(src: &Path, dest: &Path) -> std::io::Result<()> {
    let mut dest_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dest)?;
    let mut src_file = std::fs::File::open(src)?;
    std::io::copy(&mut src_file, &mut dest_file)?;
    Ok(())
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut hasher = sha2::Sha256::new();
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>())
}

fn filename_with_suffix(path: &Path, digest: &str) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().map(|e| e.to_string_lossy().to_string());
    let new_name = match ext {
        Some(ext) => format!("{}_{}.{}", stem, digest, ext),
        None => format!("{}_{}", stem, digest),
    };
    path.with_file_name(new_name)
}
