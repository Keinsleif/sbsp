use std::path::Path;
use tokio::sync::RwLock;

use super::GlobalSettings;

#[derive(Default)]
pub struct GlobalSettingsManager {
    settings: RwLock<GlobalSettings>,
}

impl GlobalSettingsManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, GlobalSettings> {
        self.settings.read().await
    }

    pub async fn update(&self, new_settings: GlobalSettings) {
        let mut settings = self.settings.write().await;
        *settings = new_settings.clone();
    }

    pub async fn load_from_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let content = tokio::fs::read_to_string(path).await?;

        let new_settings =
            tokio::task::spawn_blocking(move || serde_json::from_str::<GlobalSettings>(&content))
                .await??;

        self.update(new_settings).await;

        log::info!("GlobalSettings loaded from: {}", path.display());
        Ok(())
    }

    pub async fn save_to_file(&self, path: &Path) -> Result<(), anyhow::Error> {
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
