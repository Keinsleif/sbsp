use std::path::PathBuf;

use sbsp_backend::model::cue::CueParam;
use tauri::Manager as _;
use tauri_plugin_dialog::DialogExt as _;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::AppState;

pub mod controller;
pub mod model_manager;
pub mod server;
pub mod settings;

#[tauri::command]
pub fn get_side() -> String {
    "main".into()
}

#[tauri::command]
pub async fn process_asset(state: tauri::State<'_, AppState>, path: PathBuf) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .asset_processor_handle
        .request_file_asset_data(path)
        .await;
    Ok(())
}

#[tauri::command]
pub fn file_open(app_handle: tauri::AppHandle) {
    let model_handle = app_handle
        .state::<AppState>()
        .get_handle()
        .model_handle
        .clone();
    tokio::spawn(async move {
        let (result_tx, result_rx) = oneshot::channel();
        app_handle.dialog().file().add_filter("Show Model", &["sbsp"]).pick_file(|file_path_option| {
            result_tx.send(file_path_option).unwrap();
        });
        if let Ok(Some(file_path)) = result_rx.await {
            model_handle
                .load_from_file(file_path.into_path().unwrap())
                .await
                .unwrap();
        }
    });
}

#[tauri::command]
pub fn file_save(handle: tauri::AppHandle) {
    let model_handle = handle.state::<AppState>().get_handle().model_handle.clone();
    let file_dialog_builder = handle.dialog().file().add_filter("Show Model", &["sbsp"]);
    tokio::spawn(async move {
        if model_handle.get_current_file_path().await.is_some() {
            model_handle.save().await.unwrap();
        } else {
            let (result_tx, result_rx) = oneshot::channel();
            file_dialog_builder.save_file(move |file_path_option| {
                result_tx.send(file_path_option).unwrap();
            });
            if let Ok(Some(file_path)) = result_rx.await {
                let file_pathbuf = file_path.into_path().unwrap();
                model_handle.save_as(file_pathbuf).await.unwrap();
            }
        }
    });
}

#[tauri::command]
pub fn file_save_as(handle: tauri::AppHandle) {
    let model_handle = handle.state::<AppState>().get_handle().model_handle.clone();
    let file_dialog_builder = handle.dialog().file().add_filter("Show Model", &["sbsp"]);
    tokio::spawn(async move {
        if let Some(current_path) = model_handle.get_current_file_path().await.as_ref() {
            let (result_tx, result_rx) = oneshot::channel();
            file_dialog_builder
                .set_directory(current_path.parent().unwrap())
                .set_file_name(current_path.file_name().unwrap().to_str().unwrap())
                .save_file(move |file_path_option| {
                    result_tx.send(file_path_option).unwrap();
                });
            if let Ok(Some(file_path)) = result_rx.await {
                let file_pathbuf = file_path.into_path().unwrap();
                model_handle.save_as(file_pathbuf).await.unwrap();
            }
        } else {
            let (result_tx, result_rx) = oneshot::channel();
            file_dialog_builder.save_file(move |file_path_option| {
                result_tx.send(file_path_option).unwrap();
            });
            if let Ok(Some(file_path)) = result_rx.await {
                let file_pathbuf = file_path.into_path().unwrap();
                model_handle.save_as(file_pathbuf).await.unwrap();
            }
        }
    });
}

#[tauri::command]
pub fn export_to_folder(handle: tauri::AppHandle) {
    let model_handle = handle.state::<AppState>().get_handle().model_handle.clone();
    let file_dialog_builder = handle.dialog().file();
    tokio::spawn(async move {
        if let Some(current_path) = model_handle.get_current_file_path().await.as_ref() {
            let (result_tx, result_rx) = oneshot::channel();
            file_dialog_builder
                .set_directory(current_path.parent().unwrap())
                .set_file_name(current_path.file_name().unwrap().to_str().unwrap())
                .pick_folder(move |file_path_option| {
                    result_tx.send(file_path_option).unwrap();
                });
            if let Ok(Some(file_path)) = result_rx.await {
                let file_pathbuf = file_path.into_path().unwrap();
                model_handle.export_to_folder(file_pathbuf).await.unwrap();
            }
        } else {
            let (result_tx, result_rx) = oneshot::channel();
            file_dialog_builder.pick_folder(move |file_path_option| {
                result_tx.send(file_path_option).unwrap();
            });
            if let Ok(Some(file_path)) = result_rx.await {
                let file_pathbuf = file_path.into_path().unwrap();
                model_handle.export_to_folder(file_pathbuf).await.unwrap();
            }
        }
    });
}
#[tauri::command]
pub async fn add_empty_cue(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    cue_type: String,
    at_index: usize,
) -> Result<(), String> {
    let handle = state.get_handle();
    let templates = {
        let settings = state.settings_manager.read().await;
        settings.template.clone()
    };
    match cue_type.as_str() {
        "audio" => {
            let (result_tx, result_rx) = oneshot::channel();
            app_handle
                .dialog()
                .file()
                .add_filter(
                    "Audio",
                    &[
                        "aiff", "aif", "caf", "mp4", "m4a", "mkv", "mka", "webm", "ogg", "oga",
                        "wav", "aac", "alac", "flac", "mp3",
                    ],
                )
                .pick_files(|path| result_tx.send(path).unwrap());
            if let Some(file_paths) = result_rx.await.unwrap() {
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
        _ => Err("Invalid cue type.".into()),
    }
}
