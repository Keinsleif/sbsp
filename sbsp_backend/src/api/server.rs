use std::{collections::HashMap, path::PathBuf, time::Duration};

use async_recursion::async_recursion;
use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use gethostname::gethostname;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tokio::{
    sync::{broadcast, watch},
    time::interval,
};

use super::{FullShowState, WsCommand, WsFeedback};
use crate::{
    BackendHandle,
    api::{
        ApiServerOptions, AuthInfo, FileList,
        auth::{check_authentication_string, generate_salt, generate_secret},
    },
    asset_processor::AssetProcessorCommand,
    controller::state::ShowState,
    event::UiEvent,
    manager::ProjectStatus,
    model::ProjectType,
};

#[derive(Clone)]
struct ApiState {
    backend_handle: BackendHandle,
    state_rx: watch::Receiver<ShowState>,
    event_rx_factory: broadcast::Sender<UiEvent>,
    shutdown_tx: broadcast::Sender<()>,
    options: ApiServerOptions,
    salt: String,
}

pub async fn start_apiserver_with<F>(
    backend_handle: BackendHandle,
    state_rx: watch::Receiver<ShowState>,
    event_tx: broadcast::Sender<UiEvent>,
    options: ApiServerOptions,
    router_extender: F,
) -> anyhow::Result<broadcast::Sender<()>>
where
    F: FnOnce(Router) -> Router + Send + 'static,
{
    log::info!(
        "Starting server with port: {}, discovery: {:?}",
        &options.port,
        &options.discoverry
    );
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
    let salt = generate_salt();

    let state = ApiState {
        backend_handle,
        state_rx,
        event_rx_factory: event_tx,
        shutdown_tx: shutdown_tx.clone(),
        options: options.clone(),
        salt,
    };

    let mut app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state);

    app = router_extender(app);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", options.port)).await?;
    log::info!("ApiServer listening on 0.0.0.0:{}", options.port);

    if let Some(server_name) = options.discoverry {
        let mut shutdown_rx_clone = shutdown_tx.subscribe();
        let hostname = get_mdns_hostname()?;
        let mdns = ServiceDaemon::new()?;
        let properties: HashMap<String, String> = HashMap::new();
        let sv_info = ServiceInfo::new(
            "_sbsp._tcp.local.",
            &server_name,
            &hostname,
            "",
            options.port,
            properties,
        )
        .unwrap()
        .enable_addr_auto();
        mdns.register(sv_info).unwrap();

        tokio::spawn(async move {
            let _ = shutdown_rx_clone.recv().await;
            let mut result = mdns.shutdown();
            while let Err(mdns_sd::Error::Again) = result {
                result = mdns.shutdown();
            }
        });
    }
    // let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.recv().await;
            })
            .into_future(),
    );
    Ok(shutdown_tx)
}

pub async fn start_apiserver(
    backend_handle: BackendHandle,
    state_rx: watch::Receiver<ShowState>,
    event_tx: broadcast::Sender<UiEvent>,
    options: ApiServerOptions,
) -> anyhow::Result<broadcast::Sender<()>> {
    start_apiserver_with(backend_handle, state_rx, event_tx, options, |app| app).await
}

pub fn get_mdns_hostname() -> anyhow::Result<String> {
    gethostname()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("failed to get hostname."))
        .map(|hostname| format!("{}.local.", hostname))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: ApiState) {
    let challenge = generate_salt();
    let auth_info = if state.options.password.is_some() {
        Some(AuthInfo {
            challenge: challenge.clone(),
            salt: state.salt.clone(),
        })
    } else {
        None
    };

    if let Ok(payload) = serde_json::to_string(&WsFeedback::Hello {
        auth: auth_info.clone(),
    }) && socket.send(Message::Text(payload.into())).await.is_err()
    {
        log::info!("WebSocket client disconnected (send error).");
        return;
    }

    loop {
        if let Some(Ok(msg)) = socket.recv().await {
            if let Message::Text(text) = msg {
                if let Ok(command) = serde_json::from_str::<WsCommand>(&text)
                    && let WsCommand::Authenticate { response } = command
                {
                    if let Some(password) = &state.options.password {
                        let secret = generate_secret(password, &state.salt);
                        if let Some(auth_str) = response
                            && check_authentication_string(&secret, &challenge, &auth_str)
                        {
                            break;
                        } else {
                            if let Err(e) = socket.send(Message::Close(None)).await {
                                log::error!("Error on closing socket. e={}", e);
                                return;
                            }
                            log::debug!("Close message sent.");
                            return;
                        }
                    } else {
                        break;
                    }
                } else {
                    continue;
                }
            } else if let Message::Close(_) = msg {
                log::info!("WebSocket client sent close message.");
                return;
            }
        }
    }

    if let Ok(payload) = serde_json::to_string(&WsFeedback::Authenticated)
        && socket.send(Message::Text(payload.into())).await.is_err()
    {
        log::info!("WebSocket client disconnected (send error).");
        return;
    }

    let state_rx = state.state_rx.clone();
    let mut event_rx = state.event_rx_factory.subscribe();

    let mut poll_timer = interval(Duration::from_millis(32));

    log::info!("New WebSocket client connected.");

    let mut shutdown_rx = state.shutdown_tx.subscribe();

    loop {
        tokio::select! {
            Ok(event) = event_rx.recv() => {
                let ws_message = WsFeedback::Event(Box::new(event));

                if let Ok(payload) = serde_json::to_string(&ws_message)
                    && socket.send(Message::Text(payload.into())).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
            }
            _ = poll_timer.tick() => {
                if let Ok(changed) = state_rx.has_changed() && changed {
                    let new_state = state_rx.borrow().clone();
                    let ws_message = WsFeedback::State(new_state);

                    if let Ok(payload) = serde_json::to_string(&ws_message)
                        && socket.send(Message::Text(payload.into())).await.is_err() {
                            log::info!("WebSocket client disconnected (send error).");
                            break;
                        }
                }
            }

            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {
                    if let Ok(command_request) = serde_json::from_str::<WsCommand>(&text) {
                        match command_request {
                            WsCommand::Controll(controller_command) => {
                                if state.backend_handle.controller_handle.send_command(controller_command).await.is_err() {
                                    log::error!("Failed to send Go command to CueController.");
                                    break;
                                }
                            },
                            WsCommand::Model(model_command) => {
                                if state.backend_handle.model_handle.send_command(*model_command).await.is_err() {
                                    log::error!("Failed to send Model command to ShowModelManager.");
                                    break;
                                }
                            },
                            WsCommand::AssetProcessor(asset_processor_command) => {
                                match asset_processor_command {
                                    AssetProcessorCommand::RequestFileAssetData { path } => {
                                        state.backend_handle.asset_processor_handle.request_file_asset_data(path.clone()).await;
                                    },
                                }
                            },
                            WsCommand::RequestAssetList => {
                                log::info!("Asset List reqested.");
                                if let ProjectStatus::Saved{ project_type, path } = state.backend_handle.model_handle.get_project_state().await.clone()
                                && project_type == ProjectType::ProjectFolder
                                && let Some(parent) = path.parent()
                                && let Ok(file_list) = get_dirs(parent.to_path_buf(), None).await {
                                    let ws_message = WsFeedback::AssetList(file_list);
                                    if let Ok(payload) = serde_json::to_string(&ws_message) && socket.send(Message::Text(payload.into())).await.is_err() {
                                        log::info!("WebSocket client disconnected (send error).");
                                        break;
                                    }
                                }
                            }
                            WsCommand::RequestFullShowState => {
                                let project_status = state.backend_handle.model_handle.get_project_state().await.clone();
                                let show_model = state.backend_handle.model_handle.read().await.clone();
                                let show_state = state.state_rx.borrow().clone();

                                let full_state = FullShowState {
                                    project_status,
                                    show_model,
                                    show_state,
                                };

                                let ws_message = WsFeedback::FullShowState(full_state);
                                if let Ok(payload) = serde_json::to_string(&ws_message) && socket.send(Message::Text(payload.into())).await.is_err() {
                                    log::info!("WebSocket client disconnected (send error).");
                                    break;
                                }
                            }
                            _ => {}
                        }
                    } else {
                        log::error!("Invalid command received.")
                    }
                } else if let Message::Close(_) = msg {
                    log::info!("WebSocket client sent close message.");
                    break;
                }
            }
            _ = shutdown_rx.recv() => {
                if let Err(e) = socket.send(Message::Close(None)).await {
                    log::warn!("Failed to send Close message to client: {}", e);
                }
                break;
            }
            else => {
                if let Err(e) = socket.send(Message::Close(None)).await {
                    log::warn!("Failed to send Close message to client: {}", e);
                }
                break;
            },
        }
    }
}

#[async_recursion]
async fn get_dirs(root_dir: PathBuf, parent: Option<PathBuf>) -> anyhow::Result<Vec<FileList>> {
    let mut entries = tokio::fs::read_dir(root_dir).await?;
    let mut root_list = vec![];
    let parent_dir = parent.unwrap_or(PathBuf::from("."));
    loop {
        let entry_option = entries.next_entry().await?;
        if let Some(entry) = entry_option {
            let metadata = entry.metadata().await?;
            let path = entry.path();

            let entry_name = path
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap();
            if metadata.is_dir() {
                let file_list = get_dirs(path, Some(parent_dir.join(&entry_name))).await?;
                root_list.push(FileList::Dir {
                    name: entry_name,
                    files: file_list,
                });
                continue;
            }
            if metadata.is_file() {
                let extension = if let Some(ext) = path.extension() {
                    ext.to_os_string().into_string().unwrap()
                } else {
                    "".into()
                };
                root_list.push(FileList::File {
                    name: entry_name.clone(),
                    path: parent_dir.join(&entry_name),
                    extension,
                });
                continue;
            }

            if let Ok(symlink) = tokio::fs::read_link(path).await {
                if symlink.is_dir() {
                    let file_list = get_dirs(symlink, Some(parent_dir.join(&entry_name))).await?;
                    root_list.push(FileList::Dir {
                        name: entry_name,
                        files: file_list,
                    });
                } else {
                    let extension = if let Some(ext) = symlink.extension() {
                        ext.to_os_string().into_string().unwrap()
                    } else {
                        "".into()
                    };
                    let file_name = symlink
                        .file_name()
                        .unwrap()
                        .to_os_string()
                        .into_string()
                        .unwrap();
                    root_list.push(FileList::File {
                        name: file_name,
                        path: parent_dir.join(&entry_name),
                        extension,
                    });
                }
            }
        } else {
            break;
        }
    }
    Ok(root_list)
}
