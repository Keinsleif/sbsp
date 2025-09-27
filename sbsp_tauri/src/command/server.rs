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
pub async fn set_discovery_option(state: tauri::State<'_, AppState>, discovery_option: Option<String>) -> Result<(), String> {
    log::debug!("setting discovery option: {:?}", discovery_option);
    state.set_discovery_option(discovery_option).await;
    Ok(())
}

#[tauri::command]
pub async fn get_discovery_option(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.get_discovery_option().await)
}

#[tauri::command]
pub async fn start_server(app_handle: tauri::AppHandle ,state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.start(app_handle).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_server(app_handle: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.stop(app_handle).await;
    Ok(())
}

#[tauri::command]
pub async fn open_server_panel(app_handle: tauri::AppHandle) -> Result<(), String> {
    tauri::WebviewWindowBuilder::from_config(&app_handle, &app_handle.config().app.windows[1]).map_err(|e| e.to_string())?.build().map_err(|e| e.to_string())?;
    Ok(())
}