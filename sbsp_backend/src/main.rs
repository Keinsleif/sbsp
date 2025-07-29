mod apiserver;
mod event;
mod controller;
mod engine;
mod executor;
mod manager;
mod model;

use std::path::PathBuf;

use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;

use crate::{
    controller::{ControllerCommand, CueController, ShowState}, engine::audio_engine::{AudioCommand, AudioEngine}, event::UiEvent, executor::{EngineEvent, Executor, ExecutorCommand, ExecutorEvent}, manager::ShowModelManager, model::cue::{AudioCueFadeParam, AudioCueLevels, Cue}
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let (ctrl_tx, ctrl_rx) = mpsc::channel::<ControllerCommand>(32);
    let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
    let (executor_event_tx, executor_event_rx) = mpsc::channel::<ExecutorEvent>(32);
    let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);
    let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
    let (event_tx, _) = broadcast::channel::<UiEvent>(32);

    let (model_manager, model_handle) = ShowModelManager::new(event_tx.clone());

    let controller = CueController::new(
        model_handle.clone(),
        exec_tx,
        ctrl_rx,
        executor_event_rx,
        state_tx,
        event_tx.clone(),
    ).await;

    let executor = Executor::new(
        model_handle.clone(),
        exec_rx,
        audio_tx,
        executor_event_tx,
        engine_event_rx,
    );

    let audio_engine = AudioEngine::new(audio_rx, engine_event_tx)?;

    tokio::spawn(model_manager.run());
    tokio::spawn(controller.run());
    tokio::spawn(executor.run());
    tokio::spawn(audio_engine.run());

    let app = apiserver::create_api_router(ctrl_tx.clone(), state_rx, event_tx, model_handle.clone()).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8888").await?;
    log::info!("ApiServer listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
