use std::path::Path;
use tokio::sync::{RwLock, watch};

use super::GlobalHostSettings;
use sbsp_backend::BackendSettings;

pub struct GlobalSettingsManager {
    settings: RwLock<GlobalHostSettings>,
    settings_tx: watch::Sender<BackendSettings>,
}

impl GlobalSettingsManager {
    pub fn new() -> (Self, watch::Receiver<BackendSettings>) {
        let settings = GlobalHostSettings::default();
        let (settings_tx, settings_rx) = watch::channel(BackendSettings::from(&settings));
        (
            Self {
                settings: RwLock::new(settings),
                settings_tx,
            },
            settings_rx,
        )
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, GlobalHostSettings> {
        self.settings.read().await
    }

    pub async fn update(&self, new_settings: GlobalHostSettings) {
        {
            let mut settings = self.settings.write().await;
            *settings = new_settings.clone();
        }
        self.settings_tx.send_modify(|backend_state| {
            *backend_state = BackendSettings::from(&new_settings);
        });
    }

    pub async fn load_from_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let content = tokio::fs::read_to_string(path).await?;

        let new_settings =
            tokio::task::spawn_blocking(move || serde_json::from_str::<GlobalHostSettings>(&content))
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
