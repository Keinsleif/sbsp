// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Updates of the installed app, through Velopack.
//!
//! Only an app installed by a Velopack package can update itself; a build
//! run from the source tree, or installed from a `.deb`, cannot, and the
//! update dialog says so. Velopack's calls block, so they run on tokio's
//! blocking threads.
//!
//! The feed is `<base>/<target>` (`host` or `remote`), where `<base>` is
//! `SBSP_UPDATE_BASE_URL` at build time.

use std::sync::Arc;

use iced::futures::{SinkExt, Stream};
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, sources::HttpSource};

const DEFAULT_BASE: &str = "https://keinsleif.github.io/sbsp/update";

/// The feed of one of the two apps.
fn feed(target: &str) -> String {
    let base = option_env!("SBSP_UPDATE_BASE_URL").unwrap_or(DEFAULT_BASE);
    format!("{}/{target}", base.trim_end_matches('/'))
}

fn manager(target: &str) -> Result<UpdateManager, String> {
    UpdateManager::new(HttpSource::new(&feed(target)), None, None).map_err(|e| e.to_string())
}

/// An update that was found.
#[derive(Clone)]
pub struct Found(Arc<UpdateInfo>);

impl Found {
    pub fn version(&self) -> String {
        self.0.TargetFullRelease.Version.clone()
    }
}

/// Whether this copy was installed by Velopack, so that it can update.
pub fn installed(target: &str) -> bool {
    manager(target).is_ok()
}

/// Asks the feed whether there is a newer version.
pub async fn check(target: &'static str) -> Result<Option<Found>, String> {
    tokio::task::spawn_blocking(move || {
        let manager = manager(target)?;
        match manager.check_for_updates().map_err(|e| e.to_string())? {
            UpdateCheck::UpdateAvailable(info) => Ok(Some(Found(Arc::from(info)))),
            _ => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, PartialEq)]
pub enum Progress {
    /// Percent downloaded.
    Percent(u8),
    Done,
    Failed(String),
}

/// Downloads the update, reporting how far it got.
pub fn download(target: &'static str, found: Found) -> impl Stream<Item = Progress> {
    iced::stream::channel(16, async move |mut output| {
        let (sender, receiver) = std::sync::mpsc::channel::<i16>();
        let (forward, mut percents) = tokio::sync::mpsc::unbounded_channel::<i16>();
        // The download reports on a std channel; this thread moves it onto
        // an async one. It ends when the download drops its sender.
        std::thread::spawn(move || {
            for percent in receiver {
                if forward.send(percent).is_err() {
                    break;
                }
            }
        });
        let job = tokio::task::spawn_blocking(move || {
            let manager = manager(target)?;
            manager
                .download_updates(&found.0, Some(sender))
                .map_err(|e| e.to_string())
        });
        while let Some(percent) = percents.recv().await {
            let percent = u8::try_from(percent.clamp(0, 100)).unwrap_or(0);
            let _ = output.send(Progress::Percent(percent)).await;
        }
        let last = match job.await {
            Ok(Ok(())) => Progress::Done,
            Ok(Err(error)) => Progress::Failed(error.to_string()),
            Err(error) => Progress::Failed(error.to_string()),
        };
        let _ = output.send(last).await;
    })
}

/// Installs the downloaded update and restarts. On success the process
/// ends, so this only returns with the reason it failed.
pub async fn apply(target: &'static str, found: Found) -> String {
    tokio::task::spawn_blocking(move || {
        let manager = manager(target)?;
        manager
            .apply_updates_and_restart(&*found.0)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|result| result)
    .err()
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_app_has_its_own_feed() {
        assert!(feed("host").ends_with("/host"));
        assert!(feed("remote").ends_with("/remote"));
        assert!(!feed("host").contains("//host"));
    }

    #[test]
    fn a_build_that_was_not_installed_cannot_update() {
        assert!(!installed("host"));
    }
}
