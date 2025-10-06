// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main(worker_threads=4)]
async fn main() {
    #[cfg(feature = "console-subscriber")]
    console_subscriber::init();

    sbsp_tauri_lib::run()
}
