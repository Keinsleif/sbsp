// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::path::{Path, PathBuf};

use sbsp_backend::BackendSettings;
use sbsp_frontend_settings::{GlobalHostSettings, manager::SettingsManager};
use tokio::sync::{RwLockReadGuard, watch};

pub struct GlobalSettingsManager {
    inner: SettingsManager<GlobalHostSettings>,
    settings_tx: watch::Sender<BackendSettings>,
}

impl GlobalSettingsManager {
    pub fn new(path: Option<PathBuf>) -> (Self, watch::Receiver<BackendSettings>) {
        let inner = SettingsManager::new(path);
        let (settings_tx, settings_rx) =
            watch::channel(BackendSettings::from(&GlobalHostSettings::default()));
        (Self { inner, settings_tx }, settings_rx)
    }

    pub async fn set(&self, new_settings: GlobalHostSettings) {
        self.settings_tx
            .send_modify(|b| *b = BackendSettings::from(&new_settings));
        self.inner.set(new_settings).await;
    }

    pub async fn import_from_file(&self, path: &Path) -> Result<GlobalHostSettings, anyhow::Error> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut settings = tokio::task::spawn_blocking(move || {
            serde_json::from_str::<GlobalHostSettings>(&content)
        })
        .await??;
        sanitize_audio(&mut settings);
        self.set(settings.clone()).await;
        self.save().await?;

        log::info!("GlobalSettings imported from: {}", path.display());
        Ok(settings)
    }

    pub async fn export_to_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let mut settings = self.inner.read().await.clone();
        sanitize_audio(&mut settings);
        let content =
            tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&settings)).await??;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, content).await?;

        log::info!("GlobalSettings saved to: {}", path.display());
        Ok(())
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, GlobalHostSettings> {
        self.inner.read().await
    }
    pub async fn load(&self) -> Result<GlobalHostSettings, anyhow::Error> {
        let new_settings = self.inner.load().await?;
        self.settings_tx.send_modify(|backend_state| {
            *backend_state = BackendSettings::from(&new_settings);
        });
        Ok(new_settings)
    }
    pub async fn save(&self) -> Result<(), anyhow::Error> {
        self.inner.save().await
    }
}

fn sanitize_audio(settings: &mut GlobalHostSettings) {
    settings.audio.device_id = None;
    settings.audio.channel_count = None;
    settings.audio.sample_rate = None;
    settings.audio.buffer_size = None;
}
