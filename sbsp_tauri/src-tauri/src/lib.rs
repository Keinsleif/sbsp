use sbsp_backend::{controller::{ControllerCommand, state::ShowState}, event::UiEvent, model::ShowModel, start_backend, BackendHandle};
use tauri::{AppHandle, Emitter, LogicalSize, Manager as _, Size};
use tokio::sync::{broadcast, watch};

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
    handle.controller_tx.send(ControllerCommand::Go).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_show_model(handle: tauri::State<'_, BackendHandle>) -> Result<ShowModel, String> {
    Ok(handle.model_handle.read().await.clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {

            let (backend_handle, state_rx, event_tx) = start_backend();

            tauri::async_runtime::spawn(forward_backend_state_and_event(app.handle().clone(), state_rx, event_tx.subscribe()));

            app.manage(backend_handle);

            let main_window = app.get_webview_window("main").unwrap();
            main_window.set_size(Size::Logical(LogicalSize{ width: 1280.0, height: 720.0})).unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![go, get_show_model])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
