// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Phase 1 minimal host-side backend connection.
//!
//! Deliberately skips two things that belong to later phases:
//! - Settings persistence (Phase 2/7): starts the backend with
//!   `GlobalHostSettings::default()` and never loads or saves anything.
//! - Command coverage (Phase 4+): only exposes `get_full_state()`, since
//!   Phase 1's own goal is just "start the backend, fetch the full state
//!   once, receive the ongoing event stream" (see the migration plan).
//!
//! `HostPort` is kept out of `Message` entirely rather than routed through
//! it: `BackendHandle` only derives `Clone`, not `Debug`, and
//! `FullShowState` derives neither `Send`-adjacent nor `Debug`. Since our
//! `Message` enum derives `Debug`, embedding either directly would break
//! that derive. `HostPort` lives as a plain field on `App`; only the two
//! things Phase 1 actually needs out of it -- the fetched `FullState` and
//! each `PortEvent` -- travel through `Message` (both are `Clone`, and
//! `PortEvent`/`BackendEvent` also happens to be `Debug`).

use sbsp_backend::{BackendHandle, BackendSettings, start_backend};
use sbsp_frontend_settings::GlobalHostSettings;
use tokio::sync::{broadcast, watch};

use crate::port::{FullState, PortEvent};

pub struct HostPort {
    handle: BackendHandle,
    event_tx: broadcast::Sender<PortEvent>,
    // Keeping the sender alive keeps the watch channel open. Settings are
    // never pushed through it yet (Phase 2/7), but a dropped sender would
    // mark the channel closed under the backend, which some internal loops
    // may treat as a shutdown signal.
    _settings_tx: watch::Sender<BackendSettings>,
}

impl HostPort {
    /// Starts the backend in-process.
    ///
    /// Must be called from a task already running on a Tokio runtime:
    /// `start_backend` spawns its manager tasks with `tokio::spawn`
    /// internally. It is currently called directly and synchronously from
    /// `App::new()` (see `prototype.rs`), on the assumption that iced's
    /// `application()` boot closure already runs inside the runtime it
    /// uses for `Task`/`Subscription` execution. That assumption is
    /// untested. If this panics (something like "there is no reactor
    /// running" / "must be called from the context of a Tokio 1.x
    /// runtime"), the fix is to move this call into the app's initial
    /// `Task` instead and bridge the result back out-of-band (e.g. a
    /// `OnceLock`) rather than through `Message`, for the `Debug`/`Clone`
    /// reasons above.
    pub fn start() -> anyhow::Result<Self> {
        let (settings_tx, settings_rx) =
            watch::channel(BackendSettings::from(&GlobalHostSettings::default()));

        let (handle, _state_rx, event_tx) = start_backend(settings_rx, true)?;

        Ok(Self {
            handle,
            event_tx,
            _settings_tx: settings_tx,
        })
    }

    pub async fn get_full_state(&self) -> anyhow::Result<FullState> {
        self.handle.get_full_state().await
    }

    /// A cloned handle for issuing calls from a detached `Task` (e.g. the
    /// boot task in `prototype.rs`), since `HostPort` itself is not `Clone`
    /// and lives behind `&self`/`&mut self` on `App`.
    pub fn handle(&self) -> BackendHandle {
        self.handle.clone()
    }

    /// A `Subscription` streaming every backend event. Safe to return
    /// (recomputed) from `subscription()` on every call: `Source`'s `Hash`
    /// impl is a constant, so iced treats every call as the same logical
    /// subscription and only spawns the listener once.
    pub fn events(&self) -> iced::Subscription<PortEvent> {
        struct Source(broadcast::Sender<PortEvent>);

        // Deliberately hashes a constant rather than the sender itself:
        // this is a singleton (one backend per app run), so identity here
        // only needs to say "the same logical subscription as last time",
        // not distinguish between instances.
        impl std::hash::Hash for Source {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                "sbsp-backend-events".hash(state);
            }
        }

        iced::Subscription::run_with(Source(self.event_tx.clone()), |Source(event_tx)| {
            let mut event_rx = event_tx.subscribe();

            iced::stream::channel(64, async move |mut output| {
                use iced::futures::SinkExt;

                loop {
                    match event_rx.recv().await {
                        Ok(event) => {
                            if output.send(event).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            log::warn!(
                                "Backend event subscriber lagged, skipped {skipped} events"
                            );
                        }
                    }
                }
            })
        })
    }
}
