// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A minimal `log` + `fern` logger, shared by the host and remote binaries.
//!
//! `log::info!`/`log::error!`/etc. are no-ops until some logger
//! implementation calls `log::set_logger` -- this is that implementation.
//! It writes to stderr and to a log file in the OS log folder (the migration
//! plan, section 5.2): `~/Library/Logs/<identifier>` on macOS,
//! `dirs::data_local_dir()/<identifier>/logs` elsewhere. The file is
//! appended to across runs; once it passes `MAX_FILE_BYTES` at startup it
//! is moved to `<name>.log.old` (replacing the previous one), so the two
//! files together stay bounded.
//!
//! Not used by the web target: stdio isn't connected to the browser console
//! on wasm32, so Phase 9 will need a separate `console_log`-based setup
//! there instead of this module.

use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// A log file larger than this is rotated when the app starts.
const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;

/// The folder the log file lives in, once `init` has opened it.
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// The log folder (for "Show Log Files"). `None` when the file could not
/// be opened, so only stderr is being written.
pub fn dir() -> Option<PathBuf> {
    LOG_DIR.get().cloned()
}

/// The log folder for an app identifier.
fn dir_for(identifier: &str) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map(|home| home.join("Library").join("Logs").join(identifier))
    } else {
        dirs::data_local_dir().map(|dir| dir.join(identifier).join("logs"))
    }
}

/// Opens `<dir>/<name>.log` for appending, rotating it first when large.
fn open_file(dir: &Path, name: &str) -> std::io::Result<File> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{name}.log"));
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_FILE_BYTES) {
        fs::rename(&path, dir.join(format!("{name}.log.old")))?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// Installs the logger. Should be called once, at the very top of `main()`,
/// before anything that might log (in particular, before starting the
/// backend). `identifier` picks the log folder and `name` the file in it.
pub fn init(identifier: &str, name: &str) {
    let level = log::LevelFilter::Info;

    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stderr());

    // A missing log file is not worth refusing to start for: stderr still
    // works, and "Show Log Files" is disabled.
    let mut file_error = None;
    match dir_for(identifier) {
        Some(dir) => match open_file(&dir, name) {
            Ok(file) => {
                dispatch = dispatch.chain(file);
                let _ = LOG_DIR.set(dir);
            }
            Err(e) => file_error = Some(format!("{}: {e}", dir.display())),
        },
        None => file_error = Some("no log folder on this system".to_string()),
    }

    if let Err(e) = dispatch.apply() {
        // Only fails if a logger was already installed; fall back to
        // stderr directly so this isn't silently swallowed.
        eprintln!("failed to initialize logger: {e}");
        return;
    }
    if let Some(error) = file_error {
        log::warn!("log file unavailable: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sbsp_logging_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_folder_is_created_and_the_file_is_appended_to() {
        use std::io::Write as _;
        let dir = temp_dir("append");
        writeln!(open_file(&dir, "app").unwrap(), "one").unwrap();
        writeln!(open_file(&dir, "app").unwrap(), "two").unwrap();
        assert_eq!(fs::read_to_string(dir.join("app.log")).unwrap(), "one\ntwo\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_large_file_is_moved_aside_on_startup() {
        let dir = temp_dir("rotate");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("app.log"), vec![b'x'; MAX_FILE_BYTES as usize + 1]).unwrap();
        open_file(&dir, "app").unwrap();
        assert_eq!(fs::metadata(dir.join("app.log")).unwrap().len(), 0);
        assert!(dir.join("app.log.old").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
