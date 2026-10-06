// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::{collections::HashMap, io::Write as _, path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

use super::data::AssetData;

const ASSET_CACHE_VERSION: usize = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub last_modified: SystemTime,
    pub data: AssetData,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AssetCache {
    version: usize,
    pub entries: HashMap<PathBuf, CacheEntry>,
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            version: ASSET_CACHE_VERSION,
            entries: HashMap::new(),
        }
    }

    pub async fn load(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let data  = tokio::task::spawn_blocking(move || -> anyhow::Result<AssetCache> {
            let buf = std::fs::read(path)?;
            let data = rmp_serde::from_slice(&buf)?;
            Ok(data)
        }).await??;
        if data.version == ASSET_CACHE_VERSION {
            self.entries = data.entries
        }
        Ok(())
    }

    pub async fn save(&self, path: PathBuf) -> anyhow::Result<()> {
        let data = self.clone();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let Some(parent) = path.parent() else {
                anyhow::bail!("Invalid path to save");
            };
            let content = rmp_serde::to_vec_named(&data)?;
            #[cfg(unix)]
            let permissions = {
                match std::fs::metadata(&path) {
                    Ok(metadata) => Some(metadata.permissions()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(error.into()),
                }
            };
            #[cfg(unix)]
            let mut temp_file = tempfile::Builder::new()
                .permissions(
                    permissions
                        .clone()
                        .unwrap_or_else(|| std::fs::Permissions::from_mode(0o666)),
                )
                .tempfile_in(parent)?;
            #[cfg(not(unix))]
            let mut temp_file = tempfile::NamedTempFile::new_in(parent)?;

            {
                let file = temp_file.as_file_mut();
                file.write_all(&content)?;
                file.flush()?;
                #[cfg(unix)]
                if let Some(perm) = permissions {
                    file.set_permissions(perm)?;
                }
                file.sync_all()?;
            }
            temp_file.persist(&path)?;
            #[cfg(unix)]
            {
                std::fs::File::open(parent)?.sync_all()?;
            }
            Ok(())
        })
        .await??;
        Ok(())
    }
}