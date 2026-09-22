// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use serde::{Serialize, de::DeserializeOwned};
use std::io::Write as _;
use std::path::{Path, PathBuf};
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
            let Some(parent) = dest_path.parent() else {
                anyhow::bail!("Invalid path to save");
            };
            std::fs::create_dir_all(parent)?;

            let content = serde_json::to_string_pretty(&settings)?;
            #[cfg(unix)]
            let mut temp_file = {
                use std::os::unix::fs::PermissionsExt;
                let permissions = match std::fs::metadata(&dest_path) {
                    Ok(metadata) => metadata.permissions(),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        std::fs::Permissions::from_mode(0o666)
                    }
                    Err(error) => return Err(error.into()),
                };
                tempfile::Builder::new()
                    .permissions(permissions)
                    .tempfile_in(parent)?
            };
            #[cfg(not(unix))]
            let mut temp_file = tempfile::NamedTempFile::new_in(parent)?;
            {
                let file = temp_file.as_file_mut();
                file.write_all(content.as_bytes())?;
                file.flush()?;
                file.sync_all()?;
            }
            temp_file.persist(&dest_path)?;
            #[cfg(unix)]
            {
                std::fs::File::open(parent)?.sync_all()?;
            }
            Ok(())
        })
        .await??;

        log::info!("GlobalSettings saved to: {}", path.display());
        Ok(())
    }
}
