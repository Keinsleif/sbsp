// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Host-side backend connection.
//!
//! `HostPort::start()` now does two things Phase 1's version didn't:
//! loads persisted settings (falling back to defaults on first run or a
//! read error) instead of always using `GlobalHostSettings::default()`,
//! and fetches the full show state as part of the same call, rather than
//! as a separate step after construction.
//!
//! That second change is what let `HostPort` move out of `App::new()` and
//! into a proper boot `Task` (see `prototype.rs`), resolving the risk
//! flagged in Phase 1's patch: starting the backend synchronously inside
//! `App::new()`, on the unverified assumption that iced's boot closure
//! already runs inside the tokio runtime it uses for `Task`/`Subscription`
//! execution. `start_backend` spawns its manager tasks with `tokio::spawn`
//! internally, which needs a runtime; a `Task::perform`'d async fn is
//! guaranteed to run inside one. The other half of that original
//! workaround -- keeping `HostPort` out of `Message` because
//! `BackendHandle`/`FullShowState` aren't `Debug` -- is no longer needed
//! either: `Message` only derives `Clone` now (dropped `Debug` in the
//! Phase 1 patch, for the same reason), and every field of `HostPort` is a
//! cheaply-`Clone`-able handle (a `tokio::sync` `Sender`, or a `BackendHandle`
//! that is itself just a bundle of those), so `HostPort: Clone` costs
//! nothing beyond what `App` already pays to hold one.

use std::sync::Arc;

use sbsp_backend::api::{ApiServerOptions, server::start_apiserver_with};
use sbsp_backend::controller::state::ShowState;
use sbsp_backend::{BackendSettings, start_backend};
use sbsp_frontend_settings::{GlobalHostSettings, manager::SettingsManager};
use sbsp_license::{LicenseManager, data::LicenseInformation};
use tokio::sync::{broadcast, watch};

use crate::port::{FullState, Link};

#[derive(Clone)]
pub struct HostPort {
    link: Link,
    // The backend's view of the settings (audio hardware and the like).
    // Keeping the sender alive also keeps the channel open: a dropped one
    // would look like a shutdown to the backend's loops.
    settings_tx: Arc<watch::Sender<BackendSettings>>,
    settings: Arc<SettingsManager<GlobalHostSettings>>,
    state_rx: watch::Receiver<ShowState>,
    server: Arc<tokio::sync::Mutex<Server>>,
    license: Arc<LicenseManager>,
}

/// The API server's options, and the way to stop it while it runs.
struct Server {
    options: ApiServerOptions,
    shutdown: Option<broadcast::Sender<()>>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            options: ApiServerOptions {
                port: 5800,
                discoverry: None,
                auth_map: vec![],
            },
            shutdown: None,
        }
    }
}

/// The output devices and what they support. Asking the audio host can
/// take a moment, so it runs off the UI thread.
pub async fn hardware() -> anyhow::Result<sbsp_backend::helper::SupportedHardware> {
    tokio::task::spawn_blocking(sbsp_backend::helper::get_supported_hardware).await?
}

/// The key licenses are signed with (the same as the Tauri build's).
const PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAdlqW6bS6NMn2cdf2b4Ot1DNyjoytP2uFqoH+WlG+NeI=
-----END PUBLIC KEY-----";

/// A license manager with the license file saved earlier, if there is one.
fn license_manager() -> LicenseManager {
    let manager = LicenseManager::new_from_pem(PUBLIC_KEY_PEM);
    if let Some(path) = super::paths::license_path().filter(|path| path.exists())
        && let Err(e) = manager.activate_by_file(path)
    {
        log::warn!("saved license not usable: {e}");
    }
    manager
}

impl std::ops::Deref for HostPort {
    type Target = Link;

    fn deref(&self) -> &Link {
        &self.link
    }
}

impl HostPort {
    /// Starts the backend in-process: loads settings (or falls back to
    /// defaults, logging why), starts the backend with them, and fetches
    /// the full show state -- everything the app needs before it can show
    /// its first real frame, in one `Task::perform`-able call.
    pub async fn start() -> anyhow::Result<(Self, FullState, GlobalHostSettings)> {
        let settings = Arc::new(SettingsManager::<GlobalHostSettings>::new(
            super::paths::config_path(),
        ));

        let host_settings = match settings.load().await {
            Ok(loaded) => loaded,
            Err(e) => {
                // Expected on first run (no file yet) as well as on a
                // genuine read error; either way, defaults are a safe
                // fallback and not worth distinguishing the two here.
                log::info!("using default settings ({e})");
                GlobalHostSettings::default()
            }
        };

        let (settings_tx, settings_rx) = watch::channel(BackendSettings::from(&host_settings));

        let (handle, state_rx, event_tx) = start_backend(settings_rx, true)?;
        let full_state = handle.get_full_state().await?;

        let port = Self {
            link: Link::new(handle, event_tx, 0),
            settings_tx: Arc::new(settings_tx),
            settings,
            state_rx,
            server: Arc::new(tokio::sync::Mutex::new(Server::default())),
            license: Arc::new(license_manager()),
        };

        Ok((port, full_state, host_settings))
    }

    /// Makes `new` the settings in force: keeps them, writes them to the
    /// settings file and hands the backend its part.
    pub fn save_settings(
        &self,
        new: GlobalHostSettings,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let settings = self.settings.clone();
        let tx = self.settings_tx.clone();
        async move {
            settings.set(new.clone()).await;
            tx.send_replace(BackendSettings::from(&new));
            settings.save().await
        }
    }

    pub fn license_info(&self) -> Option<LicenseInformation> {
        self.license.get_license_info()
    }

    pub fn is_pro(&self) -> bool {
        self.license.is_pro()
    }

    /// Checks the license file, and keeps a copy of it next to the
    /// settings so it is found at the next start.
    pub fn activate_license(
        &self,
        path: std::path::PathBuf,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let license = self.license.clone();
        async move {
            license.activate_by_file(path.clone())?;
            if let Some(saved) = super::paths::license_path() {
                if let Some(directory) = saved.parent() {
                    tokio::fs::create_dir_all(directory).await?;
                }
                if saved != path {
                    tokio::fs::copy(&path, &saved).await?;
                }
            }
            Ok(())
        }
    }

    /// The server's options and whether it runs.
    pub fn server_status(
        &self,
    ) -> impl std::future::Future<Output = (ApiServerOptions, bool)> + Send + 'static {
        let server = self.server.clone();
        async move {
            let server = server.lock().await;
            (server.options.clone(), server.shutdown.is_some())
        }
    }

    /// Starts the server with these options (kept for next time).
    pub fn start_server(
        &self,
        options: ApiServerOptions,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let server = self.server.clone();
        let handle = self.link.handle();
        let state_rx = self.state_rx.clone();
        let event_tx = self.link.event_tx();
        async move {
            let mut server = server.lock().await;
            server.options = options.clone();
            if server.shutdown.is_some() {
                return Ok(());
            }
            let shutdown = start_apiserver_with(
                handle,
                state_rx,
                event_tx,
                options,
                super::web_assets::extend,
            )
            .await?;
            server.shutdown = Some(shutdown);
            Ok(())
        }
    }

    pub fn stop_server(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let server = self.server.clone();
        async move {
            if let Some(shutdown) = server.lock().await.shutdown.take() {
                let _ = shutdown.send(());
            }
        }
    }

    /// Reads settings from `path`, makes them the settings in force and
    /// keeps them. The settings file is replaced.
    pub fn import_settings(
        &self,
        path: std::path::PathBuf,
    ) -> impl std::future::Future<Output = anyhow::Result<GlobalHostSettings>> + Send + 'static {
        let settings = self.settings.clone();
        let tx = self.settings_tx.clone();
        async move {
            let imported = settings.import_from_file(&path).await?;
            tx.send_replace(BackendSettings::from(&imported));
            Ok(imported)
        }
    }

    /// Writes the settings in force to `path`.
    pub fn export_settings(
        &self,
        path: std::path::PathBuf,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let settings = self.settings.clone();
        async move { settings.export_to_file(&path).await }
    }

    pub fn settings(&self) -> Arc<SettingsManager<GlobalHostSettings>> {
        self.settings.clone()
    }
}
