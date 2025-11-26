use std::path::PathBuf;

use tauri::Manager as _;
use tauri_plugin_dialog::DialogExt as _;
use tokio::sync::oneshot;

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
        app_handle
            .dialog()
            .file()
            .add_filter("Show Model", &["sbsp"])
            .pick_file(|file_path_option| {
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
pub async fn pick_audio_assets(app_handle: tauri::AppHandle) -> Result<Vec<PathBuf>, String> {
    let (result_tx, result_rx) = oneshot::channel();
    app_handle
        .dialog()
        .file()
        .add_filter(
            "Audio",
            &[
                "aiff", "aif", "caf", "mp4", "m4a", "mkv", "mka", "webm", "ogg", "oga", "wav",
                "aac", "alac", "flac", "mp3",
            ],
        )
        .pick_files(|path| {
            let filepath_vec = path.unwrap_or(vec![]);
            let path_vec: Vec<PathBuf> = filepath_vec
                .into_iter()
                .map_while(|item| item.into_path().ok())
                .collect();
            result_tx.send(path_vec).unwrap()
        });
    match result_rx.await {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}
