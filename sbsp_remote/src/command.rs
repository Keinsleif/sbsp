use std::path::PathBuf;

use super::AppState;
use tauri::{Manager, ipc::Channel, path::BaseDirectory};

pub mod client;
pub mod controller;
pub mod model_manager;
pub mod settings;

#[tauri::command]
pub async fn get_third_party_notices(app_handle: tauri::AppHandle) -> Result<String, String> {
    let resource_path = app_handle.path().resolve("THIRD_PARTY_NOTICES.md", BaseDirectory::Resource).map_err(|e| e.to_string())?;
    tokio::fs::read_to_string(&resource_path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn process_asset(state: tauri::State<'_, AppState>, path: PathBuf) -> Result<(), String> {
    if let Some(handle) = state.get_handle().await {
        handle
            .asset_processor_handle
            .request_file_asset_data(path)
            .await;
        Ok(())
    } else {
        Err("Not connected.".into())
    }
}

#[tauri::command]
pub async fn listen_level_meter(_state: tauri::State<'_, AppState>, _level_listener: Channel<(f32, f32)>) -> Result<(), String> {
    Err("Level Meter is not implemented on remote.".into())
}
