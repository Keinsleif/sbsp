// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Domain-specific TEA modules (see the migration plan, section 4.3):
//! each owns its own view logic and `Message` type, nested into the app's
//! via `.map()`, the same pattern used for `widgets::toast`.

#[cfg(client)]
pub mod connect;
pub mod credits;
pub mod cue_list;
pub mod editor;
pub mod file_ops;
#[cfg(client)]
pub mod file_list;
pub mod header;
pub mod license_dialog;
pub mod hotkeys;
pub mod renumber;
pub mod server_panel;
pub mod settings_dialog;
pub mod side_bar;
pub mod viewer;
pub mod status_bar;
