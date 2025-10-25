mod command;
#[cfg(desktop)]
pub mod update;

use std::time::SystemTime;

use log::LevelFilter;
use sbsp_backend::{
    BackendHandle, api::server::start_apiserver, controller::state::ShowState, event::UiEvent,
    start_backend,
};
use tauri::{
    AppHandle, Emitter, Manager as _,
    menu::{MenuBuilder, MenuId, MenuItem, SubmenuBuilder},
};
use tauri_plugin_log::fern::colors::{Color, ColoredLevelConfig};
use tokio::sync::{Mutex, RwLock, broadcast, watch};

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
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(LevelFilter::Debug)
                .format(move |out, message, record| {
                    let color_level = ColoredLevelConfig::new()
                        .error(Color::Red)
                        .warn(Color::Yellow)
                        .info(Color::Green)
                        .debug(Color::White)
                        .trace(Color::BrightBlack);
                    out.finish(
                        format_args!(
                            "[{}][{}][{}] {}",
                            humantime::format_rfc3339_seconds(SystemTime::now()),
                            color_level.color(record.level()),
                            record.target(),
                            message
                        ),
                    )
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let app_handle = app.handle();

            #[cfg(desktop)]
            {
                app_handle.plugin(tauri_plugin_updater::Builder::new().build()).unwrap();
                app.manage(update::PendingUpdate::default());
            }

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
            let help_menu = SubmenuBuilder::new(app, "Help")
                .text("id_check_update", "Check for updates")
                .build()?;
            let menu = MenuBuilder::new(app)
                .items(&[&file_menu, &edit_menu, &cue_menu, &tools_menu, &help_menu])
                .build()?;
            app.set_menu(menu)?;

            let (backend_handle, state_rx, event_tx) = start_backend();

            tokio::spawn(forward_backend_state_and_event(
                app_handle.clone(),
                state_rx.clone(),
                event_tx.subscribe(),
            ));

            app.manage(AppState::new(backend_handle, state_rx, event_tx));

            Ok(())
        })
        .on_menu_event(|handle, event| match event.id().as_ref() {
            "id_open" => {
                command::file_open(handle.clone());
            }
            "id_save" => {
                command::file_save(handle.clone());
            }
            "id_save_as" => {
                command::file_save_as(handle.clone());
            }
            "id_quit" => {
                handle.cleanup_before_exit();
                std::process::exit(0);
            }
            "id_delete" | "id_renumber" | "id_audio_cue" | "id_wait_cue" | "id_check_update" => {
                let _ = handle.emit("menu_clicked", event.id());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            command::get_side,
            command::process_asset,
            command::file_open,
            command::file_save,
            command::file_save_as,
            command::add_empty_cue,
            command::controller::go,
            command::controller::pause,
            command::controller::resume,
            command::controller::stop,
            command::controller::pause_all,
            command::controller::resume_all,
            command::controller::stop_all,
            command::controller::load,
            command::controller::seek_to,
            command::controller::seek_by,
            command::controller::set_playback_cursor,
            command::controller::toggle_repeat,
            command::model_manager::get_show_model,
            command::model_manager::update_cue,
            command::model_manager::add_cue,
            command::model_manager::add_cues,
            command::model_manager::remove_cue,
            command::model_manager::move_cue,
            command::model_manager::renumber_cues,
            command::model_manager::update_settings,
            command::server::is_server_running,
            command::server::get_server_port,
            command::server::set_server_port,
            command::server::get_discovery_option,
            command::server::set_discovery_option,
            command::server::start_server,
            command::server::stop_server,
            command::server::open_server_panel,
            #[cfg(desktop)]
            update::fetch_update,
            #[cfg(desktop)]
            update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
