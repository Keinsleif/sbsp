use std::{path::PathBuf, time::Duration};

use super::AppState;
use tauri::{Listener, ipc::Channel};
use tokio::{sync::oneshot, time::interval};

pub mod client;
pub mod controller;
pub mod model_manager;
pub mod settings;

#[tauri::command]
pub fn get_side() -> String {
    "remote".into()
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
pub async fn pick_audio_assets(app_handle: tauri::AppHandle) -> Result<Vec<PathBuf>, String> {
    let pick_file_window =
        tauri::WebviewWindowBuilder::from_config(&app_handle, &app_handle.config().app.windows[2])
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;
    let (result_tx, result_rx) = oneshot::channel();
    pick_file_window.once("file-select-result", |event| {
        let path_vec = serde_json::from_str::<Option<Vec<PathBuf>>>(event.payload())
            .unwrap()
            .unwrap_or_default();
        let _ = result_tx.send(path_vec);
    });
    let result = result_rx.await;
    let _ = pick_file_window.close();
    match result {
        Ok(file_list) => Ok(file_list),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn listen_level_meter(state: tauri::State<'_, AppState>, level_listener: Channel<(f32, f32)>) -> Result<(), String> {
    Err("Level Meter is not implemented on remote.".into())
}
