use sbsp_backend::{asset_processor::AssetData, model::cue::CueParam};
use tauri::Manager as _;
use tauri_plugin_dialog::DialogExt as _;
use uuid::Uuid;

use crate::AppState;

pub mod controller;
pub mod model_manager;
pub mod server;

#[tauri::command]
pub fn get_side() -> String {
    "main".into()
}

#[tauri::command]
pub async fn process_asset(
    state: tauri::State<'_, AppState>,
    cue_id: Uuid,
) -> Result<(Uuid, AssetData), String> {
    let handle = state.get_handle();
    if let Some(cue) = handle
        .model_handle
        .read()
        .await
        .clone()
        .cues
        .iter()
        .find(|cue| cue.id == cue_id)
        && let CueParam::Audio(params) = &cue.params
    {
        handle
            .asset_processor_handle
            .request_file_asset_data(params.target.clone())
            .await
            .map_err(|e| e.to_string())
            .map(|asset_data| (cue_id, asset_data))
    } else {
        Err(format!("Cue not found. id={}", cue_id))
    }
}

#[tauri::command]
pub fn file_open(app_handle: tauri::AppHandle) {
    let model_handle = app_handle.state::<AppState>().get_handle().model_handle.clone();
    app_handle.dialog().file().pick_folder(|file_path_option| {
        if let Some(file_path) = file_path_option {
            tauri::async_runtime::spawn(async move {
                model_handle
                    .load_from_file(file_path.into_path().unwrap())
                    .await
                    .unwrap();
            });
        }
    });
}

#[tauri::command]
pub fn file_save(handle: tauri::AppHandle) {
    let model_handle = handle.state::<AppState>().get_handle().model_handle.clone();
    let file_dialog_builder = handle.dialog().file().add_filter("Show Model", &["sbsp"]);
    tauri::async_runtime::spawn(async move {
        if model_handle.get_current_file_path().await.is_some() {
            model_handle.save().await.unwrap();
        } else {
            file_dialog_builder.save_file(move |file_path_option| {
                if let Some(file_path) = file_path_option {
                    let file_pathbuf = file_path.into_path().unwrap();
                    tauri::async_runtime::spawn(async move {
                        model_handle.save_as(file_pathbuf).await.unwrap();
                    });
                }
            })
        }
    });
}

#[tauri::command]
pub fn file_save_as(handle: tauri::AppHandle) {
    let model_handle = handle.state::<AppState>().get_handle().model_handle.clone();
    let file_dialog_builder = handle.dialog().file().add_filter("Show Model", &["sbsp"]);
    tauri::async_runtime::spawn(async move {
        if let Some(current_path) = model_handle.get_current_file_path().await {
            file_dialog_builder
                .set_directory(current_path.parent().unwrap())
                .set_file_name(current_path.file_name().unwrap().to_str().unwrap())
                .save_file(move |file_path_option| {
                    if let Some(file_path) = file_path_option {
                        let file_pathbuf = file_path.into_path().unwrap();
                        tauri::async_runtime::spawn(async move {
                            model_handle.save_as(file_pathbuf).await.unwrap();
                        });
                    }
                })
        } else {
            file_dialog_builder.save_file(move |file_path_option| {
                if let Some(file_path) = file_path_option {
                    let file_pathbuf = file_path.into_path().unwrap();
                    tauri::async_runtime::spawn(async move {
                        model_handle.save_as(file_pathbuf).await.unwrap();
                    });
                }
            })
        }
    });
}

#[tauri::command]
pub async fn add_empty_cue(app_handle: tauri::AppHandle, state: tauri::State<'_, AppState>, cue_type: String, at_index: usize) -> Result<(), String> {
    let handle = state.get_handle();
    let model_lock = handle.model_handle.read().await;
    let templates = model_lock.settings.template.clone();
    drop(model_lock);
    match cue_type.as_str() {
        "audio" => {
            let file_paths_option = app_handle.dialog().file().add_filter("Audio", &[
                "aiff",
                "aif",
                "caf",
                "mp4",
                "m4a",
                "mkv",
                "mka",
                "webm",
                "ogg",
                "oga",
                "wav",
                "aac",
                "alac",
                "flac",
                "mp3",
            ]).blocking_pick_files();
            if let Some(file_paths) = file_paths_option {
                if file_paths.len() == 1 {
                    let mut new_cue = templates.audio.clone();
                    new_cue.id = Uuid::new_v4();
                    if let CueParam::Audio(cue_param) = &mut new_cue.params {
                        cue_param.target = file_paths[0].clone().into_path().unwrap();
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
                            cue_param.target = file_path.clone().into_path().unwrap();
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
        _ => Err("Invalid cue type.".into())
    }
}