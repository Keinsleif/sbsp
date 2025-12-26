mod command;
mod settings;
#[cfg(desktop)]
pub mod update;

use std::time::{SystemTime, Duration};

use log::LevelFilter;
use sbsp_backend::{
    BackendHandle, api::server::start_apiserver, controller::state::ShowState, event::UiEvent,
    start_backend,
};
use sbsp_license::LicenseManager;
use tauri::{AppHandle, Emitter as _, Manager as _, ipc::Channel};
use tauri_plugin_log::fern::colors::{Color, ColoredLevelConfig};
use tauri_plugin_dialog::{DialogExt as _, MessageDialogKind};
use tokio::{sync::{Mutex, RwLock, broadcast, watch}, time::interval};

use crate::settings::manager::GlobalSettingsManager;

const PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAdlqW6bS6NMn2cdf2b4Ot1DNyjoytP2uFqoH+WlG+NeI=
-----END PUBLIC KEY-----";

pub struct AppState {
    backend_handle: BackendHandle,
    state_rx: watch::Receiver<ShowState>,
    event_tx: broadcast::Sender<UiEvent>,
    pub settings_manager: GlobalSettingsManager,
    discovery_option: RwLock<Option<String>>,
    port: RwLock<u16>,
    shutdown_tx: Mutex<Option<broadcast::Sender<()>>>,
    level_meter_tx: watch::Sender<Option<Channel<(f32, f32)>>>
}

impl AppState {
    pub fn new(
        backend_handle: BackendHandle,
        state_rx: watch::Receiver<ShowState>,
        event_tx: broadcast::Sender<UiEvent>,
        settings_manager: GlobalSettingsManager,
        level_meter_tx: watch::Sender<Option<Channel<(f32, f32)>>>,
    ) -> Self {
        Self {
            backend_handle,
            state_rx,
            event_tx,
            settings_manager,
            discovery_option: RwLock::new(None),
            port: RwLock::new(5800),
            shutdown_tx: Mutex::new(None),
            level_meter_tx,
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
    state_rx: watch::Receiver<ShowState>,
    mut event_rx: broadcast::Receiver<UiEvent>,
) {
    let mut poll_timer = interval(Duration::from_millis(32));
    loop {
        tokio::select! {
            _ = poll_timer.tick() => {
                if let Ok(changed) = state_rx.has_changed() && changed {
                    let state = state_rx.borrow().clone();
                    app_handle.emit("backend-state-update", state).ok();
                }
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
        .plugin(tauri_plugin_os::init())
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
                    out.finish(format_args!(
                        "[{}][{}][{}] {}",
                        humantime::format_rfc3339_seconds(SystemTime::now()),
                        color_level.color(record.level()),
                        record.target(),
                        message
                    ))
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let app_handle = app.handle();

            #[cfg(desktop)]
            {
                app_handle
                    .plugin(tauri_plugin_updater::Builder::new().build())
                    .unwrap();
                app.manage(update::PendingUpdate::default());
            }

            let (settings_manager, settings_rx) = GlobalSettingsManager::new();

            let (backend_handle, state_rx, event_tx) = match start_backend(settings_rx, true) {
                Ok(backends) => backends,
                Err(e) => {
                    app.dialog()
                        .message(e.to_string())
                        .kind(MessageDialogKind::Error)
                        .title("Failed to start backend")
                        .blocking_show();
                    return Err(e.into());
                }
            };
            let (level_meter_tx, level_meter_rx) = watch::channel::<Option<Channel<(f32,f32)>>>(None);

            tokio::spawn(forward_backend_state_and_event(
                app_handle.clone(),
                state_rx.clone(),
                event_tx.subscribe(),
            ));

            app.manage(AppState::new(
                backend_handle,
                state_rx,
                event_tx,
                settings_manager,
                level_meter_tx,
            ));

            app.manage(LicenseManager::new_from_pem(PUBLIC_KEY_PEM));

            if let Ok(path) = app.path().app_config_dir() {
                let config_path = path.join("config.json");
                let app_handle_clone = app_handle.clone();
                tokio::spawn(async move {
                    let state = app_handle_clone.state::<AppState>();
                    if let Err(e) = state
                        .settings_manager
                        .load_from_file(config_path.as_path())
                        .await
                    {
                        log::error!(
                            "Failed to load config on startup. file={:?}, error={}",
                            config_path,
                            e
                        );
                    }
                });
                let license_path = path.join("license.json");
                if license_path.exists() {
                    let license_manager = app_handle.state::<LicenseManager>();
                    let _ = license_manager.activate_by_file(license_path);
                }
            }

            let app_handle_clone = app_handle.clone();
            tokio::spawn(async move {
                let mut ticker = interval(Duration::from_millis(33)); // about 30fps
                if let Some(shared_level) = app_handle_clone.state::<AppState>().get_handle().level_meter {
                    loop {
                        ticker.tick().await;
                        if let Some(level_meter) = level_meter_rx.borrow().as_ref() {
                            let (l, r) = shared_level.get();
                            if l > 0.001 && r > 0.001 {
                                level_meter.send((l, r)).ok();
                            }
                        }
                    }
                } else {
                    log::warn!("Level meter is not available.");
                }
            });

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            command::get_side,
            command::get_third_party_notices,
            command::process_asset,
            command::file_new,
            command::file_open,
            command::file_save,
            command::file_save_as,
            command::export_to_folder,
            command::pick_audio_assets,
            command::listen_level_meter,
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
            command::controller::set_volume,
            command::model_manager::get_show_model,
            command::model_manager::is_modified,
            command::model_manager::update_cue,
            command::model_manager::add_cue,
            command::model_manager::add_cues,
            command::model_manager::remove_cue,
            command::model_manager::move_cue,
            command::model_manager::renumber_cues,
            command::model_manager::update_model_name,
            command::model_manager::update_show_settings,
            command::server::is_server_running,
            command::server::get_server_port,
            command::server::set_server_port,
            command::server::get_discovery_option,
            command::server::set_discovery_option,
            command::server::start_server,
            command::server::stop_server,
            command::server::open_server_panel,
            command::settings::get_settings,
            command::settings::set_settings,
            command::settings::reload_settings,
            command::settings::save_settings,
            command::settings::import_settings_from_file,
            command::settings::export_settings_to_file,
            command::license::activate_license,
            command::license::get_license_info,
            #[cfg(desktop)]
            update::fetch_update,
            #[cfg(desktop)]
            update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
