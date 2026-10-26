// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The browser's own storage, for what the desktop builds keep in files.
//!
//! `localStorage` can be missing or refuse (a private window, blocked site
//! data), so nothing here fails: a read finds nothing, a write is logged.

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
