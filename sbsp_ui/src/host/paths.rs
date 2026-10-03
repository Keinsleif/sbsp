// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Host filesystem paths, per the migration plan's section 5.2.
//!
//! A free function for now, not part of a `platform/` abstraction: that
//! abstraction is worth building once `remote` (Phase 8) needs its own
//! version of this (a different identifier, otherwise the same shape) --
//! premature to design the trait boundary around a single caller.

use std::path::PathBuf;

/// Matches the existing Tauri build's `identifier`, so settings saved by
/// that version are found by this one (see the migration plan's risk
/// table: "保存済み設定との非互換").
const IDENTIFIER: &str = "com.keinsleif.sbsp";

/// `dirs::config_dir()/<identifier>/config.json`. `None` if the OS config
/// directory itself can't be resolved (should not normally happen).
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(IDENTIFIER).join("config.json"))
}
