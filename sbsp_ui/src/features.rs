// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Domain-specific TEA modules:each owns its own view logic and `Message`
//! type, nested into the app's via `.map()`, the same pattern used for
//! `widgets::toast`.

#[cfg(client)]
pub mod connect;
pub mod cue_list;
pub mod dialog;
pub mod editor;
pub mod file_ops;
pub mod header;
pub mod hotkeys;
pub mod mobile;
pub mod side_bar;
pub mod status_bar;
pub mod viewer;
