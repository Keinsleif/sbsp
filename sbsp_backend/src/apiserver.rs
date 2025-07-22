use axum::{extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade}, response::IntoResponse, routing::get, Router};
use serde::Serialize;
use tokio::sync::{mpsc, watch};

use crate::{controller::{ControllerCommand, ShowState}, manager::ShowModelManager, model::ShowModel};

#[derive(Clone)]
struct ApiState {
    controller_tx: mpsc::Sender<ControllerCommand>,
    state_rx: watch::Receiver<ShowState>,
    model_manager: ShowModelManager,
}

pub async fn create_api_router(
    controller_tx: mpsc::Sender<ControllerCommand>,
    state_rx: watch::Receiver<ShowState>,
    model_manager: ShowModelManager,
) -> Router {
    let state = ApiState {
        controller_tx,
        state_rx,
        model_manager,
    };

    Router::new()
        // WebSocket接続用のエンドポイント
        .route("/ws", get(websocket_handler))
        // 初回接続時にショー全体の状態を取得するエンドポイント
        .route("/api/show/full_state", get(get_full_state_handler))
        .with_state(state) // ルーター全体で状態を共有
}

#[derive(Serialize)]
struct FullShowState {
    show_model: ShowModel,
    show_state: ShowState,
}

async fn get_full_state_handler(
    State(state): State<ApiState>,
) -> axum::Json<FullShowState> {

    let show_model = state.model_manager.read().await.clone();    
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

    log::info!("New WebSocket client connected.");

    loop {
        tokio::select! {
            Ok(_) = state_rx.changed() => {
                let new_state = state_rx.borrow().clone();
                
                if let Ok(payload) = serde_json::to_string(&new_state) {
                    if socket.send(Message::Text(payload.into())).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
            }
            
            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {
                    if let Ok(command_request) = serde_json::from_str::<ControllerCommand>(&text) {
                        if state.controller_tx.send(command_request).await.is_err() {
                            log::error!("Failed to send Go command to CueController.");
                            break;
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
