use sbsp_backend::api::{ApiServerOptions, server::get_mdns_hostname};

use crate::AppState;

#[tauri::command]
pub async fn is_server_running(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_running().await)
}

#[tauri::command]
pub async fn set_server_options(
    state: tauri::State<'_, AppState>,
    options: ApiServerOptions,
) -> Result<(), String> {
    state.set_server_options(options).await;
    Ok(())
}

#[tauri::command]
pub async fn get_server_options(
    state: tauri::State<'_, AppState>,
) -> Result<ApiServerOptions, String> {
    Ok(state.get_server_options().await)
}

#[tauri::command]
pub fn get_hostname() -> Result<String, String> {
    get_mdns_hostname().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_server(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.start(app_handle).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_server(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.stop(app_handle).await;
    Ok(())
}
