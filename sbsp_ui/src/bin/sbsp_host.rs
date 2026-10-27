// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

// No console window behind the app on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> iced::Result {
    // The installer and updater run this program with hooks; it must come
    // before anything else, and it exits for them.
    velopack::VelopackApp::build().run();
    sbsp_ui::logging::init("com.keinsleif.sbsp", "sbsp");
    sbsp_ui::host::run()
}
