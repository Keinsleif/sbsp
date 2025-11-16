use std::path::PathBuf;

use super::AppState;
use tauri::Listener;
use tokio::sync::oneshot;

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
pub async fn pick_audio_assets(
    app_handle: tauri::AppHandle,
) -> Result<Vec<PathBuf>, String> {
    let pick_file_window = tauri::WebviewWindowBuilder::from_config(
        &app_handle,
        &app_handle.config().app.windows[2],
    )
    .map_err(|e| e.to_string())?
    .build()
    .map_err(|e| e.to_string())?;
    let (result_tx, result_rx) = oneshot::channel();
    pick_file_window.once("file-select-result", |event| {
        let path_vec = serde_json::from_str::<Option<Vec<PathBuf>>>(
            event.payload(),
        ).unwrap().unwrap_or_default();
        let _ = result_tx.send(path_vec);
    });
    let _ = pick_file_window.close();
    match result_rx.await {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}
