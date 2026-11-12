// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Where the host keeps its files.

use std::path::PathBuf;

/// The application identifier, which names the host's directory under the
/// OS config directory. It is also the identifier of the installed
/// package, so it must not change: settings saved under the old name would
/// no longer be found.
const IDENTIFIER: &str = "com.keinsleif.sbsp";

/// `dirs::config_dir()/<identifier>/config.json`. `None` if the OS config
/// directory itself can't be resolved (should not normally happen).
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(IDENTIFIER).join("config.json"))
}
