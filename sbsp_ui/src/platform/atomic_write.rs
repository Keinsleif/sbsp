// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Writing a file so that it is never seen half written.
//!
//! The content goes to a temporary file in the same folder, which then
//! replaces the destination. An existing file keeps its permissions (a
//! `chmod 600` settings file stays private); a new one gets the usual
//! defaults. The data and the folder are synced, so a power cut leaves the
//! old file or the new one, not an empty one.

use std::io::Write as _;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

/// Writes `content` to `dest`, creating its folder when needed. Blocking.
pub fn write_atomic(dest: &Path, content: &[u8]) -> anyhow::Result<()> {
    let Some(parent) = dest.parent() else {
        anyhow::bail!("Invalid path to save");
    };
    std::fs::create_dir_all(parent)?;

    #[cfg(unix)]
    let permissions = match std::fs::metadata(dest) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    // The temporary file starts with the permissions the destination has,
    // so there is no moment when the content is more readable than it was.
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
        file.write_all(content)?;
        file.flush()?;
        #[cfg(unix)]
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)?;
        }
        file.sync_all()?;
    }
    temp_file.persist(dest)?;
    #[cfg(unix)]
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_created_with_its_folder_and_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a").join("b.json");

        write_atomic(&path, b"first").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
    }

    #[test]
    fn no_temporary_file_is_left_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.json");

        write_atomic(&path, b"x").unwrap();
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["b.json"]);
    }

    #[cfg(unix)]
    #[test]
    fn an_existing_file_keeps_its_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private.json");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        write_atomic(&path, b"new").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
