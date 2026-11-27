// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The host's connection to the backend it runs in the same process.
//!
//! [`HostPort::start`] loads the saved settings (defaults on a first run or a
//! read error), starts the backend with them and fetches the full show
//! state, so that one call gives the app everything it needs for its first
//! frame. It must be run as a `Task`: `start_backend` spawns its manager
//! tasks with `tokio::spawn`, which needs the runtime that iced runs tasks
//! on.
//!
//! [`HostPort`] derefs to [`Link`], the handle the app sends commands
//! through, and adds what only the host has: saving the settings, the
//! output devices, and the API server's options and lifetime. It is cheap
//! to clone: every field is a handle (a channel end, an `Arc`, or the
//! backend handle, itself a bundle of those).

use std::sync::Arc;

use sbsp_backend::api::{ApiServerOptions, server::start_apiserver_with};
use sbsp_backend::controller::state::ShowState;
use sbsp_backend::{BackendSettings, start_backend};
use tokio::sync::{broadcast, watch};

use crate::port::{FullState, Link};

#[derive(Clone)]
pub struct HostPort {
    link: Link,
    state_rx: watch::Receiver<ShowState>,
    server: Arc<tokio::sync::Mutex<Server>>,
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
                trust_ip_headers: false,
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
    pub async fn start(
        settings_rx: watch::Receiver<BackendSettings>,
    ) -> anyhow::Result<(Self, FullState)> {
        let (handle, state_rx, event_tx) = start_backend(settings_rx, true)?;
        let full_state = handle.get_full_state().await?;

        let port = Self {
            link: Link::new(handle, event_tx, 0),
            state_rx,
            server: Arc::new(tokio::sync::Mutex::new(Server::default())),
        };

        Ok((port, full_state))
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
}
