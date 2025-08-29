use sbsp_backend::{controller::ControllerCommand, BackendHandle};
use uuid::Uuid;

#[tauri::command]
pub async fn go(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Go)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Load(cue_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Pause(cue_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Resume(cue_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Stop(cue_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_to(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid, position: f64) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::SeekTo(cue_id, position))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_by(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid, amount: f64) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::SeekBy(cue_id, amount))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_playback_cursor(
    handle: tauri::State<'_, BackendHandle>,
    cue_id: Option<Uuid>,
) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::SetPlaybackCursor { cue_id })
        .await
        .map_err(|e| e.to_string())
}