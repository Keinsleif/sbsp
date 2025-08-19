use std::str::FromStr;

use sbsp_backend::{
    controller::{state::ShowState, ControllerCommand}, event::UiEvent, model::{cue::Cue, settings::ShowSettings, ShowModel}, start_backend, BackendHandle
};
use tauri::{
    AppHandle, Emitter, Manager as _,
    menu::{Menu, MenuId, MenuItem, SubmenuBuilder},
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};
use tokio::sync::{broadcast, watch};
use uuid::Uuid;

async fn forward_backend_state_and_event(
    app_handle: AppHandle,
    mut state_rx: watch::Receiver<ShowState>,
    mut event_rx: broadcast::Receiver<UiEvent>,
) {
    loop {
        tokio::select! {
            Ok(_) = state_rx.changed() => {
                let state = state_rx.borrow().clone();
                app_handle.emit("backend-state-update", state).ok();
            },
            Ok(event) = event_rx.recv() => {
                app_handle.emit("backend-event", event).ok();
            }
            else => break,
        }
    }
}

#[tauri::command]
async fn go(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Go)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pause(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Pause)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn resume(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Resume)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Stop)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn load(handle: tauri::State<'_, BackendHandle>) -> Result<(), String> {
    handle
        .controller_tx
        .send(ControllerCommand::Load)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_playback_cursor(
    handle: tauri::State<'_, BackendHandle>,
    cue_id: Option<String>,
) -> Result<(), String> {
    let cursor = if let Some(cue_id_string) = cue_id {
        match Uuid::from_str(&cue_id_string) {
            Ok(uuid) => Some(uuid),
            Err(e) => return Err(e.to_string()),
        }
    } else {
        None
    };
    handle
        .controller_tx
        .send(ControllerCommand::SetPlaybackCursor { cue_id: cursor })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_show_model(handle: tauri::State<'_, BackendHandle>) -> Result<ShowModel, String> {
    Ok(handle.model_handle.read().await.clone())
}

#[tauri::command]
async fn update_cue(handle: tauri::State<'_, BackendHandle>, cue: Cue) -> Result<(), String> {
    handle
        .model_handle
        .update_cue(cue)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_cue(
    handle: tauri::State<'_, BackendHandle>,
    cue: Cue,
    at_index: usize,
) -> Result<(), String> {
    handle
        .model_handle
        .add_cue(cue, at_index)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_cue(handle: tauri::State<'_, BackendHandle>, cue_id: &str) -> Result<(), String> {
    match Uuid::from_str(cue_id) {
        Ok(cue_uuid) => handle
            .model_handle
            .remove_cue(cue_uuid)
            .await
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn move_cue(
    handle: tauri::State<'_, BackendHandle>,
    cue_id: &str,
    to_index: usize,
) -> Result<(), String> {
    match Uuid::from_str(cue_id) {
        Ok(cue_uuid) => handle
            .model_handle
            .move_cue(cue_uuid, to_index)
            .await
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn update_settings(handle: tauri::State<'_, BackendHandle>, new_settings: ShowSettings) -> Result<(), String> {
    handle.model_handle.update_settings(new_settings).await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle();

            let menu = Menu::new(app_handle)?;
            menu.append(
                &SubmenuBuilder::new(app_handle, "File")
                    .items(&[
                        &MenuItem::with_id(
                            app_handle,
                            MenuId::new("id_open"),
                            "Open",
                            true,
                            Some("Ctrl+O"),
                        )?,
                        &MenuItem::with_id(
                            app_handle,
                            MenuId::new("id_save"),
                            "Save",
                            true,
                            Some("Ctrl+S"),
                        )?,
                        &MenuItem::with_id(
                            app_handle,
                            MenuId::new("id_save_as"),
                            "Save As...",
                            true,
                            Some("Ctrl+Shift+S"),
                        )?,
                    ])
                    .separator()
                    .text(MenuId::new("id_quit"), "Quit")
                    .build()?,
            )?;

            app.set_menu(menu)?;

            let (backend_handle, state_rx, event_tx) = start_backend();

            tauri::async_runtime::spawn(forward_backend_state_and_event(
                app_handle.clone(),
                state_rx,
                event_tx.subscribe(),
            ));

            app.manage(backend_handle);

            let main_window = app.get_webview_window("main").unwrap();
            main_window.restore_state(StateFlags::all()).unwrap();
            Ok(())
        })
        .on_menu_event(|handle, event| match event.id().as_ref() {
            "id_open" => {
                let model_handle = handle.state::<BackendHandle>().model_handle.clone();
                handle.dialog().file().pick_file(|file_path_option| {
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
            "id_save" => {
                let model_handle = handle.state::<BackendHandle>().model_handle.clone();
                let file_dialog_builder =
                    handle.dialog().file().add_filter("Show Model", &["json"]);
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
            "id_save_as" => {
                let model_handle = handle.state::<BackendHandle>().model_handle.clone();
                let file_dialog_builder =
                    handle.dialog().file().add_filter("Show Model", &["json"]);
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
            "id_quit" => {
                handle.cleanup_before_exit();
                std::process::exit(0);
            }
            _ => {}
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window
                    .app_handle()
                    .save_window_state(StateFlags::all())
                    .unwrap();
            }
        })
        .invoke_handler(tauri::generate_handler![
            go,
            pause,
            resume,
            stop,
            load,
            get_show_model,
            set_playback_cursor,
            update_cue,
            add_cue,
            remove_cue,
            move_cue,
            update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
