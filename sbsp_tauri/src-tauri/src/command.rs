use sbsp_backend::{asset_processor::AssetData, model::cue::CueParam, BackendHandle};
use tauri::Manager as _;
use tauri_plugin_dialog::DialogExt as _;
use uuid::Uuid;

pub mod controller;
pub mod model_manager;

#[tauri::command]
pub async fn process_asset(
    handle: tauri::State<'_, BackendHandle>,
    cue_id: Uuid,
) -> Result<(Uuid, AssetData), String> {
    if let Some(cue) = handle
        .model_handle
        .read()
        .await
        .cues
        .iter()
        .find(|cue| cue.id == cue_id)
        && let CueParam::Audio { target, .. } = &cue.params
    {
        handle
            .asset_handle
            .request_file_asset_data(target.clone())
            .await
            .map_err(|e| e.to_string())
            .map(|asset_data| (cue_id, asset_data))
    } else {
        Err(format!("Cue not found. id={}", cue_id))
    }
}

#[tauri::command]
pub fn file_open(app_handle: tauri::AppHandle) {
    let model_handle = app_handle.state::<BackendHandle>().model_handle.clone();
    app_handle.dialog().file().pick_file(|file_path_option| {
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
    let model_handle = handle.state::<BackendHandle>().model_handle.clone();
    let file_dialog_builder = handle.dialog().file().add_filter("Show Model", &["json"]);
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
    let model_handle = handle.state::<BackendHandle>().model_handle.clone();
    let file_dialog_builder = handle.dialog().file().add_filter("Show Model", &["json"]);
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