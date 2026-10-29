// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(feature = "remote")]
pub mod backend;
// The browser's connection has the same names, on the browser's WebSocket.
#[cfg(not(feature = "remote"))]
mod web_backend;
#[cfg(not(feature = "remote"))]
pub mod backend {
    pub use super::web_backend::*;
}

#[cfg(feature = "remote")]
pub fn run() -> iced::Result {
    crate::app::run("remote")
}
