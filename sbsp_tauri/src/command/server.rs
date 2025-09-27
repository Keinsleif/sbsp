use crate::AppState;

#[tauri::command]
pub async fn is_server_running(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_running().await)
}

#[tauri::command]
pub async fn set_server_port(state: tauri::State<'_, AppState>, port: u16) -> Result<(), String> {
    state.set_port(port).await;
    Ok(())
}

#[tauri::command]
pub async fn get_server_port(state: tauri::State<'_, AppState>) -> Result<u16, String> {
    Ok(state.get_port().await)
}

#[tauri::command]
pub async fn start_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.start().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.stop().await;
    Ok(())
}