mod command;
#[cfg(desktop)]
pub mod update;

use std::time::SystemTime;

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
use tauri_plugin_log::fern::colors::{Color, ColoredLevelConfig};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc, watch};

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
            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_updater::Builder::new().build()).unwrap();
                app.manage(update::PendingUpdate::default());
            }

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
            let help_menu = SubmenuBuilder::new(app, "Help")
                .text("id_check_update", "Check for updates")
                .build()?;
            let menu = MenuBuilder::new(app)
                .items(&[&file_menu, &edit_menu, &cue_menu, &tools_menu, &help_menu])
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
            "id_delete" | "id_renumber" | "id_audio_cue" | "id_wait_cue" | "id_check_update" => {
                let _ = handle.emit("menu_clicked", event.id());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            command::get_side,
            command::process_asset,
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
            command::client::get_server_address,
            command::client::connect_to_server,
            command::client::disconnect_from_server,
            command::client::start_server_discovery,
            command::client::stop_server_discovery,
            command::client::request_file_list,
            #[cfg(desktop)]
            update::fetch_update,
            #[cfg(desktop)]
            update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
