use sbsp_backend::{manager::InsertPosition, model::{ShowModel, cue::Cue, settings::ShowSettings}};
use uuid::Uuid;

use crate::AppState;

#[tauri::command]
pub async fn get_show_model(state: tauri::State<'_, AppState>) -> Result<ShowModel, String> {
    let handle = state.get_handle();
    Ok(handle.model_handle.read().await.clone())
}

#[tauri::command]
pub fn is_modified(state: tauri::State<'_, AppState>) -> bool {
    state.get_handle().model_handle.is_modified()
}

#[tauri::command]
pub async fn update_cue(state: tauri::State<'_, AppState>, cue: Cue) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .model_handle
        .update_cue(cue)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_cue(
    state: tauri::State<'_, AppState>,
    cue: Cue,
    target_id: Option<Uuid>,
    to_before: bool,
) -> Result<(), String> {
    let handle = state.get_handle();
    if let Some(target) = target_id {
        if to_before {
            handle
                .model_handle
                .add_cue(cue, InsertPosition::Before { target })
                .await
                .map_err(|e| e.to_string())
        } else {
            handle
                .model_handle
                .add_cue(cue, InsertPosition::After { target })
                .await
                .map_err(|e| e.to_string())
        }
    } else {
        handle
            .model_handle
            .add_cue(cue, InsertPosition::Last)
            .await
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn add_cues(
    state: tauri::State<'_, AppState>,
    cues: Vec<Cue>,
    target_id: Option<Uuid>,
    to_before: bool,
) -> Result<(), String> {
    let handle = state.get_handle();
    if let Some(target) = target_id {
        if to_before {
            handle
                .model_handle
                .add_cues(cues, InsertPosition::Before { target })
                .await
                .map_err(|e| e.to_string())
        } else {
            handle
                .model_handle
                .add_cues(cues, InsertPosition::After { target })
                .await
                .map_err(|e| e.to_string())
        }
    } else {
        handle
            .model_handle
            .add_cues(cues, InsertPosition::Last)
            .await
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn remove_cue(state: tauri::State<'_, AppState>, cue_id: Uuid) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .model_handle
        .remove_cue(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn move_cue(
    state: tauri::State<'_, AppState>,
    cue_id: Uuid,
    target_id: Option<Uuid>,
) -> Result<(), String> {
    let handle = state.get_handle();
    if let Some(target) = target_id {
        handle
            .model_handle
            .move_cue(cue_id, InsertPosition::Before { target })
            .await
            .map_err(|e| e.to_string())
    } else {
        handle
            .model_handle
            .move_cue(cue_id, InsertPosition::Last)
            .await
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn renumber_cues(
    state: tauri::State<'_, AppState>,
    cues: Vec<Uuid>,
    start_from: f64,
    increment: f64,
) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .model_handle
        .renumber_cues(cues, start_from, increment)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_model_name(
    state: tauri::State<'_, AppState>,
    new_name: String,
) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .model_handle
        .update_model_name(new_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_show_settings(
    state: tauri::State<'_, AppState>,
    new_settings: ShowSettings,
) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .model_handle
        .update_settings(new_settings)
        .await
        .map_err(|e| e.to_string())
}
