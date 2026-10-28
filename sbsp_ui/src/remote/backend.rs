// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The remote's side of the backend boundary: a websocket connection to a
//! host, and the discovery of hosts on the local network.
//!
//! `create_remote_backend` hands back the same `BackendHandle` the host
//! has, whose calls go over the connection instead of to an in-process
//! backend. So most of what the UI does (playback commands, model edits,
//! events) is on the shared [`Link`]; this adds what only a connection has:
//! the permissions the host granted, the address, and the way to end it.

use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use crate::settings::{GlobalRemoteSettings, manager::SettingsManager};

use sbsp_backend::api::{
    FileList, Permissions,
    client::{FileListHandle, ServiceEntry, create_remote_backend, start_discovery},
};
use tokio::sync::mpsc;

use crate::port::{FullState, Link};

/// Numbers the connections, so each has its own event subscription.
static NEXT_CONNECTION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct RemotePort {
    link: Link,
    address: String,
    permission: Permissions,
    file_list: FileListHandle,
    disconnect_tx: mpsc::Sender<()>,
}

impl std::ops::Deref for RemotePort {
    type Target = Link;

    fn deref(&self) -> &Link {
        &self.link
    }
}

impl RemotePort {
    /// Connects to `address` (`host:port`), authenticates with `password`
    /// (none when the host has no password for the permissions it grants)
    /// and fetches the host's full state: everything the UI needs for its
    /// first frame, in one call for `Task::perform`.
    pub async fn connect(
        address: String,
        password: Option<String>,
    ) -> anyhow::Result<(Self, FullState)> {
        let (handle, event_tx, file_list, disconnect_tx, permission) =
            create_remote_backend(address.clone(), password).await?;
        let full_state = handle.get_full_state().await?;
        let id = NEXT_CONNECTION.fetch_add(1, Ordering::Relaxed);

        let port = Self {
            link: Link::new(handle, event_tx, id),
            address,
            permission,
            file_list,
            disconnect_tx,
        };
        Ok((port, full_state))
    }

    /// What the host lets this connection do.
    pub fn permission(&self) -> Permissions {
        self.permission
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    /// Asks the connection to close.
    pub fn disconnect(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let tx = self.disconnect_tx.clone();
        async move {
            let _ = tx.send(()).await;
        }
    }

    /// Finishes when the connection has ended, whether this side closed it
    /// or the host (or the network) did.
    pub fn closed(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let tx = self.disconnect_tx.clone();
        async move { tx.closed().await }
    }

    /// Asks the host for the list of files it can offer; the answer comes
    /// through the file list handle.
    pub fn request_file_list(
        &self,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let file_list = self.file_list.clone();
        async move {
            file_list.request_file_list().await?;
            Ok(())
        }
    }
}

impl RemotePort {
    /// The lists of files the host sends after `request_file_list`, as they
    /// arrive. Keyed on the connection, like the events.
    pub fn file_lists(&self) -> iced::Subscription<Vec<FileList>> {
        struct Source(FileListHandle, u64);

        impl std::hash::Hash for Source {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                "sbsp-file-lists".hash(state);
                self.1.hash(state);
            }
        }

        iced::Subscription::run_with(
            Source(self.file_list.clone(), self.link.id()),
            |Source(handle, _)| {
                let mut handle = handle.clone();
                iced::stream::channel(8, async move |mut output| {
                    use iced::futures::SinkExt;

                    while let Ok(list) = handle.recv_file_list().await {
                        if output.send(list).await.is_err() {
                            break;
                        }
                    }
                })
            },
        )
    }
}

/// Matches the Tauri remote's `identifier`, so the settings it saved are
/// found by this one.
const IDENTIFIER: &str = "com.keinsleif.sbsp.remote";

/// `dirs::config_dir()/<identifier>/config.json`.
fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(IDENTIFIER).join("config.json"))
}

/// The remote's own settings file. These belong to this computer, not to
/// a connection, so they are there before (and between) connections.
#[derive(Clone)]
pub struct RemoteSettings(Arc<SettingsManager<GlobalRemoteSettings>>);

impl RemoteSettings {
    /// Loads the settings, or falls back to the defaults (logging why: it
    /// is expected on a first run).
    pub async fn load() -> (Self, GlobalRemoteSettings) {
        let manager = Arc::new(SettingsManager::<GlobalRemoteSettings>::new(config_path()));
        let settings = match manager.load().await {
            Ok(loaded) => loaded,
            Err(e) => {
                log::info!("using default settings ({e})");
                GlobalRemoteSettings::default()
            }
        };
        (Self(manager), settings)
    }

    /// Makes `new` the settings in force and writes them to the file.
    pub fn save(
        &self,
        new: GlobalRemoteSettings,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let manager = self.0.clone();
        async move {
            manager.set(new).await;
            manager.save().await
        }
    }

    /// Reads settings from `path` and makes them the settings in force.
    pub fn import(
        &self,
        path: PathBuf,
    ) -> impl std::future::Future<Output = anyhow::Result<GlobalRemoteSettings>> + Send + 'static
    {
        let manager = self.0.clone();
        async move { manager.import_from_file(&path).await }
    }

    pub fn export(
        &self,
        path: PathBuf,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let manager = self.0.clone();
        async move { manager.export_to_file(&path).await }
    }
}

/// The hosts found on the local network, as a list that is sent again each
/// time it changes. Looking stops when the subscription is dropped.
pub fn discovery() -> iced::Subscription<Vec<ServiceEntry>> {
    iced::Subscription::run(|| {
        iced::stream::channel(8, async move |mut output| {
            use iced::futures::SinkExt;

            let mut services = match start_discovery() {
                Ok(services) => services,
                Err(e) => {
                    log::error!("server discovery failed: {e}");
                    // Nothing to report and nothing to retry; keep the
                    // subscription (and so the error) from restarting.
                    std::future::pending::<()>().await;
                    return;
                }
            };
            // The list as it is now, including empty: a fresh look must
            // replace whatever an earlier one left on screen.
            let first = services.borrow_and_update().clone();
            if output.send(first).await.is_err() {
                return;
            }
            while services.changed().await.is_ok() {
                let list = services.borrow_and_update().clone();
                if output.send(list).await.is_err() {
                    break;
                }
            }
        })
    })
}
