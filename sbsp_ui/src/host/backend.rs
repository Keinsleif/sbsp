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

use sbsp_backend::{BackendHandle, BackendSettings, start_backend};
use sbsp_frontend_settings::{GlobalHostSettings, manager::SettingsManager};
use tokio::sync::{broadcast, watch};

use crate::port::{Control, FullState, ModelEdit, PortEvent};

#[derive(Clone)]
pub struct HostPort {
    handle: BackendHandle,
    event_tx: broadcast::Sender<PortEvent>,
    // Keeping the sender alive keeps the watch channel open. Settings are
    // never pushed through it yet (Phase 7, once there's a settings
    // dialog), but a dropped sender would mark the channel closed under
    // the backend, which some internal loops may treat as a shutdown
    // signal.
    _settings_tx: watch::Sender<BackendSettings>,
    settings: Arc<SettingsManager<GlobalHostSettings>>,
}

impl HostPort {
    /// Starts the backend in-process: loads settings (or falls back to
    /// defaults, logging why), starts the backend with them, and fetches
    /// the full show state -- everything the app needs before it can show
    /// its first real frame, in one `Task::perform`-able call.
    pub async fn start() -> anyhow::Result<(Self, FullState)> {
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

        let (handle, _state_rx, event_tx) = start_backend(settings_rx, true)?;
        let full_state = handle.get_full_state().await?;

        let port = Self {
            handle,
            event_tx,
            _settings_tx: settings_tx,
            settings,
        };

        Ok((port, full_state))
    }

    pub async fn get_full_state(&self) -> anyhow::Result<FullState> {
        self.handle.get_full_state().await
    }

    /// A cloned handle for issuing calls from a detached `Task`, since
    /// `HostPort` itself usually lives behind `&self`/`&mut self` on `App`.
    pub fn handle(&self) -> BackendHandle {
        self.handle.clone()
    }

    /// Sends playback commands in order. Stops at the first failure, since
    /// a failed send means the controller is gone and the rest would fail
    /// the same way.
    ///
    /// The returned future owns what it needs (a cloned handle), so it can
    /// be handed straight to `Task::perform` without borrowing `self`.
    pub fn send(
        &self,
        controls: Vec<Control>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        use sbsp_backend::action::{AudioAction, CueAction};

        let controller = self.handle.controller_handle.clone();
        async move {
            for control in controls {
                match control {
                    Control::Execute(id) => controller.execute(id).await?,
                    Control::Load(id) => controller.load(id).await?,
                    Control::Pause(id) => controller.pause(id).await?,
                    Control::Resume(id) => controller.resume(id).await?,
                    Control::Stop(id) => controller.stop(id).await?,
                    Control::SeekBy(id, amount) => controller.seek_by(id, amount).await?,
                    Control::SeekTo(id, position) => controller.seek_to(id, position).await?,
                    Control::ToggleRepeat(id) => {
                        controller
                            .perform_action(id, CueAction::Audio(AudioAction::ToggleRepeat))
                            .await?
                    }
                    Control::SetVolume(id, volume) => {
                        controller
                            .perform_action(
                                id,
                                CueAction::Audio(AudioAction::SetVolume(
                                    sbsp_backend::model::cue::audio::Decibels::from(volume as f32),
                                )),
                            )
                            .await?
                    }
                    Control::PauseAll => controller.pause_all().await?,
                    Control::ResumeAll => controller.resume_all().await?,
                    Control::StopAll => controller.stop_all().await?,
                }
            }
            Ok(())
        }
    }

    /// Applies a model edit. The result comes back as `CueListUpdated`
    /// (and `CueRemoved`), like any other change; this only reports
    /// whether the backend took the command.
    pub fn edit(
        &self,
        edit: ModelEdit,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let handle = self.handle.clone();
        async move {
            let model = &handle.model_handle;
            match edit {
                ModelEdit::AddCues { mut cues, position } => match cues.len() {
                    0 => Ok(()),
                    1 => model.add_cue(cues.remove(0), position).await,
                    _ => model.add_cues(cues, position).await,
                },
                ModelEdit::AddGroup {
                    group,
                    position,
                    members,
                } => {
                    let group_id = group.id;
                    model.add_cue(group, position).await?;
                    if members.is_empty() {
                        return Ok(());
                    }
                    model
                        .move_cues(
                            members.into_iter().collect(),
                            sbsp_backend::manager::InsertPosition::Inside {
                                target: Some(group_id),
                                index: Some(0),
                            },
                        )
                        .await
                }
                ModelEdit::RemoveCues(ids) => model.remove_cues(ids.into_iter().collect()).await,
                ModelEdit::MoveCues { ids, position } => {
                    model.move_cues(ids.into_iter().collect(), position).await
                }
                ModelEdit::RenumberCues {
                    ids,
                    start_from,
                    increment,
                    prefix,
                    suffix,
                } => {
                    model
                        .renumber_cues(ids, start_from, increment, prefix, suffix)
                        .await
                }
                ModelEdit::UpdateCue(cue) => model.update_cue(cue).await,
            }
        }
    }

    /// Asks the asset processor to analyze `path` (metadata, waveform,
    /// peak). The answer comes back later as `AssetMetadata`/`AssetResult`
    /// events, so this only reports whether the request was queued.
    pub fn request_asset(
        &self,
        path: std::path::PathBuf,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send + 'static {
        let processor = self.handle.asset_processor_handle.clone();
        async move { processor.request_file_asset_data(path).await }
    }

    /// The engine's current peak level per channel, if there is an audio
    /// output to measure. Cheap (two atomic loads), so the UI can poll it
    /// every frame.
    pub fn level(&self) -> Option<(f32, f32)> {
        self.handle.level_meter.as_ref().map(|level| level.get())
    }

    pub fn settings(&self) -> Arc<SettingsManager<GlobalHostSettings>> {
        self.settings.clone()
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
                            log::warn!("Backend event subscriber lagged, skipped {skipped} events");
                        }
                    }
                }
            })
        })
    }
}
