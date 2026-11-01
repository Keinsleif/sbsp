// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! SBSP iced UI. The library is shared by the host, remote and web entry points.

#[cfg(all(feature = "web", any(feature = "host", feature = "remote")))]
compile_error!("feature `web` cannot be combined with `host` or `remote`");

pub mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod atomic_write;
pub mod domain;
pub mod features;
#[cfg(any(feature = "host", feature = "remote"))]
pub mod files;
pub mod i18n;
pub mod menu;
pub mod persist;
pub mod port;
pub mod power;
pub mod settings;
pub mod state;
pub mod theme;
pub mod time;
#[cfg(installer)]
pub mod update;
pub mod widgets;

#[cfg(not(target_arch = "wasm32"))]
pub mod logging;

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(feature = "host")]
pub mod host;
#[cfg(client)]
pub mod remote;
#[cfg(feature = "web")]
pub mod web;
