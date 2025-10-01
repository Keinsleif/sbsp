mod command;

use log::LevelFilter;
use sbsp_backend::{
    BackendHandle, api::server::start_apiserver, controller::state::ShowState, event::UiEvent,
    start_backend,
};
use tauri::{
    AppHandle, Emitter, Manager as _,
    menu::{MenuBuilder, MenuId, MenuItem, SubmenuBuilder},
};
use tokio::sync::{Mutex, RwLock, broadcast, watch};

use crate::command::{
    add_empty_cue,
    controller::{
        go, load, pause, pause_all, resume, resume_all, seek_by, seek_to, set_playback_cursor,
        stop, stop_all, toggle_repeat,
    },
    file_open, file_save, file_save_as, get_side,
    model_manager::{
        add_cue, add_cues, get_show_model, move_cue, remove_cue, renumber_cues, update_cue,
        update_settings,
    },
    process_asset,
    server::{
        get_discovery_option, get_server_port, is_server_running, open_server_panel,
        set_discovery_option, set_server_port, start_server, stop_server,
    },
};

pub struct AppState {
    backend_handle: BackendHandle,
    state_rx: watch::Receiver<ShowState>,
    event_tx: broadcast::Sender<UiEvent>,
    discovery_option: RwLock<Option<String>>,
    port: RwLock<u16>,
    shutdown_tx: Mutex<Option<broadcast::Sender<()>>>,
}

impl AppState {
    pub fn new(
        backend_handle: BackendHandle,
        state_rx: watch::Receiver<ShowState>,
        event_tx: broadcast::Sender<UiEvent>,
    ) -> Self {
        Self {
            backend_handle,
            state_rx,
            event_tx,
            discovery_option: RwLock::new(None),
            port: RwLock::new(5800),
            shutdown_tx: Mutex::new(None),
        }
    }

    pub fn get_handle(&self) -> BackendHandle {
        self.backend_handle.clone()
    }

    pub async fn is_running(&self) -> bool {
        self.shutdown_tx.lock().await.is_some()
    }

    pub async fn is_discoverable(&self) -> bool {
        self.discovery_option.read().await.is_some()
    }

    pub async fn set_discovery_option(&self, discovery_option: Option<String>) {
        let mut name_lock = self.discovery_option.write().await;
        *name_lock = discovery_option;
        drop(name_lock)
    }

    pub async fn get_discovery_option(&self) -> Option<String> {
        self.discovery_option.read().await.clone()
    }

    pub async fn set_port(&self, port: u16) {
        let mut port_write_lock = self.port.write().await;
        *port_write_lock = port;
        drop(port_write_lock);
    }

    pub async fn get_port(&self) -> u16 {
        *self.port.read().await
    }

    pub async fn start(&self, app_handle: AppHandle) -> anyhow::Result<()> {
        let port_read_lock = self.port.read().await;
        let name_lock = self.discovery_option.read().await;
        let shutdown_tx = start_apiserver(
            *port_read_lock,
            self.backend_handle.clone(),
            self.state_rx.clone(),
            self.event_tx.clone(),
            name_lock.clone(),
        )
        .await?;
        drop(port_read_lock);
        let mut shutdown_tx_lock = self.shutdown_tx.lock().await;
        *shutdown_tx_lock = Some(shutdown_tx);
        drop(shutdown_tx_lock);
        let _ = app_handle.emit("backend-server-status-changed", "started");
        Ok(())
    }

    pub async fn stop(&self, app_handle: AppHandle) {
        let mut shutdown_tx_lock = self.shutdown_tx.lock().await;
        if let Some(shutdown_tx) = &(*shutdown_tx_lock) {
            let _ = shutdown_tx.send(());
        }
        *shutdown_tx_lock = None;
        let _ = app_handle.emit("backend-server-status-changed", "stopped");
        drop(shutdown_tx_lock);
    }
}

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
            let app_handle = app.handle();

            let file_menu = SubmenuBuilder::new(app, "File")
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

            let (backend_handle, state_rx, event_tx) = start_backend();

            tauri::async_runtime::spawn(forward_backend_state_and_event(
                app_handle.clone(),
                state_rx.clone(),
                event_tx.subscribe(),
            ));

            app.manage(AppState::new(backend_handle, state_rx, event_tx));

            Ok(())
        })
        .on_menu_event(|handle, event| match event.id().as_ref() {
            "id_open" => {
                file_open(handle.clone());
            }
            "id_save" => {
                file_save(handle.clone());
            }
            "id_save_as" => {
                file_save_as(handle.clone());
            }
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
            is_server_running,
            get_server_port,
            set_server_port,
            get_discovery_option,
            set_discovery_option,
            start_server,
            stop_server,
            open_server_panel,
            process_asset,
            file_open,
            file_save,
            file_save_as,
            add_empty_cue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
