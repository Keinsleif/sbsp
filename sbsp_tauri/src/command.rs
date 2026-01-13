use std::path::PathBuf;

use tauri::{Manager as _, ipc::Channel, path::BaseDirectory};
use tauri_plugin_dialog::DialogExt as _;
use tokio::sync::oneshot;

use crate::AppState;

pub mod controller;
pub mod model_manager;
pub mod server;
pub mod settings;
pub mod license;

#[tauri::command]
pub async fn get_third_party_notices(app_handle: tauri::AppHandle) -> Result<String, String> {
    let resource_path = app_handle.path().resolve("THIRD_PARTY_NOTICES.md", BaseDirectory::Resource).map_err(|e| e.to_string())?;
    tokio::fs::read_to_string(&resource_path).await.map_err(|e| e.to_string())
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
pub async fn file_open(app_handle: tauri::AppHandle) -> Result<(), String> {
    let model_handle = app_handle
        .state::<AppState>()
        .get_handle()
        .model_handle;
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
            .load_from_file(file_path.into_path().map_err(|e| e.to_string())?)
            .await.map_err(|e| e.to_string())?
    }
    Ok(())
}

#[tauri::command]
pub async fn file_new(handle: tauri::AppHandle) -> Result<(), String> {
    let handle = handle.state::<AppState>().get_handle();
    handle.model_handle.reset().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn file_save(app_handle: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let handle = state.get_handle();
    let file_dialog_builder = app_handle.dialog().file().add_filter("Show Model", &["sbsp"]);
    if handle.model_handle.get_current_file_path().await.is_some() {
        handle.model_handle.save().await.map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        let (result_tx, result_rx) = oneshot::channel();
        file_dialog_builder.save_file(move |file_path_option| {
            let _ = result_tx.send(file_path_option);
        });
        if let Ok(Some(file_path)) = result_rx.await {
            let file_pathbuf = file_path.into_path().map_err(|e| e.to_string())?;
            handle.model_handle.save_as(file_pathbuf).await.map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn file_save_as(app_handle: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let handle = state.get_handle();
    let file_dialog_builder = app_handle.dialog().file().add_filter("Show Model", &["sbsp"]);
    if let Some(current_path) = handle.model_handle.get_current_file_path().await.as_ref() {
        let (result_tx, result_rx) = oneshot::channel();
        file_dialog_builder
            .set_directory(current_path.parent().unwrap())
            .set_file_name(current_path.file_name().unwrap().to_str().unwrap())
            .save_file(move |file_path_option| {
                result_tx.send(file_path_option).unwrap();
            });
        if let Ok(Some(file_path)) = result_rx.await {
            let file_pathbuf = file_path.into_path().map_err(|e| e.to_string())?;
            handle.model_handle.save_as(file_pathbuf).await.map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        let (result_tx, result_rx) = oneshot::channel();
        file_dialog_builder.save_file(move |file_path_option| {
            result_tx.send(file_path_option).unwrap();
        });
        if let Ok(Some(file_path)) = result_rx.await {
            let file_pathbuf = file_path.into_path().map_err(|e| e.to_string())?;
            handle.model_handle.save_as(file_pathbuf).await.map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn export_to_folder(app_handle: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let handle = state.get_handle();
    let file_dialog_builder = app_handle.dialog().file();
    if let Some(current_path) = handle.model_handle.get_current_file_path().await.as_ref() {
        let (result_tx, result_rx) = oneshot::channel();
        file_dialog_builder
            .set_directory(current_path.parent().unwrap())
            .set_file_name(current_path.file_name().unwrap().to_str().unwrap())
            .pick_folder(move |file_path_option| {
                result_tx.send(file_path_option).unwrap();
            });
        if let Ok(Some(file_path)) = result_rx.await {
            let file_pathbuf = file_path.into_path().map_err(|e| e.to_string())?;
            handle.model_handle.export_to_folder(file_pathbuf).await.map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        let (result_tx, result_rx) = oneshot::channel();
        file_dialog_builder.pick_folder(move |file_path_option| {
            result_tx.send(file_path_option).unwrap();
        });
        if let Ok(Some(file_path)) = result_rx.await {
            let file_pathbuf = file_path.into_path().map_err(|e| e.to_string())?;
            handle.model_handle.export_to_folder(file_pathbuf).await.map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn listen_level_meter(state: tauri::State<'_, AppState>, level_listener: Channel<(f32, f32)>) -> Result<(), String> {
    state.level_meter_tx.send_modify(|channel| {
        *channel = Some(level_listener);
    });
    Ok(())
}
