use crate::{AppState, settings::GlobalRemoteSettings};
use std::path::Path;

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
) -> Result<GlobalRemoteSettings, String> {
    Ok(state.settings_manager.read().await.clone())
}

#[tauri::command]
pub async fn set_settings(
    state: tauri::State<'_, AppState>,
    new_settings: GlobalRemoteSettings,
) -> Result<(), String> {
    if new_settings != *state.settings_manager.read().await {
        state.settings_manager.update(&new_settings).await;
        if let Err(e) = state.settings_manager.save().await {
            log::error!("Failed to save config. {}", e);
            Err(format!("Failed to save config. {}", e))
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn reload_settings(
    state: tauri::State<'_, AppState>,
) -> Result<GlobalRemoteSettings, String> {
    if let Err(e) = state.settings_manager.load().await {
        log::error!("Failed to load config. {}", e);
        Err(format!("Failed to load config. {}", e))
    } else {
        Ok(state.settings_manager.read().await.clone())
    }
}

#[tauri::command]
pub async fn import_settings_from_file(
    state: tauri::State<'_, AppState>,
    path: &Path,
) -> Result<GlobalRemoteSettings, String> {
    if let Err(e) = state.settings_manager.import_from_file(path).await {
        log::error!("Failed to imported config. {}", e);
        return Err(format!("Failed to imported config. {}", e));
    }
    Ok(state.settings_manager.read().await.clone())
}

#[tauri::command]
pub async fn export_settings_to_file(
    state: tauri::State<'_, AppState>,
    path: &Path,
) -> Result<(), String> {
    state
        .settings_manager
        .export_to_file(path)
        .await
        .map_err(|e| e.to_string())
}
