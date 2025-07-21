use std::{path::Path, sync::Arc};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::model::{cue::Cue, ShowModel};

#[derive(Clone)]
pub struct ShowModelManager {
    state: Arc<RwLock<ShowModel>>
}

impl ShowModelManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ShowModel::default()))
        }
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, ShowModel> {
        self.state.read().await
    }

    pub async fn write_with<F, R>(&self, updater: F) -> R
    where
        F: FnOnce(&mut ShowModel) -> R,
    {
        let mut guard = self.state.write().await;
        updater(&mut guard)
    }

    pub async fn load_from_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let content = tokio::fs::read_to_string(path).await?;
        
        let new_model: ShowModel = tokio::task::spawn_blocking(move || {
            serde_json::from_str(&content)
        }).await??;

        self.write_with(|state| {
            *state = new_model;
        }).await;

        log::info!("Show loaded from: {}", path.display());
        Ok(())
    }

    pub async fn save_to_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let state_guard = self.read().await;

        let model_clone = state_guard.clone();
        drop(state_guard); // Readロックを明示的に解放

        let content = tokio::task::spawn_blocking(move || {
            serde_json::to_string_pretty(&model_clone)
        }).await??;

        tokio::fs::write(path, content).await?;
        log::info!("Show saved to: {}", path.display());
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
}