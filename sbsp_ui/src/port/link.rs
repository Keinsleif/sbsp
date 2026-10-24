// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! What the host's and the remote's backends have in common: a
//! `BackendHandle` (the host's in-process one, or the remote's, which sends
//! the same calls over the websocket) and the broadcast of its events. The
//! UI sends playback commands and model edits and listens to events through
//! this, whichever side it runs on.

use sbsp_backend::BackendHandle;
use tokio::sync::broadcast;

use super::{Control, FullState, ModelEdit, PortEvent};

#[derive(Clone)]
pub struct Link {
    handle: BackendHandle,
    event_tx: broadcast::Sender<PortEvent>,
    /// Which connection this is (0 for the host's one backend). A remote
    /// connects more than once; the event subscription is keyed on this.
    id: u64,
}

impl Link {
    pub fn new(handle: BackendHandle, event_tx: broadcast::Sender<PortEvent>, id: u64) -> Self {
        Self {
            handle,
            event_tx,
            id,
        }
    }

    /// Which connection this is (see `new`).
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn event_tx(&self) -> broadcast::Sender<PortEvent> {
        self.event_tx.clone()
    }

    pub async fn get_full_state(&self) -> anyhow::Result<FullState> {
        self.handle.get_full_state().await
    }

    /// A cloned handle for issuing calls from a detached `Task`, since
    /// the port itself usually lives behind `&self`/`&mut self` on `App`.
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
                ModelEdit::UpdateShowSettings(settings) => model.update_settings(settings).await,
                ModelEdit::RenameShow(name) => model.update_model_name(name).await,
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

    /// Asks for a `SyncState` event (where every playing cue is now), which
    /// corrects the positions the UI works out in between.
    pub fn request_state_sync(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let handle = self.handle.clone();
        async move { handle.request_state_sync().await }
    }

    /// The engine's current peak level per channel, if there is an audio
    /// output to measure. Cheap (two atomic loads), so the UI can poll it
    /// every frame.
    pub fn level(&self) -> Option<(f32, f32)> {
        self.handle.level_meter.as_ref().map(|level| level.get())
    }

    /// A `Subscription` streaming every backend event. Safe to return
    /// (recomputed) from `subscription()` on every call: `Source`'s `Hash`
    /// impl is a constant, so iced treats every call as the same logical
    /// subscription and only spawns the listener once.
    pub fn events(&self) -> iced::Subscription<PortEvent> {
        struct Source(broadcast::Sender<PortEvent>, u64);

        // Deliberately hashes the connection's number rather than the
        // sender itself: it only needs to say "the same logical
        // subscription as last time", and to differ after a reconnect, so
        // the new connection's events are listened to.
        impl std::hash::Hash for Source {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                "sbsp-backend-events".hash(state);
                self.1.hash(state);
            }
        }

        iced::Subscription::run_with(Source(self.event_tx.clone(), self.id), |Source(event_tx, _)| {
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
