// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `stores/uiSettings.ts`.
//!
//! There is no new type or logic to add here, by design: `sbsp_frontend_settings::GlobalHostSettings`/
//! `GlobalRemoteSettings` already are the settings state (the original
//! store's `DEFAULT_SETTINGS` constant exists in Rust as
//! `GlobalHostSettings::default()`/`GlobalRemoteSettings::default()`,
//! via that crate's own `impl Default`). Unlike `showModel`/`showState`/
//! `assetResult`, every one of this store's actions (`update`/`reload`/
//! `import_from_file`/`export_to_file`) is a backend API call with no
//! local state logic beyond "replace the stored value with what the call
//! returned" -- so, like `ShowModelState::update_all`, there is no method
//! here worth wrapping a one-line assignment in. `clone()` is likewise
//! just `.clone()`, available for free since both settings types derive
//! `Clone`.
//!
//! host and remote use different concrete types (`GlobalHostSettings` vs
//! `GlobalRemoteSettings`), not an enum of the two: each is a separate
//! binary built with its own `sbsp_ui` feature, so there is no point in
//! the program where a single piece of state needs to hold either one
//! depending on a runtime check.
