use std::{
    collections::HashSet, path::{Path, PathBuf}, sync::Arc
};

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc};
use uuid::Uuid;

use crate::{
    event::{UiError, UiEvent},
    model::{ShowModel, cue::Cue, settings::ShowSettings},
};

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "command",
    content = "params",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ModelCommand {
    UpdateCue(Cue),
    AddCue { cue: Cue, at_index: usize },
    AddCues { cues: Vec<Cue>, at_index: usize },
    RemoveCue { cue_id: Uuid },
    MoveCue { cue_id: Uuid, to_index: usize },
    
    RenumberCues {
        cues: Vec<Uuid>,
        start_from: f64,
        increment: f64,
    },

    UpdateSettings(Box<ShowSettings>),

    Save,
    SaveToFile(PathBuf),
    LoadFromFile(PathBuf),
}

pub struct ShowModelManager {
    model: Arc<RwLock<ShowModel>>,
    command_rx: mpsc::Receiver<ModelCommand>,
    event_tx: broadcast::Sender<UiEvent>,

    show_model_path: Arc<RwLock<Option<PathBuf>>>,
}

impl ShowModelManager {
    pub fn new(event_tx: broadcast::Sender<UiEvent>) -> (Self, ShowModelHandle) {
        let (command_tx, command_rx) = mpsc::channel(32);
        let model = Arc::new(RwLock::new(ShowModel::default()));
        let show_model_path = Arc::new(RwLock::new(None));
        let manager = Self {
            model: model.clone(),
            command_rx,
            event_tx,
            show_model_path: show_model_path.clone(),
        };
        let handle = ShowModelHandle {
            model,
            command_tx,
            show_model_path,
        };

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
        match command {
            ModelCommand::UpdateCue(cue) => {
                let mut model = self.model.write().await;
                let event = if let Some(index) = model.cues.iter().position(|c| c.id == cue.id) {
                    model.cues[index] = cue.clone();
                    UiEvent::CueUpdated { cue }
                } else {
                    UiEvent::OperationFailed {
                        error: UiError::CueEdit {
                            message: format!("Cue doesn't exist: cue_id={}", cue.id),
                        },
                    }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::AddCue { cue, at_index } => {
                let mut model = self.model.write().await;
                let event = if model.cues.iter().any(|c| c.id == cue.id) {
                    UiEvent::OperationFailed {
                        error: UiError::CueEdit {
                            message: format!("Cue already exist: cue_id={}", cue.id),
                        },
                    }
                } else if at_index > model.cues.len() {
                    UiEvent::OperationFailed {
                        error: UiError::CueEdit {
                            message: "Insert index is out of list.".to_string(),
                        },
                    }
                } else {
                    model.cues.insert(at_index, cue.clone());
                    UiEvent::CueAdded { cue, at_index }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::AddCues { cues, at_index } => {
                let mut model = self.model.write().await;
                let mut added_cues = vec![];
                if at_index > model.cues.len() {
                    self.event_tx.send(UiEvent::OperationFailed {
                        error: UiError::CueEdit {
                            message: "Insert index is out of list.".to_string(),
                        },
                    })?;
                } else {
                    let mut insert_index = 0;
                    for cue in cues.iter() {
                        if model.cues.iter().any(|c| c.id == cue.id) {
                            self.event_tx.send(UiEvent::OperationFailed {
                                error: UiError::CueEdit {
                                    message: format!("Cue already exist. cue_id={}", cue.id),
                                },
                            })?;
                        } else {
                            model.cues.insert(at_index + insert_index, cue.clone());
                            added_cues.push(cue.clone());
                            insert_index += 1;
                        }
                    }
                    self.event_tx.send(UiEvent::CuesAdded { cues: added_cues, at_index })?;
                }
                Ok(())
            }
            ModelCommand::RemoveCue { cue_id } => {
                let mut model = self.model.write().await;
                let event = if let Some(index) = model.cues.iter().position(|c| c.id == cue_id) {
                    model.cues.remove(index);
                    UiEvent::CueRemoved { cue_id }
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
            ModelCommand::MoveCue { cue_id, to_index } => {
                let mut model = self.model.write().await;
                let event = if let Some(index) = model.cues.iter().position(|c| c.id == cue_id) {
                    let cue = model.cues.remove(index);
                    model.cues.insert(to_index, cue.clone());
                    UiEvent::CueMoved { cue_id, to_index }
                } else if to_index > model.cues.len() {
                    UiEvent::OperationFailed {
                        error: UiError::CueEdit {
                            message: "Insert index is out of list.".to_string(),
                        },
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
            ModelCommand::RenumberCues { cues, start_from, increment } => {
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
                    self.event_tx.send(UiEvent::CueListUpdated { cues: model.cues.clone() })?;
                }
                Ok(())
            }
            ModelCommand::UpdateSettings(new_settings) => {
                let mut model = self.model.write().await;
                // TODO setting validation
                model.settings = *new_settings.clone();
                self.event_tx.send(UiEvent::SettingsUpdated { new_settings })?;
                Ok(())
            }
            ModelCommand::Save => {
                let event = if let Some(path) = self.show_model_path.read().await.as_ref() {
                    if let Err(error) = self.save_to_file(path.as_path()).await {
                        log::error!("Failed to save model file: {}", error);
                        UiEvent::OperationFailed {
                            error: UiError::FileSave {
                                path: path.to_path_buf(),
                                message: error.to_string(),
                            },
                        }
                    } else {
                        UiEvent::ShowModelSaved {
                            path: path.to_path_buf(),
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
                let event = if let Err(error) = self.save_to_file(path.as_path()).await {
                    log::error!("Failed to save model file: {}", error);
                    UiEvent::OperationFailed {
                        error: UiError::FileSave {
                            path,
                            message: error.to_string(),
                        },
                    }
                } else {
                    let mut show_model_path = self.show_model_path.write().await;
                    *show_model_path = Some(path.clone());
                    UiEvent::ShowModelSaved { path }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::LoadFromFile(path) => {
                let event = if let Err(error) = self.load_from_file(path.as_path()).await {
                    log::error!("Failed to load model file: {}", error);
                    UiEvent::OperationFailed {
                        error: UiError::FileLoad {
                            path,
                            message: error.to_string(),
                        },
                    }
                } else {
                    let mut show_model_path = self.show_model_path.write().await;
                    *show_model_path = Some(path.clone());
                    UiEvent::ShowModelLoaded { path }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
        }
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, ShowModel> {
        self.model.read().await
    }

    pub async fn write_with<F, R>(&self, updater: F) -> R
    where
        F: FnOnce(&mut ShowModel) -> R,
    {
        let mut guard = self.model.write().await;
        updater(&mut guard)
    }

    pub async fn load_from_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let content = tokio::fs::read_to_string(path).await?;

        let new_model: ShowModel =
            tokio::task::spawn_blocking(move || serde_json::from_str(&content)).await??;

        self.write_with(|state| {
            *state = new_model;
        })
        .await;

        log::info!("Show loaded from: {}", path.display());
        Ok(())
    }

    pub async fn save_to_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let state_guard = self.read().await;

        let model_clone = state_guard.clone();
        drop(state_guard); // Readロックを明示的に解放

        let content =
            tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&model_clone))
                .await??;

        tokio::fs::write(path, content).await?;
        log::info!("Show saved to: {}", path.display());
        Ok(())
    }
}

#[derive(Clone)]
pub struct ShowModelHandle {
    model: Arc<RwLock<ShowModel>>,
    command_tx: mpsc::Sender<ModelCommand>,
    show_model_path: Arc<RwLock<Option<PathBuf>>>,
}

impl ShowModelHandle {
    pub async fn send_command(&self, command: ModelCommand) -> anyhow::Result<()> {
        self.command_tx.send(command).await?;
        Ok(())
    }

    pub async fn update_cue(&self, cue: Cue) -> anyhow::Result<()> {
        self.send_command(ModelCommand::UpdateCue(cue)).await?;
        Ok(())
    }

    pub async fn add_cue(&self, cue: Cue, at_index: usize) -> anyhow::Result<()> {
        self.send_command(ModelCommand::AddCue { cue, at_index })
            .await?;
        Ok(())
    }

    pub async fn add_cues(&self, cues: Vec<Cue>, at_index: usize) -> anyhow::Result<()> {
        self.send_command(ModelCommand::AddCues { cues, at_index }).await?;
        Ok(())
    }

    pub async fn remove_cue(&self, cue_id: Uuid) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RemoveCue { cue_id })
            .await?;
        Ok(())
    }

    pub async fn move_cue(&self, cue_id: Uuid, to_index: usize) -> anyhow::Result<()> {
        self.send_command(ModelCommand::MoveCue { cue_id, to_index })
            .await?;
        Ok(())
    }

    pub async fn renumber_cues(&self, cues: Vec<Uuid>, start_from: f64, increment: f64) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RenumberCues { cues, start_from, increment })
            .await?;
        Ok(())
    }

    pub async fn update_settings(&self, new_settings: ShowSettings) -> anyhow::Result<()> {
        self.send_command(ModelCommand::UpdateSettings(Box::new(new_settings)))
            .await?;
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

    pub async fn load_from_file(&self, path: PathBuf) -> anyhow::Result<()> {
        self.send_command(ModelCommand::LoadFromFile(path)).await?;
        Ok(())
    }

    pub async fn get_cue_by_id(&self, cue_id: &Uuid) -> Option<Cue> {
        self.read()
            .await
            .cues
            .iter()
            .find(|c| c.id.eq(cue_id))
            .cloned()
    }

    pub async fn get_current_file_path(&self) -> Option<PathBuf> {
        self.show_model_path.read().await.clone()
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, ShowModel> {
        self.model.read().await
    }
}
