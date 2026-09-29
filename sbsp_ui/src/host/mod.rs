// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(target_os = "macos")]
pub mod native_menu;

pub fn run() -> iced::Result {
    crate::prototype::run("host")
}
