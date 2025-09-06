#[cfg(not(feature = "type_export"))]
use tokio::sync::{broadcast, mpsc, watch};

#[cfg(not(feature = "type_export"))]
use crate::{
    asset_processor::{AssetProcessor, AssetProcessorHandle},
    controller::{CueController, CueControllerHandle, state::ShowState},
    engine::{
        EngineEvent,
        audio_engine::{AudioCommand, AudioEngine},
        wait_engine::{WaitCommand, WaitEngine},
    },
    event::UiEvent,
    executor::{Executor, ExecutorCommand, ExecutorEvent},
    manager::{ShowModelHandle, ShowModelManager},
    model::settings::AudioSettings,
};

#[cfg(feature = "apiserver")]
pub mod apiserver;

#[cfg(not(feature = "type_export"))]
pub mod action;
#[cfg(not(feature = "type_export"))]
pub mod asset_processor;
#[cfg(feature = "type_export")]
pub mod asset_processor {
    mod data;
    pub use data::AssetData;
}
#[cfg(not(feature = "type_export"))]
pub mod controller;
#[cfg(feature = "type_export")]
pub mod controller {
    pub mod state;
}
#[cfg(not(feature = "type_export"))]
mod engine;
pub mod event;
#[cfg(not(feature = "type_export"))]
mod executor;
#[cfg(not(feature = "type_export"))]
pub mod manager;
pub mod model;

#[cfg(not(feature = "type_export"))]
pub struct BackendHandle {
    pub model_handle: ShowModelHandle,
    pub asset_handle: AssetProcessorHandle,
    pub controller_handle: CueControllerHandle,
}

#[cfg(not(feature = "type_export"))]
pub fn start_backend() -> (
    BackendHandle,
    watch::Receiver<ShowState>,
    broadcast::Sender<UiEvent>,
) {
    let (executor_command_tx, executor_command_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
    let (wait_tx, wait_rx) = mpsc::channel::<WaitCommand>(32);
    let (executor_event_tx, executor_event_rx) = mpsc::channel::<ExecutorEvent>(32);
    let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);
    let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
    let (event_tx, event_rx) = broadcast::channel::<UiEvent>(32);

    let (model_manager, model_handle) = ShowModelManager::new(event_tx.clone());
    let (controller, controller_handle) = CueController::new(
        model_handle.clone(),
        executor_command_tx,
        executor_event_rx,
        state_tx,
        event_tx.clone(),
    );

    let executor = Executor::new(
        model_handle.clone(),
        executor_command_rx,
        audio_tx,
        wait_tx,
        executor_event_tx,
        engine_event_rx,
    );

    let audio_engine =
        AudioEngine::new(audio_rx, engine_event_tx.clone(), AudioSettings::default()).unwrap();
    let wait_engine = WaitEngine::new(wait_rx, engine_event_tx);

    let (asset_processor, asset_handle) = AssetProcessor::new(model_handle.clone(), event_rx);

    tokio::spawn(model_manager.run());
    tokio::spawn(controller.run());
    tokio::spawn(executor.run());
    tokio::spawn(audio_engine.run());
    tokio::spawn(wait_engine.run());
    tokio::spawn(asset_processor.run());

    (
        BackendHandle {
            model_handle,
            asset_handle,
            controller_handle,
        },
        state_rx,
        event_tx,
    )
}
