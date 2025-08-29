use sbsp_backend::{model::{cue::Cue, settings::ShowSettings, ShowModel}, BackendHandle};
use uuid::Uuid;


#[tauri::command]
pub async fn get_show_model(handle: tauri::State<'_, BackendHandle>) -> Result<ShowModel, String> {
    Ok(handle.model_handle.read().await.clone())
}

#[tauri::command]
pub async fn update_cue(handle: tauri::State<'_, BackendHandle>, cue: Cue) -> Result<(), String> {
    handle
        .model_handle
        .update_cue(cue)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_cue(
    handle: tauri::State<'_, BackendHandle>,
    cue: Cue,
    at_index: usize,
) -> Result<(), String> {
    handle
        .model_handle
        .add_cue(cue, at_index)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_cue(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .model_handle
        .remove_cue(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn move_cue(
    handle: tauri::State<'_, BackendHandle>,
    cue_id: Uuid,
    to_index: usize,
) -> Result<(), String> {
    handle
        .model_handle
        .move_cue(cue_id, to_index)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_settings(
    handle: tauri::State<'_, BackendHandle>,
    new_settings: ShowSettings,
) -> Result<(), String> {
    handle
        .model_handle
        .update_settings(new_settings)
        .await
        .map_err(|e| e.to_string())
}