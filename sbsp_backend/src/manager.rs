mod handle;

pub use handle::ShowModelHandle;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc};
use uuid::Uuid;

use crate::{
    event::{UiError, UiEvent},
    model::{cue::{Cue, CueParam}, settings::ShowSettings, ShowModel},
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
    AddCue {
        cue: Cue,
        at_index: usize,
    },
    AddCues {
        cues: Vec<Cue>,
        at_index: usize,
    },
    RemoveCue {
        cue_id: Uuid,
    },
    MoveCue {
        cue_id: Uuid,
        to_index: usize,
    },

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
                let model_path_option = self.show_model_path.read().await.clone();
                let event = if let Some(index) = model.cues.iter().position(|c| c.id == cue.id) {
                    let mut new_cue = cue.clone();
                    if let CueParam::Audio(audio_param) = &mut new_cue.params
                    && let Some(model_path) = &model_path_option && model.settings.general.copy_assets_when_add {
                        let new_target = self.import_asset_file(&audio_param.target, model_path).await;
                        if let Ok(target) = new_target {
                            audio_param.target = target;
                        } // ignore failed to import asset. use absolute path
                    }
                    model.cues[index] = new_cue.clone();
                    UiEvent::CueUpdated { cue: new_cue }
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
                let model_path_option = self.show_model_path.read().await.clone();
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
                    let mut new_cue = cue.clone();
                    if let CueParam::Audio(audio_param) = &mut new_cue.params
                    && let Some(model_path) = &model_path_option && model.settings.general.copy_assets_when_add {
                        let new_target = self.import_asset_file(&audio_param.target, model_path).await;
                        if let Ok(target) = new_target {
                            audio_param.target = target;
                        } // ignore failed to import asset. use absolute path
                    }
                    model.cues.insert(at_index, new_cue.clone());
                    UiEvent::CueAdded { cue: new_cue, at_index }
                };
                self.event_tx.send(event)?;
                Ok(())
            }
            ModelCommand::AddCues { cues, at_index } => {
                let mut model = self.model.write().await;
                let model_path_option = self.show_model_path.read().await.clone();
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
                            let mut new_cue = cue.clone();
                            if let CueParam::Audio(audio_param) = &mut new_cue.params
                            && let Some(model_path) = &model_path_option && model.settings.general.copy_assets_when_add {
                                let new_target = self.import_asset_file(&audio_param.target, model_path).await;
                                if let Ok(target) = new_target {
                                    audio_param.target = target;
                                } // ignore failed to import asset. use absolute path
                            }
                            model.cues.insert(at_index + insert_index, new_cue.clone());
                            added_cues.push(new_cue.clone());
                            insert_index += 1;
                        }
                    }
                    self.event_tx.send(UiEvent::CuesAdded {
                        cues: added_cues,
                        at_index,
                    })?;
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
                    self.event_tx.send(UiEvent::CueListUpdated {
                        cues: model.cues.clone(),
                    })?;
                }
                Ok(())
            }
            ModelCommand::UpdateSettings(new_settings) => {
                let mut model = self.model.write().await;
                // TODO setting validation
                model.settings = *new_settings.clone();
                self.event_tx
                    .send(UiEvent::SettingsUpdated { new_settings })?;
                Ok(())
            }
            ModelCommand::Save => {
                let event = if let Some(path) = self.show_model_path.read().await.as_ref() {
                    if let Err(error) = self.save_to_file(path).await {
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
                let event = if let Err(error) = self.save_to_file(&path).await {
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

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, ShowModel> {
        self.model.write().await
    }

    pub async fn load_from_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let content = tokio::fs::read_to_string(path.join("model.json")).await?;

        let new_model: ShowModel =
            tokio::task::spawn_blocking(move || serde_json::from_str(&content)).await??;

        let mut write_lock = self.write().await;
        *write_lock = new_model;
        drop(write_lock);

        log::info!("Show loaded from: {}", path.display());
        Ok(())
    }

    pub async fn save_to_file(&self, path: &PathBuf) -> Result<(), anyhow::Error> {
        let mut state_guard = self.model.write().await;

        if !path.is_dir() {
            tokio::fs::create_dir_all(&path).await?;
        }

        let model_path_option = self.show_model_path.read().await.clone();
        for cue in &mut state_guard.cues {
            if let CueParam::Audio(audio_param) = &mut cue.params {
                if let Some(model_path) = &model_path_option && model_path != path {
                    // copy exists project to another path
                    let asset_path = model_path.join("audio").join(&audio_param.target);
                    let new_path = self.import_asset_file(&asset_path, path).await?;
                    audio_param.target = new_path;
                } else if audio_param.target.is_absolute() {
                    // update exists project
                    // create new project
                    let new_path = self.import_asset_file(&audio_param.target, path).await?;
                    audio_param.target = new_path;
                }
            }
        }

        let model_clone = state_guard.clone();
        drop(state_guard);

        let content =
            tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&model_clone))
                .await??;

        tokio::fs::write(path.join("model.json"), content).await?;
        log::info!("Show saved to: {}", path.display());
        Ok(())
    }

    async fn import_asset_file(&self, asset_path: &PathBuf, model_path: &Path) -> anyhow::Result<PathBuf> {
        let audio_dir = model_path.join("audio");
        if !audio_dir.exists() {
            tokio::fs::create_dir_all(&audio_dir).await?;
        } else if audio_dir.is_file() {
            anyhow::bail!("Failed to copy asset to library. Library is not directory");
        }
        let dest_path = audio_dir.join(asset_path.file_name().unwrap().to_str().unwrap()).clone();
        tokio::fs::copy(asset_path, &dest_path).await?;
        Ok(dest_path)
    }

    #[cfg(test)]
    pub async fn set_show_model_path(&self, path_option: Option<PathBuf>) {
        let mut write_lock = self.show_model_path.write().await;
        *write_lock = path_option;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::{tempdir, NamedTempFile};
    use tokio::sync::broadcast;
    use uuid::Uuid;
    use crate::{event::UiEvent, model::{cue::{audio::{AudioCueParam, SoundType}, Cue, CueParam, CueSequence}, settings::{GeneralSettings, ShowSettings}, ShowModel}};

    use super::{ShowModelManager, ShowModelHandle};

    async fn setup_manager(initial_model: Option<ShowModel>, model_path: Option<PathBuf>) -> (ShowModelHandle, broadcast::Receiver<UiEvent>) {
        let (event_tx, event_rx) = broadcast::channel::<UiEvent>(32);
        let (model_manager, model_handle) = ShowModelManager::new(event_tx.clone());
        if let Some(inital) = initial_model {
            let mut model_lock = model_manager.write().await;
            *model_lock = inital;
            drop(model_lock);
        }
        model_manager.set_show_model_path(model_path).await;
        tokio::spawn(model_manager.run());
        (model_handle, event_rx)
    }

    #[tokio::test]
    async fn update_cue() {
        let temp_dir = tempdir().unwrap();
        let temp_target = NamedTempFile::with_suffix(".mp3").unwrap();
        let temp_target_after = NamedTempFile::with_suffix(".wav").unwrap();
        let cue_id = Uuid::new_v4();
        let (model_handle, mut event_rx) = setup_manager(Some(ShowModel {
            name: "test".into(),
            cues: vec![
                Cue { id: cue_id, number: "1".into(), name: Some("test cue".into()), notes: "note".into(), pre_wait: 0.0, sequence: CueSequence::DoNotContinue, params: CueParam::Audio(AudioCueParam{ target: temp_target.path().to_path_buf(), start_time: None, fade_in_param: None, end_time: None, fade_out_param: None, volume: 0.0, pan: 0.0, repeat: false, sound_type: SoundType::Streaming }) }
            ],
            settings: ShowSettings { general: GeneralSettings { copy_assets_when_add: true, ..Default::default() }, ..Default::default() },
        }), Some(temp_dir.path().to_path_buf())).await;

        let new_cue = Cue { id: cue_id, number: "1".into(), name: Some("test cue".into()), notes: "note".into(), pre_wait: 0.0, sequence: CueSequence::DoNotContinue, params: CueParam::Audio(AudioCueParam{ target: temp_target_after.path().to_path_buf(), start_time: None, fade_in_param: None, end_time: None, fade_out_param: None, volume: 0.0, pan: 0.0, repeat: false, sound_type: SoundType::Streaming }) };
        model_handle.update_cue(new_cue.clone()).await.unwrap();

        let estimated_audio_target = temp_dir.path().join("audio").join(temp_target_after.path().file_name().unwrap().to_str().unwrap());
        let mut estimated_new_cue = new_cue.clone();
        if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
            audio_param.target = estimated_audio_target.clone();
        }

        let updated_cue;
        loop {
            if let Ok(UiEvent::CueUpdated { cue }) = event_rx.recv().await {
                updated_cue = cue;
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
        let (model_handle, mut event_rx) = setup_manager(Some(ShowModel {
            name: "test".into(),
            cues: vec![],
            settings: ShowSettings { general: GeneralSettings { copy_assets_when_add: true, ..Default::default() }, ..Default::default() },
        }), Some(temp_dir.path().to_path_buf())).await;

        let new_cue = Cue { id: cue_id, number: "1".into(), name: Some("test cue".into()), notes: "note".into(), pre_wait: 0.0, sequence: CueSequence::DoNotContinue, params: CueParam::Audio(AudioCueParam{ target: temp_target.path().to_path_buf(), start_time: None, fade_in_param: None, end_time: None, fade_out_param: None, volume: 0.0, pan: 0.0, repeat: false, sound_type: SoundType::Streaming }) };
        model_handle.add_cue(new_cue.clone(), 0).await.unwrap();

        let estimated_audio_target = temp_dir.path().join("audio").join(temp_target.path().file_name().unwrap().to_str().unwrap());
        let mut estimated_new_cue = new_cue.clone();
        if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
            audio_param.target = estimated_audio_target.clone();
        }

        let added_cue;
        let added_at_index;
        loop {
            if let Ok(UiEvent::CueAdded { cue, at_index }) = event_rx.recv().await {
                added_cue = cue;
                added_at_index = at_index;
                break;
            }
        }
        assert_eq!(added_cue, estimated_new_cue);
        assert_eq!(added_at_index, 0);

        let model = model_handle.read().await;
        assert_eq!(model.cues[0], estimated_new_cue);
        assert!(estimated_audio_target.exists());
        drop(temp_target);
        drop(temp_dir);
    }
}