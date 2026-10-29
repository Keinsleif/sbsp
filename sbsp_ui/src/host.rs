// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(target_os = "macos")]
pub mod native_menu;

pub mod backend;
pub mod paths;
pub mod web_assets;

pub fn run() -> iced::Result {
    crate::app::run("host")
}
