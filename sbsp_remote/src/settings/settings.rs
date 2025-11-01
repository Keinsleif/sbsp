use std::path::Path;
use tauri::Manager as _;
use crate::{AppState, settings::GlobalSettings};

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<GlobalSettings, String> {
    Ok(state.settings_manager.read().await.clone())
}

#[tauri::command]
pub async fn set_settings(state: tauri::State<'_, AppState>, new_settings: GlobalSettings) -> Result<(), String> {
    state.settings_manager.update(new_settings).await;
    Ok(())
}

#[tauri::command]
pub async fn reload_settings(handle: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<GlobalSettings, String> {
    if let Ok(path) = handle.path().app_config_dir() {
        let config_path = path.join("config.json");
        if let Err(e) = state.settings_manager.load_from_file(config_path.as_path()).await {
            log::error!("Failed to load config. {}", e);
            Err(format!("Failed to load config. {}", e))
        } else {
            Ok(state.settings_manager.read().await.clone())
        }
    } else {
        Err("Failed to locate config path.".into())
    }
}

#[tauri::command]
pub async fn save_settings(handle: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    if let Ok(path) = handle.path().app_config_dir() {
        let config_path = path.join("config.json");
        if let Err(e) = state.settings_manager.save_to_file(&config_path).await {
            log::error!("Failed to save config. {}", e);
            Err(format!("Failed to save config. {}", e))
        } else {
            Ok(())
        }
    } else {
        Err("Failed to locate config path.".into())
    }
}

#[tauri::command]
pub async fn import_settings_from_file(state: tauri::State<'_, AppState>, path: &Path) -> Result<GlobalSettings, String> {
    state.settings_manager.load_from_file(path).await.map_err(|e| e.to_string())?;
    Ok(state.settings_manager.read().await.clone())
}

#[tauri::command]
pub async fn export_settings_to_file(state: tauri::State<'_, AppState>, path: &Path) -> Result<(), String> {
    state.settings_manager.save_to_file(path).await.map_err(|e| e.to_string())
}
