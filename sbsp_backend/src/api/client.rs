mod file_list_handler;
mod service_entry;

use std::sync::{Arc, atomic::AtomicBool};

use futures_util::{SinkExt, TryStreamExt};
use mdns_sd::{Error, ServiceDaemon, ServiceEvent};
use tokio::sync::{RwLock, broadcast, mpsc, watch};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::{WsCommand, WsFeedback};
use crate::{
    BackendHandle,
    api::auth::{generate_authentication_string, generate_secret},
    asset_processor::{AssetProcessorCommand, AssetProcessorHandle},
    controller::{ControllerCommand, CueControllerHandle},
    event::UiEvent,
    manager::{ModelCommand, ProjectStatus, ShowModelHandle},
    model::ShowModel,
};
pub use file_list_handler::FileListHandle;
pub use service_entry::ServiceEntry;

type ConnectionHandles = (
    BackendHandle,
    broadcast::Sender<UiEvent>,
    FileListHandle,
    mpsc::Sender<()>,
);

pub async fn create_remote_backend(
    address: String,
    password: Option<String>,
) -> anyhow::Result<ConnectionHandles> {
    let model = Arc::new(RwLock::new(ShowModel::default()));
    let project_status = Arc::new(RwLock::new(ProjectStatus::Unsaved));
    let (event_tx, _) = broadcast::channel::<UiEvent>(32);
    let (model_tx, mut model_rx) = mpsc::channel::<ModelCommand>(32);
    let (controller_tx, mut controller_rx) = mpsc::channel::<ControllerCommand>(32);
    let (asset_tx, mut asset_rx) = mpsc::channel::<AssetProcessorCommand>(32);

    let (asset_list_tx, asset_list_rx) = watch::channel(Vec::new());
    let (asset_list_command_tx, mut asset_list_command_rx) = mpsc::channel(8);
    let asset_list_handle = FileListHandle::new(asset_list_rx, asset_list_command_tx);

    let (request_state_sync_tx, mut request_state_sync_rx) = mpsc::channel(8);

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    let model_clone = model.clone();
    let project_status_clone = project_status.clone();
    let event_tx_clone = event_tx.clone();

    let (mut websocket, _) = connect_async(format!("ws://{}/ws", address)).await?;

    if let Ok(Some(message)) = websocket.try_next().await {
        if let Message::Text(text) = &message
            && let Ok(feedback) = serde_json::from_str::<WsFeedback>(text)
            && let WsFeedback::Hello { auth } = feedback
        {
            let response = if let Some(auth_info) = auth {
                if let Some(pass) = password {
                    let secret = generate_secret(&pass, &auth_info.salt);
                    Some(generate_authentication_string(
                        &secret,
                        &auth_info.challenge,
                    ))
                } else {
                    anyhow::bail!("Password is required to connect.")
                }
            } else {
                None
            };
            if let Ok(payload) = serde_json::to_string(&WsCommand::Authenticate { response })
                && websocket.send(Message::Text(payload.into())).await.is_err()
            {
                log::info!("WebSocket client disconnected (send error).");
                anyhow::bail!("Connection closed during authentication.");
            }
        } else if let Message::Close { .. } = &message {
            log::info!("WebSocket server sent close message.");
            anyhow::bail!("Connection closed during authentication.");
        }
    }

    loop {
        if let Ok(Some(message)) = websocket.try_next().await {
            if let Message::Text(text) = &message
                && let Ok(feedback) = serde_json::from_str::<WsFeedback>(text)
                && let WsFeedback::Authenticated = feedback
            {
                break;
            } else if let Message::Close { .. } = &message {
                log::info!("WebSocket server sent close message.");
                anyhow::bail!("Connection closed during authentication.");
            }
        }
    }

    if let Ok(payload) = serde_json::to_string(&WsCommand::RequestFullShowState)
        && websocket.send(Message::Text(payload.into())).await.is_err()
    {
        anyhow::bail!("WebSocket client disconnected (send error).");
    }

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(Some(message)) = websocket.try_next() => {
                    match message {
                        Message::Text(text) => {
                            if let Ok(ws_message) = serde_json::from_str::<WsFeedback>(&text) {
                                match ws_message {
                                    WsFeedback::Event(ui_event) => {
                                        if let UiEvent::ShowModelLoaded { model, project_type, path } = &*ui_event {
                                            {
                                                let mut model_lock = model_clone.write().await;
                                                *model_lock = model.clone();
                                            }
                                            {
                                                let mut project_status = project_status_clone.write().await;
                                                *project_status = ProjectStatus::Saved{
                                                    project_type: project_type.clone(),
                                                    path: path.clone(),
                                                };
                                            }
                                        } else if let UiEvent::ShowModelSaved {project_type, path} = &*ui_event {
                                            {
                                                let mut project_status = project_status_clone.write().await;
                                                *project_status = ProjectStatus::Saved{
                                                    project_type: project_type.clone(),
                                                    path: path.clone(),
                                                };
                                            }
                                        } else if let UiEvent::ShowModelReset { model } = &*ui_event {
                                            {
                                                let mut model_lock = model_clone.write().await;
                                                *model_lock = model.clone();
                                            }
                                            {
                                                let mut project_status = project_status_clone.write().await;
                                                *project_status = ProjectStatus::Unsaved;
                                            }
                                        }
                                        if event_tx_clone.send(*ui_event).is_err() {
                                            log::error!("Failed to send UiEvent to channel.");
                                            break;
                                        }
                                    },
                                    WsFeedback::AssetList(file_list) => {
                                        if asset_list_tx.send(file_list).is_err() {
                                            log::error!("Failed to send asset list to channel.");
                                        }
                                    }
                                    WsFeedback::FullShowState(full_state) => {
                                        {
                                            let mut show_model = model_clone.write().await;
                                            *show_model = full_state.show_model;
                                        }
                                        {
                                            let mut project_status = project_status_clone.write().await;
                                            *project_status = full_state.project_status;
                                        }
                                    }
                                    WsFeedback::Hello { .. } => {},
                                    WsFeedback::Authenticated => {},
                                }
                            } else {
                                log::error!("Invalid command received.")
                            }
                        }
                        Message::Close{ .. } => {
                            log::info!("WebSocket server sent close message.");
                            break;
                        }
                        Message::Ping(bytes) => {
                            if websocket.send(Message::Pong(bytes)).await.is_err() {
                                log::info!("WebSocket client disconnected (send error).");
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                Some(model_command) = model_rx.recv() => {
                    let api_command = WsCommand::Model(Box::new(model_command));
                    if let Ok(payload) = serde_json::to_string(&api_command)
                    && websocket.send(Message::Text(payload.into())).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
                Some(controller_command) = controller_rx.recv() => {
                    let api_command = WsCommand::Controll(controller_command);
                    if let Ok(payload) = serde_json::to_string(&api_command)
                    && websocket.send(Message::Text(payload.into())).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
                Some(asset_processor_command) = asset_rx.recv() => {
                    let api_command = WsCommand::AssetProcessor(asset_processor_command);
                    if let Ok(payload) = serde_json::to_string(&api_command)
                    && websocket.send(Message::Text(payload.into())).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
                Some(_) = asset_list_command_rx.recv() => {
                    if let Ok(payload) = serde_json::to_string(&WsCommand::RequestAssetList) && websocket.send(Message::Text(payload.into())).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
                Some(_) = request_state_sync_rx.recv() => {
                    if let Ok(payload) = serde_json::to_string(&WsCommand::RequestSyncState) && websocket.send(Message::Text(payload.into())).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
                _ = shutdown_rx.recv() => {
                    if let Err(e) = websocket.send(Message::Close(None)).await {
                        log::warn!("Failed to send Close message to client: {}", e);
                    }
                    break;
                }
            }
        }
    });

    Ok((
        BackendHandle {
            model_handle: ShowModelHandle::new(
                model,
                model_tx,
                project_status,
                Arc::new(AtomicBool::new(false)), // Behave as saved in client
            ),
            asset_processor_handle: AssetProcessorHandle {
                command_tx: asset_tx,
            },
            controller_handle: CueControllerHandle {
                command_tx: controller_tx,
            },
            level_meter: None,
            request_state_sync_tx,
        },
        event_tx,
        asset_list_handle,
        shutdown_tx,
    ))
}

pub fn start_discovery() -> watch::Receiver<Vec<ServiceEntry>> {
    let (services_tx, services_rx) = watch::channel(Vec::new());
    tokio::spawn(async move {
        let service_type = "_sbsp._tcp.local.";
        let mdns = ServiceDaemon::new().expect("Failed to create daemon");
        let receiver = mdns.browse(service_type).expect("Failed to browse");
        loop {
            tokio::select! {
                Ok(event) = receiver.recv_async() => {
                    match event {
                        ServiceEvent::ServiceResolved(resolved) => {
                            let fullname: String = resolved.get_fullname().into();
                            let mut server_name = fullname.replace(service_type, "");
                            server_name.pop();
                            let entry = ServiceEntry {
                                fullname,
                                server_name,
                                host: resolved.get_hostname().into(),
                                port: resolved.get_port(),
                            };
                            services_tx.send_modify(|services| {
                                if !services.contains(&entry) {
                                    services.push(entry);
                                }
                            });
                        }
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            services_tx.send_modify(|services| {
                                services.retain(|sv| sv.fullname != fullname);
                            });
                        }
                        _ => {}
                    }
                }
                _ = services_tx.closed() => break,
            }
        }
        let mut result = mdns.shutdown();
        while let Err(Error::Again) = result {
            result = mdns.shutdown();
        }
    });
    services_rx
}
