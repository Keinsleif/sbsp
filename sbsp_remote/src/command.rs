use std::path::PathBuf;

use super::AppState;
use sbsp_backend::model::cue::CueParam;
use tauri::Listener;
use tokio::sync::oneshot;
use uuid::Uuid;

pub mod client;
pub mod controller;
pub mod model_manager;

#[tauri::command]
pub fn get_side() -> String {
    "remote".into()
}

#[tauri::command]
pub async fn process_asset(
    state: tauri::State<'_, AppState>,
    path: PathBuf,
) -> Result<(), String> {
    if let Some(handle) = state.get_handle().await {
        handle
            .asset_processor_handle
            .request_file_asset_data(path)
            .await;
        Ok(())
    } else {
        Err("Not connected.".into())
    }
}

#[tauri::command]
pub async fn add_empty_cue(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    cue_type: String,
    at_index: usize,
) -> Result<(), String> {
    if let Some(handle) = state.get_handle().await {
        let templates = handle.model_handle.read().await.settings.template.clone();
        match cue_type.as_str() {
            "audio" => {
                let pick_file_window = tauri::WebviewWindowBuilder::from_config(
                    &app_handle,
                    &app_handle.config().app.windows[2],
                )
                .map_err(|e| e.to_string())?
                .build()
                .map_err(|e| e.to_string())?;
                let (result_tx, result_rx) = oneshot::channel();
                pick_file_window.once("file-select-result", |event| {
                    let _ = result_tx.send(serde_json::from_str::<Option<Vec<PathBuf>>>(
                        event.payload(),
                    ));
                });
                let result = result_rx.await.unwrap().map_err(|e| e.to_string())?;
                let _ = pick_file_window.close();
                if let Some(file_paths) = result {
                    if file_paths.len() == 1 {
                        let mut new_cue = templates.audio.clone();
                        new_cue.id = Uuid::new_v4();
                        if let CueParam::Audio(cue_param) = &mut new_cue.params {
                            cue_param.target = file_paths[0].clone();
                        }
                        handle
                            .model_handle
                            .add_cue(new_cue, at_index)
                            .await
                            .map_err(|e| e.to_string())
                    } else {
                        let mut new_cues = vec![];
                        for file_path in file_paths {
                            let mut new_cue = templates.audio.clone();
                            new_cue.id = Uuid::new_v4();
                            if let CueParam::Audio(cue_param) = &mut new_cue.params {
                                cue_param.target = file_path.clone();
                            }
                            new_cues.push(new_cue);
                        }
                        handle
                            .model_handle
                            .add_cues(new_cues, at_index)
                            .await
                            .map_err(|e| e.to_string())
                    }
                } else {
                    Ok(())
                }
            }
            "wait" => {
                let mut new_cue = templates.wait.clone();
                new_cue.id = Uuid::new_v4();
                handle
                    .model_handle
                    .add_cue(new_cue, at_index)
                    .await
                    .map_err(|e| e.to_string())
            }
            _ => Err("Invalid cue type.".into()),
        }
    } else {
        Err("Not connected.".into())
    }
}
