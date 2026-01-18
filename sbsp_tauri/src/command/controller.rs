use sbsp_backend::action::{AudioAction, CueAction};
use uuid::Uuid;

use crate::AppState;

#[tauri::command]
pub async fn go(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .go()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load(state: tauri::State<'_, AppState>, cue_id: Uuid) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .load(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause(state: tauri::State<'_, AppState>, cue_id: Uuid) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .pause(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume(state: tauri::State<'_, AppState>, cue_id: Uuid) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .resume(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop(state: tauri::State<'_, AppState>, cue_id: Uuid) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .stop(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .pause_all()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .resume_all()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .stop_all()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_to(
    state: tauri::State<'_, AppState>,
    cue_id: Uuid,
    position: f64,
) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .seek_to(cue_id, position)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_by(
    state: tauri::State<'_, AppState>,
    cue_id: Uuid,
    amount: f64,
) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .seek_by(cue_id, amount)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_playback_cursor(
    state: tauri::State<'_, AppState>,
    cue_id: Option<Uuid>,
) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .set_playback_cursor(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_repeat(state: tauri::State<'_, AppState>, cue_id: Uuid) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .perform_action(cue_id, CueAction::Audio(AudioAction::ToggleRepeat))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_volume(
    state: tauri::State<'_, AppState>,
    cue_id: Uuid,
    volume: f32,
) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .perform_action(cue_id, CueAction::Audio(AudioAction::SetVolume(volume)))
        .await
        .map_err(|e| e.to_string())
}
