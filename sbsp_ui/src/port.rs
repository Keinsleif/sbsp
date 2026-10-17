// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The backend boundary (see the migration plan, section 4.2).
//!
//! Phase 1 scope only: this currently just aliases `sbsp_backend`'s own
//! types rather than defining independent `PortCommand`/`PortEvent` enums.
//! That works today because both `host` (via the `server` feature) and
//! `remote` (via `client`) pull in `sbsp_backend`'s `backend` feature, so
//! `FullShowState` and `BackendEvent` are the same concrete Rust type on
//! both targets.
//!
//! This stops being true once `web` needs its own port (Phase 9): wasm
//! cannot enable the `backend` feature (it pulls in cpal, rodio, etc.), so
//! the web build will only have `sbsp_backend::api` (the `protocol`
//! feature) and will need to decode these from JSON instead. At that point
//! `PortEvent`/`FullState` should become real enums/structs defined here,
//! with `From`/`TryFrom` conversions on each platform, rather than a
//! shared alias. Not needed yet, so not done yet.
#[cfg(any(feature = "host", feature = "remote"))]
pub use sbsp_backend::FullShowState as FullState;
#[cfg(any(feature = "host", feature = "remote"))]
pub use sbsp_backend::event::BackendEvent as PortEvent;

/// A playback command from the UI to whichever backend it talks to. Kept
/// as data (not a call on `HostPort`) so a remote/web port can later
/// serialize the same values instead of invoking a local handle.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    Execute(sbsp_backend::model::cue::Uuid),
    Load(sbsp_backend::model::cue::Uuid),
    Pause(sbsp_backend::model::cue::Uuid),
    Resume(sbsp_backend::model::cue::Uuid),
    Stop(sbsp_backend::model::cue::Uuid),
    SeekBy(sbsp_backend::model::cue::Uuid, f64),
    SeekTo(sbsp_backend::model::cue::Uuid, f64),
    ToggleRepeat(sbsp_backend::model::cue::Uuid),
    /// Change the volume (dB) of a playing audio cue.
    SetVolume(sbsp_backend::model::cue::Uuid, f64),
    PauseAll,
    ResumeAll,
    StopAll,
}

/// An edit of the show model: the data counterpart of `Control` for the
/// cue list. Not `PartialEq` because `InsertPosition` is not.
#[derive(Debug, Clone)]
pub enum ModelEdit {
    /// New cues, in order, at `position`.
    AddCues {
        cues: Vec<sbsp_backend::model::cue::Cue>,
        position: sbsp_backend::manager::InsertPosition,
    },
    /// A new group cue that then takes `members` into itself (at the
    /// top, in the backend's order).
    AddGroup {
        group: sbsp_backend::model::cue::Cue,
        position: sbsp_backend::manager::InsertPosition,
        members: Vec<sbsp_backend::model::cue::Uuid>,
    },
    RemoveCues(Vec<sbsp_backend::model::cue::Uuid>),
    /// Replaces the cue with the same id.
    UpdateCue(sbsp_backend::model::cue::Cue),
}
