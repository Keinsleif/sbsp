// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The browser's own storage, for what the desktop builds keep in files.
//!
//! `localStorage` can be missing or refuse (a private window, blocked site
//! data), so nothing here fails: a read finds nothing, a write is logged.

use std::sync::LazyLock;
use web_sys::window;

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn get(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

pub fn set(key: &str, value: &str) {
    match storage() {
        Some(storage) => {
            if storage.set_item(key, value).is_err() {
                log::warn!("could not write {key} to the browser's storage");
            }
        }
        None => log::debug!("the browser's storage is not available"),
    }
}

/// In the Wasm only, perform a runtime check upon the initial access and cache the result.
pub static IS_MACOS: LazyLock<bool> = LazyLock::new(|| {
    let window = match window() {
        Some(w) => w,
        None => return false,
    };
    let navigator = window.navigator();

    if let Ok(platform) = navigator.platform() {
        if platform.to_lowercase().contains("mac") {
            return true;
        }
    }
    if let Ok(user_agent) = navigator.user_agent() {
        if user_agent.to_lowercase().contains("macintosh") || user_agent.to_lowercase().contains("mac os x") {
            return true;
        }
    }
    false
});