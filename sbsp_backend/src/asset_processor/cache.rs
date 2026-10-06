// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::{
    collections::HashMap,
    io::Write as _,
    path::{Path, PathBuf},
    time::SystemTime,
};

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

    pub async fn load(&mut self, model_path: &Path) -> anyhow::Result<()> {
        let Some(path) = get_cache_path(model_path) else {
            anyhow::bail!("Invalid or missing file name in path")
        };

        let data = tokio::task::spawn_blocking(move || -> anyhow::Result<AssetCache> {
            let buf = std::fs::read(path)?;
            let data = rmp_serde::from_slice(&buf)?;
            Ok(data)
        })
        .await??;
        if data.version == ASSET_CACHE_VERSION {
            self.entries = data.entries
        }
        Ok(())
    }

    pub async fn save(&self, model_path: &Path) -> anyhow::Result<()> {
        let Some(path) = get_cache_path(model_path) else {
            anyhow::bail!("Invalid or missing file name in path")
        };
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

            #[cfg(windows)]
            if let Err(e) = set_hidden(temp_file.path()) {
                log::warn!("Failed to hide cache file: {e}");
            }

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

#[cfg(windows)]
fn set_hidden(path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, GetFileAttributesW, INVALID_FILE_ATTRIBUTES, SetFileAttributesW,
    };

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let attrs = GetFileAttributesW(wide.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(std::io::Error::last_os_error());
        }
        if SetFileAttributesW(wide.as_ptr(), attrs | FILE_ATTRIBUTE_HIDDEN) == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

fn get_cache_path(model_path: &Path) -> Option<PathBuf> {
    let path = model_path.with_extension("cache");
    let file_name = path.file_name()?.to_str()?;

    if file_name.starts_with('.') {
        return Some(path);
    }

    let new_file_name = format!(".{}", file_name);

    Some(path.with_file_name(new_file_name))
}
