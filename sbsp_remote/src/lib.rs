mod command;

use log::LevelFilter;
use sbsp_backend::{
    BackendHandle,
    api::client::{FileListHandle, create_remote_backend, start_discovery},
    controller::state::ShowState,
    event::UiEvent,
};
use tauri::{
    AppHandle, Emitter, Manager as _,
    menu::{MenuBuilder, MenuId, MenuItem, SubmenuBuilder},
};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc, watch};

use crate::command::{
    add_empty_cue,
    client::{
        connect_to_server, disconnect_from_server, get_server_address, request_file_list,
        start_server_discovery, stop_server_discovery,
    },
    controller::{
        go, load, pause, pause_all, resume, resume_all, seek_by, seek_to, set_playback_cursor,
        stop, stop_all, toggle_repeat,
    },
    get_side,
    model_manager::{
        add_cue, add_cues, get_show_model, move_cue, remove_cue, renumber_cues, update_cue,
        update_settings,
    },
    process_asset,
};

async fn forward_backend_state_and_event(
    app_handle: AppHandle,
    mut state_rx: watch::Receiver<ShowState>,
    mut event_rx: broadcast::Receiver<UiEvent>,
    mut asset_list_handle: FileListHandle,
) {
    loop {
        tokio::select! {
            Ok(_) = state_rx.changed() => {
                let state = state_rx.borrow().clone();
                app_handle.emit("backend-state-update", state).ok();
            },
            Ok(event) = event_rx.recv() => {
                app_handle.emit("backend-event", event).ok();
            },
            Ok(list) = asset_list_handle.recv_file_list() => {
                app_handle.emit("asset-list-update", list).ok();
            }
            else => break,
        }
    }
}

pub struct ConnectionData {
    backend_handle: BackendHandle,
    address: String,
    asset_list_handle: FileListHandle,
    disconnect_tx: mpsc::Sender<()>,
}

#[derive(Default)]
pub struct AppState {
    connection_data: RwLock<Option<ConnectionData>>,
    discovery_stop_tx: Mutex<Option<mpsc::Sender<()>>>,
}

impl AppState {
    pub async fn get_handle(&self) -> Option<BackendHandle> {
        self.connection_data
            .read()
            .await
            .as_ref()
            .map(|connection_data| connection_data.backend_handle.clone())
    }

    pub async fn get_address(&self) -> Option<String> {
        self.connection_data
            .read()
            .await
            .as_ref()
            .map(|connection_data| connection_data.address.clone())
    }

    pub async fn connect(&self, address: String, app_handle: AppHandle) -> anyhow::Result<()> {
        let (remote_handle, state_rx, event_tx, asset_list_handle, shutdown_tx) =
            create_remote_backend(address.clone()).await?;
        let mut connection_data_lock = self.connection_data.write().await;
        *connection_data_lock = Some(ConnectionData {
            backend_handle: remote_handle,
            address,
            asset_list_handle: asset_list_handle.clone(),
            disconnect_tx: shutdown_tx.clone(),
        });
        drop(connection_data_lock);

        tokio::spawn(forward_backend_state_and_event(
            app_handle.clone(),
            state_rx,
            event_tx.subscribe(),
            asset_list_handle,
        ));

        tokio::spawn(async move {
            shutdown_tx.closed().await;
            let state = app_handle.state::<AppState>();
            state.disconnect_cleanup().await;
            tauri::WebviewWindowBuilder::from_config(
                &app_handle,
                &app_handle.config().app.windows[1],
            )
            .unwrap()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();
            if let Some(main_window) = app_handle.get_webview_window("main") {
                let _ = main_window.close();
            }
        });
        Ok(())
    }

    pub async fn disconnect(&self) {
        if let Some(conn_data) = self.connection_data.read().await.as_ref() {
            let _ = conn_data.disconnect_tx.send(()).await;
        }
    }

    pub async fn disconnect_cleanup(&self) {
        let mut connection_data_lock = self.connection_data.write().await;
        *connection_data_lock = None;
    }

    pub async fn request_file_list(&self) -> anyhow::Result<()> {
        if let Some(conn_data) = self.connection_data.read().await.as_ref() {
            conn_data.asset_list_handle.request_file_list().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected."))
        }
    }

    pub async fn start_discovery(&self, app_handle: AppHandle) {
        let mut watch_rx = start_discovery();
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(_) = watch_rx.changed() => {
                        let services = watch_rx.borrow().clone();
                        app_handle.emit("remote-discovery", services).ok();
                    },
                    _ = shutdown_rx.recv() => {
                        drop(watch_rx);
                        break;
                    },
                }
            }
        });
        *(self.discovery_stop_tx.lock().await) = Some(shutdown_tx);
    }

    pub async fn stop_discovery(&self) {
        if let Some(stop_tx) = self.discovery_stop_tx.lock().await.as_ref() {
            let _ = stop_tx.send(()).await;
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_denylist(&["settings"])
                .build(),
        )
        .setup(|app| {
            let file_menu = SubmenuBuilder::new(app, "File")
                .separator()
                .text(MenuId::new("id_quit"), "Quit")
                .build()?;
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .cut()
                .copy()
                .paste()
                .item(&MenuItem::with_id(
                    app,
                    MenuId::new("id_delete"),
                    "Delete",
                    true,
                    Some("Ctrl+Backspace"),
                )?)
                .select_all()
                .build()?;
            let cue_menu = SubmenuBuilder::new(app, "Cue")
                .text("id_audio_cue", "Audio Cue")
                .text("id_wait_cue", "Wait Cue")
                .build()?;
            let tools_menu = SubmenuBuilder::new(app, "Tools")
                .item(&MenuItem::with_id(
                    app,
                    MenuId::new("id_renumber"),
                    "Renumber selected cues",
                    true,
                    Some("Ctrl+R"),
                )?)
                .build()?;
            let menu = MenuBuilder::new(app)
                .items(&[&file_menu, &edit_menu, &cue_menu, &tools_menu])
                .build()?;
            app.set_menu(menu)?;

            app.manage(AppState::default());

            Ok(())
        })
        .on_menu_event(|handle, event| match event.id().as_ref() {
            "id_quit" => {
                handle.cleanup_before_exit();
                std::process::exit(0);
            }
            "id_delete" | "id_renumber" | "id_audio_cue" | "id_wait_cue" => {
                let _ = handle.emit("menu_clicked", event.id());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_side,
            go,
            pause,
            resume,
            stop,
            pause_all,
            resume_all,
            stop_all,
            load,
            seek_to,
            seek_by,
            get_show_model,
            set_playback_cursor,
            toggle_repeat,
            update_cue,
            add_cue,
            add_cues,
            remove_cue,
            move_cue,
            renumber_cues,
            update_settings,
            get_server_address,
            connect_to_server,
            disconnect_from_server,
            start_server_discovery,
            stop_server_discovery,
            request_file_list,
            process_asset,
            add_empty_cue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
