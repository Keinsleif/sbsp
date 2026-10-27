// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

// No console window behind the app on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> iced::Result {
    // The installer and updater run this program with hooks; it must come
    // before anything else, and it exits for them.
    velopack::VelopackApp::build().run();
    sbsp_ui::logging::init("com.keinsleif.sbsp.remote", "sbsp_remote");
    // After a Velopack install, the Tauri version it replaces is removed.
    sbsp_ui::migrate::remove_tauri_install("SBS Player Remote");
    sbsp_ui::remote::run()
}
