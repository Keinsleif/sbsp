// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The boundary between the UI and the backend it talks to.
//!
//! [`Port`] is the backend of a native build: the host's own, or the one a
//! remote is connected to. What the two share is on [`Link`], which `Port`
//! derefs to. The UI sends the backend [`Control`]s (playback) and
//! [`ModelEdit`]s (the cue list) as data, and receives [`PortEvent`]s and a
//! [`FullState`] when it connects.
//!
//! On the native targets those event and state types are `sbsp_backend`'s
//! own, because both `host` and `remote` link its `backend` feature. The
//! browser cannot link it (it pulls in the audio and file code), so the web
//! build has its own `Link` (`port/web_link.rs`) and `wire`, which turns
//! `Control` and `ModelEdit` into the protocol's commands and reads the
//! host's answers.

#[cfg(any(feature = "host", feature = "remote"))]
mod link;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[path = "port/web_link.rs"]
pub(crate) mod link;
#[cfg(feature = "web")]
pub mod wire;

#[cfg(backend)]
pub use link::Link;
#[cfg(backend)]
pub use sbsp_backend::FullShowState as FullState;
#[cfg(backend)]
pub use sbsp_backend::event::BackendEvent as PortEvent;

/// The backend the app talks to: the host's own, or the one a remote is
/// connected to. Everything the two share is on [`Link`], which this
/// derefs to; what only one has is reached through `host()` / `remote()`.
#[cfg(backend)]
#[derive(Clone)]
pub enum Port {
    #[cfg(feature = "host")]
    Host(crate::host::backend::HostPort),
    #[cfg(client)]
    Remote(crate::remote::backend::RemotePort),
}

#[cfg(backend)]
impl std::ops::Deref for Port {
    type Target = Link;

    fn deref(&self) -> &Link {
        match self {
            #[cfg(feature = "host")]
            Port::Host(port) => port,
            #[cfg(client)]
            Port::Remote(port) => port,
        }
    }
}

#[cfg(backend)]
impl Port {
    #[cfg(feature = "host")]
    pub fn host(&self) -> Option<&crate::host::backend::HostPort> {
        match self {
            Port::Host(port) => Some(port),
            #[cfg(client)]
            _ => None,
        }
    }

    #[cfg(client)]
    pub fn remote(&self) -> Option<&crate::remote::backend::RemotePort> {
        match self {
            Port::Remote(port) => Some(port),
            #[cfg(feature = "host")]
            _ => None,
        }
    }
}
