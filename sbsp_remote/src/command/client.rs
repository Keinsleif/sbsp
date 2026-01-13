use tauri::AppHandle;

use crate::AppState;

#[tauri::command]
pub async fn is_connected(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_connected().await)
}

#[tauri::command]
pub async fn get_server_address(
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    Ok(state.get_address().await)
}

#[tauri::command]
pub async fn connect_to_server(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<(), String> {
    state
        .connect(address, app_handle.clone())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disconnect_from_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.disconnect().await;
    Ok(())
}

#[tauri::command]
pub async fn start_server_discovery(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.start_discovery(app_handle).await;
    Ok(())
}

#[tauri::command]
pub async fn stop_server_discovery(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.stop_discovery().await;
    Ok(())
}

#[tauri::command]
pub async fn request_file_list(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.request_file_list().await.map_err(|e| e.to_string())
}
