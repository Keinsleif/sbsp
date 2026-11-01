// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::{fs, task};

pub struct SettingsManager<T> {
    path: Option<PathBuf>,
    settings: RwLock<T>,
}

impl<T> SettingsManager<T>
where
    T: Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static,
{
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            settings: RwLock::new(T::default()),
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.settings.read().await
    }

    pub async fn set(&self, new_settings: T) {
        *self.settings.write().await = new_settings;
    }

    pub async fn load(&self) -> Result<T, anyhow::Error> {
        if let Some(path) = &self.path {
            let content = fs::read_to_string(path.clone()).await?;

            let new_settings =
                task::spawn_blocking(move || serde_json::from_str::<T>(&content)).await??;

            self.set(new_settings.clone()).await;

            log::info!("GlobalSettings loaded from: {}", path.display());
            Ok(new_settings)
        } else {
            Err(anyhow::anyhow!(
                "Settings file unavailable. Settings only exist in memory."
            ))
        }
    }

    pub async fn save(&self) -> Result<(), anyhow::Error> {
        if let Some(path) = &self.path {
            let settings = self.settings.read().await.clone();

            Self::write_settings_to_file(path, settings).await
        } else {
            Err(anyhow::anyhow!(
                "Settings file unavailable. Settings only exist in memory."
            ))
        }
    }

    pub async fn import_from_file(&self, path: &Path) -> Result<T, anyhow::Error> {
        let content = fs::read_to_string(path).await?;
        let settings = task::spawn_blocking(move || serde_json::from_str::<T>(&content)).await??;
        self.set(settings.clone()).await;
        self.save().await?;

        log::info!("GlobalSettings imported from: {}", path.display());
        Ok(settings)
    }

    pub async fn export_to_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        let settings = self.settings.read().await.clone();
        Self::write_settings_to_file(path, settings).await
    }

    pub async fn write_settings_to_file(path: &Path, settings: T) -> anyhow::Result<()> {
        let dest_path = path.to_path_buf();
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let content = serde_json::to_string_pretty(&settings)?;
            crate::atomic_write::write_atomic(&dest_path, content.as_bytes())
        })
        .await??;

        log::info!("GlobalSettings saved to: {}", path.display());
        Ok(())
    }
}
