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
