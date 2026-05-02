use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

use super::GlobalRemoteSettings;

pub struct GlobalSettingsManager {
    path: PathBuf,
    settings: RwLock<GlobalRemoteSettings>,
}

impl GlobalSettingsManager {
    pub fn new(path: PathBuf) -> Self {
        let settings = GlobalRemoteSettings::default();
        Self {
            path,
            settings: RwLock::new(settings),
        }
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, GlobalRemoteSettings> {
        self.settings.read().await
    }

    pub async fn update(&self, new_settings: &GlobalRemoteSettings) {
        let mut settings = self.settings.write().await;
        *settings = new_settings.clone();
    }

    pub async fn load(&self) -> Result<GlobalRemoteSettings, anyhow::Error> {
        let content = tokio::fs::read_to_string(self.path.clone()).await?;

        let new_settings = tokio::task::spawn_blocking(move || {
            serde_json::from_str::<GlobalRemoteSettings>(&content)
        })
        .await??;

        self.update(&new_settings).await;

        log::info!("GlobalSettings loaded from: {}", self.path.display());
        Ok(new_settings)
    }

    pub async fn save(&self) -> Result<(), anyhow::Error> {
        let settings = self.settings.read().await.clone();

        let content =
            tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&settings)).await??;

        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(self.path.clone(), content).await?;
        log::info!("GlobalSettings saved to: {}", self.path.display());
        Ok(())
    }

    pub async fn import_from_file(&self, path: &Path) -> Result<GlobalRemoteSettings, anyhow::Error> {
        let content = tokio::fs::read_to_string(path).await?;

        let new_settings = tokio::task::spawn_blocking(move || {
            serde_json::from_str::<GlobalRemoteSettings>(&content)
        })
        .await??;

        self.update(&new_settings).await;

        self.save().await?;

        log::info!("GlobalSettings imported from: {}", path.display());
        Ok(new_settings)
    }

    pub async fn export_to_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let settings = self.settings.read().await.clone();

        let content =
            tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&settings)).await??;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, content).await?;
        log::info!("GlobalSettings saved to: {}", path.display());
        Ok(())
    }
}
