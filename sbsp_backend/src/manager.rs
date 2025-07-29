use std::{path::{Path, PathBuf}, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::{event::{UiError, UiEvent}, model::{cue::Cue, ShowModel}};

#[derive(Serialize, Deserialize)]
#[serde(tag = "command", content = "params", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ModelCommand {
    UpdateCue(Cue),
    AddCue {
        cue: Cue,
        at_index: usize,
    },
    RemoveCue {
        cue_id: Uuid,
    },
    MoveCue {
        cue_id: Uuid,
        to_index: usize,
    },

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
            let event = self.process_command(command).await;
            if let Some(event) = event {
                self.event_tx.send(event).ok();
            }
        }
    }

    async fn process_command(&self, command: ModelCommand) -> Option<UiEvent> {
        match command {
            ModelCommand::UpdateCue(cue) => {
                let mut model = self.model.write().await;
                if let Some(index) = model.cues.iter().position(|c| c.id == cue.id) {
                    model.cues[index] = cue.clone();
                    Some(UiEvent::CueUpdated { cue })
                } else {
                    Some(UiEvent::OperationFailed { error: UiError::CueEdit { cue_id: cue.id, message: "Cue doesn't exist.".to_string() } })
                }
            }
            ModelCommand::AddCue { cue, at_index } => {
                let mut model = self.model.write().await;
                if model.cues.iter().any(|c| c.id == cue.id) {
                    Some(UiEvent::OperationFailed { error: UiError::CueEdit { cue_id: cue.id, message: "Cue already exist.".to_string() } })
                } else if at_index > model.cues.len() {
                    Some(UiEvent::OperationFailed { error: UiError::CueEdit { cue_id: cue.id, message: "Insert index is out of list.".to_string() } })
                } else {
                    model.cues.insert(at_index, cue.clone());
                    Some(UiEvent::CueAdded { cue, at_index })
                }
            }
            ModelCommand::RemoveCue { cue_id } => {
                let mut model = self.model.write().await;
                if let Some(index) = model.cues.iter().position(|c| c.id == cue_id) {
                    model.cues.remove(index);
                    Some(UiEvent::CueRemoved { cue_id })
                } else {
                    Some(UiEvent::OperationFailed { error: UiError::CueEdit { cue_id, message: "Cue doesn't exist.".to_string() } })
                }
            }
            ModelCommand::MoveCue { cue_id, to_index } => {
                let mut model = self.model.write().await;
                if let Some(index) = model.cues.iter().position(|c| c.id == cue_id) {
                    let cue = model.cues.remove(index);
                    model.cues.insert(to_index, cue.clone());
                    Some(UiEvent::CueMoved { cue_id, to_index })
                } else if to_index > model.cues.len() {
                    Some(UiEvent::OperationFailed { error: UiError::CueEdit { cue_id, message: "Insert index is out of list.".to_string() } })
                } else {
                    Some(UiEvent::OperationFailed { error: UiError::CueEdit { cue_id, message: "Cue doesn't exist.".to_string() } })
                }
            }
            ModelCommand::Save => {
                if let Some(path) = self.show_model_path.read().await.as_ref() {
                    if let Err(error) = self.save_to_file(path.as_path()).await {
                        log::error!("Failed to save model file: {}", error);
                        Some(UiEvent::OperationFailed { error: UiError::FileSave { path: path.to_path_buf(), message: error.to_string() } })
                    } else {
                        Some(UiEvent::ShowModelSaved { path: path.to_path_buf() })
                    }
                } else {
                    log::warn!("Save command issued, but no file path is set. Use SaveToFile first.");
                    Some(UiEvent::OperationFailed { error: UiError::FileSave { path: PathBuf::new(), message: "Save command issued, but no file path is set. Use SaveToFile first.".to_string() } })
                }
            }
            ModelCommand::SaveToFile(path) => {
                if let Err(error) = self.save_to_file(path.as_path()).await {
                    log::error!("Failed to save model file: {}", error);
                    Some(UiEvent::OperationFailed {error: UiError::FileSave { path, message: error.to_string() }})
                } else {
                    let mut show_model_path = self.show_model_path.write().await;
                    *show_model_path = Some(path.clone());
                    Some(UiEvent::ShowModelSaved { path })
                }
            }
            ModelCommand::LoadFromFile(path) => {
                if let Err(error) = self.load_from_file(path.as_path()).await {
                    log::error!("Failed to load model file: {}", error);
                    Some(UiEvent::OperationFailed {error: UiError::FileLoad { path, message: error.to_string() }})
                } else {
                    let mut show_model_path = self.show_model_path.write().await;
                    *show_model_path = Some(path.clone());
                    Some(UiEvent::ShowModelLoaded { path })
                }
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
        self.send_command(ModelCommand::AddCue { cue, at_index }).await?;
        Ok(())
    }

    pub async fn remove_cue(&self, cue_id: Uuid) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RemoveCue { cue_id }).await?;
        Ok(())
    }

    pub async fn move_cue(&self, cue_id: Uuid, to_index: usize) -> anyhow::Result<()> {
        self.send_command(ModelCommand::MoveCue { cue_id, to_index }).await?;
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
