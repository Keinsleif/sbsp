// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

pub mod backend;

pub fn run() -> iced::Result {
    crate::app::run("remote")
}
