// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use serde::{Serialize, de::DeserializeOwned};
use tokio::{fs, sync::watch, task};

use crate::platform::atomic_write;

/// The application identifier, which names the host's directory under the
/// OS config directory. It is also the identifier of the installed
/// package, so it must not change: settings saved under the old name would
/// no longer be found.
#[cfg(feature = "host")]
const IDENTIFIER: &str = "com.keinsleif.sbsp";
#[cfg(feature = "remote")]
const IDENTIFIER: &str = "com.keinsleif.sbsp.remote";

/// `dirs::config_dir()/<identifier>/config.json`. `None` if the OS config
/// directory itself can't be resolved (should not normally happen).
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(IDENTIFIER).join("config.json"))
}

pub struct NoSubscribe;

impl<T> From<&T> for NoSubscribe {
    fn from(_: &T) -> Self {
        Self
    }
}

pub struct SettingsManager<T, B = NoSubscribe> {
    path: Option<PathBuf>,
    settings: T,
    settings_tx: Option<watch::Sender<B>>,
}

impl<T, B> SettingsManager<T, B>
where
    T: Serialize + DeserializeOwned + Clone + Default + Send + Sync + 'static,
    for<'a> B: From<&'a T> + Send + Sync + 'static,
{
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            settings: T::default(),
            settings_tx: None,
        }
    }

    pub fn subscribe(&mut self) -> watch::Receiver<B> {
        if let Some(tx) = &self.settings_tx {
            tx.subscribe()
        } else {
            let (tx, rx) = watch::channel(B::from(&self.settings));
            self.settings_tx = Some(tx);
            rx
        }
    }

    pub fn apply(&mut self, new_settings: T) {
        self.settings = new_settings;
        if let Some(tx) = &self.settings_tx {
            tx.send_replace(B::from(&self.settings));
        }
    }

    pub fn load_task(&self) -> impl Future<Output = anyhow::Result<T>> + Send + 'static {
        let path_opt = self.path.clone();
        async move {
            if let Some(path) = path_opt {
                let content = fs::read_to_string(path.clone()).await?;

                let new_settings =
                    task::spawn_blocking(move || serde_json::from_str::<T>(&content)).await??;

                log::info!("GlobalSettings loaded from: {}", path.display());
                Ok(new_settings)
            } else {
                Err(anyhow::anyhow!(
                    "Settings path unavailable. Settings only exist in memory."
                ))
            }
        }
    }

    pub fn save_task(&self) -> impl Future<Output = anyhow::Result<()>> + Send + 'static {
        let settings = self.settings.clone();
        let path_opt = self.path.clone();
        let tx_opt = self.settings_tx.clone();
        async move {
            if let Some(tx) = tx_opt {
                tx.send_replace(B::from(&settings));
            }

            if let Some(path) = path_opt {
                Self::export(path, settings).await
            } else {
                Err(anyhow::anyhow!(
                    "Settings file unavailable. Settings only exist in memory."
                ))
            }
        }
    }

    pub async fn import_task(path: PathBuf) -> anyhow::Result<T> {
        let content = fs::read_to_string(path).await?;
        let settings = task::spawn_blocking(move || serde_json::from_str::<T>(&content)).await??;
        Ok(settings)
    }

    pub async fn export(path: PathBuf, settings: T) -> anyhow::Result<()> {
        task::spawn_blocking(move || -> anyhow::Result<()> {
            let content = serde_json::to_string_pretty(&settings)?;
            atomic_write::write_atomic(&path, content.as_bytes())?;
            log::info!("GlobalSettings saved to: {}", path.display());
            Ok(())
        })
        .await?
    }
}

impl<T, B> Deref for SettingsManager<T, B> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.settings
    }
}

impl<T, B> DerefMut for SettingsManager<T, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.settings
    }
}
