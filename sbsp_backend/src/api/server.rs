use std::{collections::HashMap, path::PathBuf};

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
use tokio::{sync::{broadcast, watch}};

use crate::{
    api::FileList, asset_processor::{AssetProcessorCommand, ProcessResult}, controller::state::ShowState, event::UiEvent, BackendHandle
};
use super::{
    WsCommand, WsFeedback, FullShowState
};

#[derive(Clone)]
struct ApiState {
    backend_handle: BackendHandle,
    state_rx: watch::Receiver<ShowState>,
    event_rx_factory: broadcast::Sender<UiEvent>,
    shutdown_tx: broadcast::Sender<()>,
}

pub async fn start_apiserver(port: u16, backend_handle: BackendHandle, state_rx: watch::Receiver<ShowState>, event_tx: broadcast::Sender<UiEvent>, discover_option: Option<String>) -> anyhow::Result<broadcast::Sender<()>> {
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

    let state = ApiState {
        backend_handle,
        state_rx,
        event_rx_factory: event_tx,
        shutdown_tx: shutdown_tx.clone(),
    };

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/show/full_state", get(get_full_state_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    log::info!("ApiServer listening on 0.0.0.0:{}", port);

    if let Some(server_name) = discover_option {
        let mut shutdown_rx_clone = shutdown_tx.subscribe();
        tokio::spawn(async move {
            let hostname = format!("{}.local.", gethostname().to_str().unwrap_or("NoHostname"));
            let mdns = ServiceDaemon::new().expect("Could not create service daemon");

            let properties: HashMap<String, String> = HashMap::new();

            let sv_info = ServiceInfo::new("_sbsp._tcp.local.", &server_name, &hostname, "", port, properties).unwrap().enable_addr_auto();
            let sv_fullname = sv_info.get_fullname().to_string();
            mdns.register(sv_info).unwrap();

            let _ = shutdown_rx_clone.recv().await;

            let result = mdns.unregister(&sv_fullname).unwrap();
            let _ = result.recv();
        });
    }
    // let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
        })
        .into_future()
    );
    Ok(shutdown_tx)
}

async fn get_full_state_handler(State(state): State<ApiState>) -> axum::Json<FullShowState> {
    let show_model = state.backend_handle.model_handle.read().await.clone();
    let show_state = state.state_rx.borrow().clone();

    let full_state = FullShowState {
        show_model,
        show_state,
    };

    axum::Json(full_state)
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: ApiState) {
    let mut state_rx = state.state_rx.clone();
    let mut event_rx = state.event_rx_factory.subscribe();

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
            Ok(_) = state_rx.changed() => {
                let new_state = state_rx.borrow().clone();
                let ws_message = WsFeedback::State(new_state);

                if let Ok(payload) = serde_json::to_string(&ws_message)
                    && socket.send(Message::Text(payload.into())).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
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
                                    AssetProcessorCommand::RequestFileAssetData { id, path } => {
                                        let result = state.backend_handle.asset_processor_handle.request_file_asset_data(path.clone()).await;
                                        let ws_message = WsFeedback::AssetProcessorResult(ProcessResult {
                                            id,
                                            path,
                                            data: result,
                                        });
                                        if let Ok(payload) = serde_json::to_string(&ws_message)
                                        && socket.send(Message::Text(payload.into())).await.is_err() {
                                            log::info!("WebSocket client disconnected (send error).");
                                            break;
                                        }
                                    },
                                    AssetProcessorCommand::ProcessAll => {
                                        state.backend_handle.asset_processor_handle.request_process_all().await;
                                    },
                                }
                            },
                            WsCommand::RequestAssetList => {
                                log::info!("Asset List reqested.");
                                if let Some(model_dir) = state.backend_handle.model_handle.get_current_file_path().await {
                                    let asset_dir = model_dir.join("audio");
                                    if let Ok(file_list) = get_dirs(asset_dir, None).await
                                        && let Ok(payload) = serde_json::to_string(&file_list) && socket.send(Message::Text(payload.into())).await.is_err() {
                                            log::info!("WebSocket client disconnected (send error).");
                                            break;
                                        }
                                }
                            }
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
async fn get_dirs(root_dir: PathBuf, parent: Option<PathBuf>) -> anyhow::Result<Vec<Box<FileList>>> {
    let mut entries = tokio::fs::read_dir(root_dir).await?;
    let mut root_list = vec![];
    let parent_dir = parent.unwrap_or(PathBuf::from("."));
    loop {
        let entry_option = entries.next_entry().await?;
        if let Some(entry) = entry_option {
            let metadata = entry.metadata().await?;
            let path = entry.path();

            let entry_name = path.file_name().unwrap().to_os_string().into_string().unwrap();
            if metadata.is_dir() {
                let file_list = get_dirs(path, Some(parent_dir.join(&entry_name))).await?;
                root_list.push(Box::new(FileList::Dir { name: entry_name, files: file_list }));
                continue;
            }
            if metadata.is_file() {
                let extension = if let Some(ext) = path.extension() {
                    ext.to_os_string().into_string().unwrap()
                } else {
                    "".into()
                };
                root_list.push(Box::new(FileList::File { name: entry_name.clone(), path: parent_dir.join(&entry_name) , extension}));
                continue;
            }

            if let Ok(symlink) = tokio::fs::read_link(path).await {
                if symlink.is_dir() {
                    let file_list = get_dirs(symlink, Some(parent_dir.join(&entry_name))).await?;
                    root_list.push(Box::new(FileList::Dir { name: entry_name, files: file_list }));
                } else {
                    let extension = if let Some(ext) = symlink.extension() {
                        ext.to_os_string().into_string().unwrap()
                    } else {
                        "".into()
                    };
                    let file_name = symlink.file_name().unwrap().to_os_string().into_string().unwrap();
                    root_list.push(Box::new(FileList::File { name: file_name, path: parent_dir.join(&entry_name), extension }));
                }
            }
        } else {
            break;
        }
    }
    Ok(root_list)
}