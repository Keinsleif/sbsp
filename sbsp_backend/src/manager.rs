use std::{path::{Path, PathBuf}, sync::Arc};

use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::{event::UiEvent, model::{cue::Cue, ShowModel}};

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
                    return Some(UiEvent::CueUpdated { cue });
                }
            }
            ModelCommand::AddCue { cue, at_index } => {

            }
            ModelCommand::RemoveCue { cue_id } => {

            }
            ModelCommand::MoveCue { cue_id, to_index } => {
                
            }
            // ★ Saveコマンドのロジック
            ModelCommand::Save => {
                if let Some(path) = self.show_model_path.read().await.as_ref() {
                    self.save_to_file(path.as_path()).await.unwrap();
                } else {
                    log::warn!("Save command issued, but no file path is set. Use SaveToFile first.");
                }
            }
            // ★ SaveAsコマンドのロジック
            ModelCommand::SaveToFile(path) => {
                self.save_to_file(path.as_path()).await.unwrap();
                let mut show_model_path = self.show_model_path.write().await;
                *show_model_path = Some(path);
            }
            // ★ LoadFromFileコマンドのロジック
            ModelCommand::LoadFromFile(path) => {
                self.load_from_file(path.as_path()).await.unwrap();
                let mut show_model_path = self.show_model_path.write().await;
                *show_model_path = Some(path);
            }
            // ... 他のコマンド処理
        }
        None
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
    pub async fn update_cue(&self, cue: Cue) {
        self.command_tx.send(ModelCommand::UpdateCue(cue)).await.ok();
    }
    
    pub async fn add_cue(&self, cue: Cue, at_index: usize) {
        self.command_tx.send(ModelCommand::AddCue { cue, at_index }).await.ok();
    }

    pub async fn remove_cue(&self, cue_id: Uuid) {
        self.command_tx.send(ModelCommand::RemoveCue { cue_id }).await.ok();
    }

    pub async fn move_cue(&self, cue_id: Uuid, to_index: usize) {
        self.command_tx.send(ModelCommand::MoveCue { cue_id, to_index }).await.ok();
    }

    pub async fn save(&self) {
        self.command_tx.send(ModelCommand::Save).await.ok();
    }

    pub async fn save_as(&self, path: PathBuf) {
        self.command_tx.send(ModelCommand::SaveToFile(path)).await.ok();
    }

    pub async fn load_from_file(&self, path: PathBuf) {
        self.command_tx.send(ModelCommand::LoadFromFile(path)).await.ok();
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
