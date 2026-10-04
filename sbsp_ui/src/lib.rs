// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! SBSP iced UI. The library is shared by the host, remote and web entry points.

#[cfg(all(feature = "web", any(feature = "host", feature = "remote")))]
compile_error!("feature `web` cannot be combined with `host` or `remote`");

pub mod app;
pub mod domain;
pub mod i18n;
pub mod menu;
pub mod port;
pub mod state;
pub mod theme;
pub mod widgets;

#[cfg(not(target_arch = "wasm32"))]
pub mod logging;

#[cfg(feature = "host")]
pub mod host;
#[cfg(feature = "remote")]
pub mod remote;
#[cfg(feature = "web")]
pub mod web;
