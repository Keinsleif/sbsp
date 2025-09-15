use std::sync::Arc;

use futures_util::{SinkExt, TryStreamExt};
use reqwest::Client;
use reqwest_websocket::{Message, RequestBuilderExt};
use tokio::sync::{broadcast, mpsc, watch, RwLock};

use crate::{asset_processor::{AssetProcessorCommand, AssetProcessorHandle}, controller::{state::ShowState, ControllerCommand, CueControllerHandle}, event::UiEvent, manager::{ModelCommand, ShowModelHandle}, model::ShowModel, BackendHandle};
use super::{
    WsCommand, WsFeedback, FullShowState
};

pub fn create_remote_backend(host: &str, port: u16) -> anyhow::Result<(
    BackendHandle,
    watch::Receiver<ShowState>,
    broadcast::Sender<UiEvent>,
)> {
    let host = format!("{}:{}", host, port);

    let model = Arc::new(RwLock::new(ShowModel::default()));
    let show_model_path = Arc::new(RwLock::new(None));
    let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
    let (event_tx, _) = broadcast::channel::<UiEvent>(32);
    let (model_tx, mut model_rx) = mpsc::channel::<ModelCommand>(32);
    let (controller_tx, mut controller_rx) = mpsc::channel::<ControllerCommand>(32);
    let (asset_tx, mut asset_rx) = mpsc::channel::<AssetProcessorCommand>(32);
    let (asset_result_tx, _) = broadcast::channel(8);

    let model_clone = model.clone();
    let event_tx_clone = event_tx.clone();
    let asset_result_tx_clone = asset_result_tx.clone();
    tokio::spawn(async move {
        let full_state = reqwest::get(format!("http://{}/api/show/full_state", host)).await.unwrap().json::<FullShowState>().await.unwrap();
        let mut show_model = model_clone.write().await;
        *show_model = full_state.show_model;
        drop(show_model);
        state_tx.send_modify(|state| {
            *state = full_state.show_state;
        });
        let response = Client::default().get(format!("ws://{}", host)).upgrade().send().await.unwrap();
        let mut websocket = response.into_websocket().await.unwrap();

        loop {
            tokio::select! {
                Ok(Some(message)) = websocket.try_next() => {
                    if let Message::Text(text) = message {
                        if let Ok(ws_message) = serde_json::from_str::<WsFeedback>(&text) {
                            match ws_message {
                                WsFeedback::Event(ui_event) => {
                                    if event_tx_clone.send(*ui_event).is_err() {
                                        log::error!("Failed to send UiEvent to channel.");
                                        break;
                                    }
                                },
                                WsFeedback::State(show_state) => {
                                    if state_tx.send(show_state).is_err() {
                                        log::error!("Failed to send ShowState to channel.");
                                        break;
                                    }
                                },
                                WsFeedback::AssetProcessorResult(process_result) => {
                                    if asset_result_tx_clone.send(process_result).is_err() {
                                        log::error!("Failed to send AssetProcessor result to channel.");
                                        break;
                                    }
                                },
                            }
                        } else {
                            log::error!("Invalid command received.")
                        }
                    } else if let Message::Close{ .. } = message {
                        log::info!("WebSocket client sent close message.");
                        break;
                    }
                }
                Some(model_command) = model_rx.recv() => {
                    let api_command = WsCommand::Model(Box::new(model_command));
                    if let Ok(payload) = serde_json::to_string(&api_command)
                    && websocket.send(Message::Text(payload)).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
                Some(controller_command) = controller_rx.recv() => {
                    let api_command = WsCommand::Controll(controller_command);
                    if let Ok(payload) = serde_json::to_string(&api_command)
                    && websocket.send(Message::Text(payload)).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
                Some(asset_processor_command) = asset_rx.recv() => {
                    let api_command = WsCommand::AssetProcessor(asset_processor_command);
                    if let Ok(payload) = serde_json::to_string(&api_command)
                    && websocket.send(Message::Text(payload)).await.is_err() {
                        log::info!("WebSocket client disconnected (send error).");
                        break;
                    }
                }
            }
        }
    });

    Ok((
        BackendHandle {
            model_handle: ShowModelHandle { model, command_tx: model_tx, show_model_path },
            asset_processor_handle: AssetProcessorHandle {
                result_rx_factory: asset_result_tx,
                command_tx: asset_tx,
            },
            controller_handle: CueControllerHandle {
                command_tx: controller_tx,
            },
        },
        state_rx,
        event_tx,
    ))
}