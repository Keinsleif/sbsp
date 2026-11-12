// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! SBSP iced UI. The library is shared by the host, remote and web entry points.

#[cfg(all(feature = "web", any(feature = "host", feature = "remote")))]
compile_error!("feature `web` cannot be combined with `host` or `remote`");

// pub mod app;
pub mod domain;
pub mod features;
pub mod i18n;
pub mod menu;
pub mod persist;
pub mod platform;
pub mod port;
pub mod settings;
pub mod state;
pub mod theme;
pub mod widgets;

#[cfg(feature = "host")]
pub mod host;
#[cfg(client)]
pub mod remote;
#[cfg(feature = "web")]
pub mod web;
