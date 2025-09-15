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
use tokio::{runtime::Handle, sync::{broadcast, oneshot, watch}};
use libmdns::Responder;

use crate::{
    asset_processor::{AssetProcessorCommand, ProcessResult}, controller::state::ShowState, event::UiEvent, BackendHandle
};
use super::{
    WsCommand, WsFeedback, FullShowState
};

#[derive(Clone)]
struct ApiState {
    backend_handle: BackendHandle,
    state_rx: watch::Receiver<ShowState>,
    event_rx_factory: broadcast::Sender<UiEvent>,
}

pub async fn run(port: usize, backend_handle: BackendHandle, state_rx: watch::Receiver<ShowState>, event_tx: broadcast::Sender<UiEvent>, discoverable: bool) -> anyhow::Result<()> {
    let app = create_api_router(
        backend_handle,
        state_rx,
        event_tx,
    )
    .await;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    log::info!("ApiServer listening on 0.0.0.0:{}", port);

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    if discoverable {
        tokio::task::spawn_blocking(move || {
            let handle = Handle::current();
            let responder = Responder::spawn(&handle).unwrap();
            let hostname = gethostname();
            let svc = responder.register("_sbsp._tcp", hostname.to_str().unwrap_or("SBSP_Server"), port as u16, &[]);
            let _ = shutdown_rx.blocking_recv();
            std::mem::drop(svc);
        });
        tokio::spawn(async move {
            shutdown_signal().await;
            let _ = shutdown_tx.send(0);
        });
    }

    axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

pub async fn create_api_router(
    backend_handle: BackendHandle,
    state_rx: watch::Receiver<ShowState>,
    event_rx_factory: broadcast::Sender<UiEvent>,
) -> Router {
    let state = ApiState {
        backend_handle,
        state_rx,
        event_rx_factory,
    };

    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/show/full_state", get(get_full_state_handler))
        .with_state(state)
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
                                    AssetProcessorCommand::ProcessAll => todo!(),
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

            else => break,
        }
    }
}
