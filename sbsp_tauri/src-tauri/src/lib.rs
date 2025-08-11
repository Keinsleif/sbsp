use std::str::FromStr;

use sbsp_backend::{
    controller::{state::ShowState, ControllerCommand},
    event::UiEvent,
    model::{cue::Cue, ShowModel},
    start_backend, BackendHandle,
};
use tauri::{
    menu::{Menu, MenuId, MenuItem, SubmenuBuilder},
    AppHandle, Emitter, LogicalSize, Manager as _, Size,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle();

            let menu = Menu::new(app_handle)?;
            menu.append(
                &SubmenuBuilder::new(app_handle, "File")
                    .items(&[&MenuItem::with_id(
                        app_handle,
                        MenuId::new("id_open"),
                        "Open",
                        true,
                        Some("Ctro+O"),
                    )?])
                    .separator()
                    .text(MenuId::new("id_quit"), "Quit")
                    .build()?,
            )?;

            app.set_menu(menu)?;

            app.on_menu_event(|handle, event| match event.id().as_ref() {
                "id_open" => {
                    if let Some(file_path) = handle.dialog().file().blocking_pick_file() {
                        let model_handle = handle.state::<BackendHandle>().model_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            model_handle
                                .load_from_file(file_path.into_path().unwrap())
                                .await
                                .unwrap();
                        });
                    }
                }
                "id_quit" => {
                    handle.cleanup_before_exit();
                    std::process::exit(0);
                }
                _ => {}
            });

            let (backend_handle, state_rx, event_tx) = start_backend();

            tauri::async_runtime::spawn(forward_backend_state_and_event(
                app_handle.clone(),
                state_rx,
                event_tx.subscribe(),
            ));

            app.manage(backend_handle);

            let main_window = app.get_webview_window("main").unwrap();
            if main_window.restore_state(StateFlags::all()).is_err() {
                main_window
                    .set_size(Size::Logical(LogicalSize {
                        width: 1280.0,
                        height: 720.0,
                    }))
                    .unwrap();
            }
            Ok(())
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
            get_show_model,
            set_playback_cursor,
            update_cue,
            add_cue,
            remove_cue,
            move_cue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
