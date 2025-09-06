use sbsp_backend::BackendHandle;
use uuid::Uuid;

#[tauri::command]
pub async fn go(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_handle
        .go()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .controller_handle
        .load(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .controller_handle
        .pause(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .controller_handle
        .resume(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid) -> Result<(), String> {
    handle
        .controller_handle
        .stop(cue_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause_all(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_handle
        .pause_all()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume_all(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_handle
        .resume_all()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_all(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_handle
        .stop_all()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_to(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid, position: f64) -> Result<(), String> {
    handle
        .controller_handle
        .seek_to(cue_id, position)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_by(handle: tauri::State<'_, BackendHandle>, cue_id: Uuid, amount: f64) -> Result<(), String> {
    handle
        .controller_handle
        .seek_by(cue_id, amount)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_playback_cursor(
    handle: tauri::State<'_, BackendHandle>,
    cue_id: Option<Uuid>,
) -> Result<(), String> {
    handle
        .controller_handle
        .set_playback_cursor(cue_id)
        .await
        .map_err(|e| e.to_string())
}