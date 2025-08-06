use tokio::sync::{broadcast, mpsc, watch};

use crate::{controller::{ControllerCommand, CueController, state::ShowState}, engine::audio_engine::{AudioCommand, AudioEngine}, event::UiEvent, executor::{EngineEvent, Executor, ExecutorCommand, ExecutorEvent}, manager::{ShowModelHandle, ShowModelManager}};

pub mod event;
pub mod controller;
mod engine;
mod executor;
pub mod manager;
pub mod model;
pub mod apiserver;

pub struct BackendHandle {
    pub model_handle: ShowModelHandle,
    pub controller_tx: mpsc::Sender<ControllerCommand>,
}

pub fn start_backend() -> (
    BackendHandle,
    watch::Receiver<ShowState>,
    broadcast::Sender<UiEvent>,
) {
    let (controller_tx, controller_rx) = mpsc::channel::<ControllerCommand>(32);
    let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
    let (executor_event_tx, executor_event_rx) = mpsc::channel::<ExecutorEvent>(32);
    let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);
    let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
    let (event_tx, _event_rx) = broadcast::channel::<UiEvent>(32);

    let (model_manager, model_handle) = ShowModelManager::new(event_tx.clone());
    let controller = CueController::new(
        model_handle.clone(),
        exec_tx,
        controller_rx,
        executor_event_rx,
        state_tx,
        event_tx.clone(),
    );

    let executor = Executor::new(
        model_handle.clone(),
        exec_rx,
        audio_tx,
        executor_event_tx,
        engine_event_rx,
    );

    let audio_engine = AudioEngine::new(audio_rx, engine_event_tx).unwrap();

    tokio::spawn(model_manager.run());
    tokio::spawn(controller.run());
    tokio::spawn(executor.run());
    tokio::spawn(audio_engine.run());

    (
        BackendHandle { model_handle, controller_tx },
        state_rx, event_tx,
    )
}