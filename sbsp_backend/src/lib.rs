#[cfg(feature = "backend")]
use tokio::sync::{broadcast, mpsc, watch};

#[cfg(feature = "backend")]
use crate::{
    asset_processor::{AssetProcessor, AssetProcessorHandle},
    controller::{CueController, CueControllerHandle, state::ShowState},
    engine::{
        EngineEvent,
        audio_engine::{AudioCommand, AudioEngine, level_meter::SharedLevel},
        wait_engine::{WaitCommand, WaitEngine},
    },
    event::UiEvent,
    executor::{Executor, ExecutorCommand, ExecutorEvent},
    manager::{ShowModelHandle, ShowModelManager},
    model::settings::ShowAudioSettings,
};

#[cfg(feature = "backend")]
pub mod action;
#[cfg(feature = "backend")]
pub mod asset_processor;
#[cfg(feature = "backend")]
pub mod controller;
#[cfg(feature = "backend")]
mod engine;
pub mod event;
#[cfg(feature = "backend")]
mod executor;
#[cfg(feature = "backend")]
pub mod manager;
pub mod model;

#[cfg(any(feature = "apiserver", feature = "apiclient"))]
pub mod api;

#[cfg(feature = "type_export")]
pub mod asset_processor {
    mod data;
    pub use data::AssetData;
}
#[cfg(feature = "type_export")]
pub mod controller {
    pub mod state;
}
#[cfg(feature = "type_export")]
pub mod api {
    pub mod client {
        mod service_entry;
        pub use service_entry::ServiceEntry;
    }
    mod file_list;
    pub use file_list::FileList;
}

#[cfg(feature = "backend")]
#[derive(Default)]
pub struct BackendSettings {
    pub advance_cursor_when_go: bool,
    pub copy_assets_when_add: bool,
}

#[cfg(feature = "backend")]
#[derive(Clone)]
pub struct BackendHandle {
    pub model_handle: ShowModelHandle,
    pub asset_processor_handle: AssetProcessorHandle,
    pub controller_handle: CueControllerHandle,
    pub level_meter: Option<SharedLevel>,
}

#[cfg(feature = "backend")]
pub fn start_backend(settings_rx: watch::Receiver<BackendSettings>, enable_metering: bool) -> (
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
    let (event_tx, _) = broadcast::channel::<UiEvent>(32);

    let (model_manager, model_handle) = ShowModelManager::new(event_tx.clone(), settings_rx.clone());
    let (controller, controller_handle) = CueController::new(
        model_handle.clone(),
        settings_rx,
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

    let (audio_engine, level_meter) = if enable_metering {
        let (engine, shared_level) = AudioEngine::new_with_level_meter(audio_rx, engine_event_tx.clone(), ShowAudioSettings::default()).unwrap();
        (engine, Some(shared_level))
    } else {
        let engine = AudioEngine::new(audio_rx, engine_event_tx.clone(), ShowAudioSettings::default()).unwrap();
        (engine, None)
    };
    let wait_engine = WaitEngine::new(wait_rx, engine_event_tx);

    let (asset_processor, asset_processor_handle) =
        AssetProcessor::new(model_handle.clone(), event_tx.clone());

    tokio::spawn(model_manager.run());
    tokio::spawn(controller.run());
    tokio::spawn(executor.run());
    tokio::spawn(audio_engine.run());
    tokio::spawn(wait_engine.run());
    tokio::spawn(asset_processor.run());

    (
        BackendHandle {
            model_handle,
            asset_processor_handle,
            controller_handle,
            level_meter,
        },
        state_rx,
        event_tx,
    )
}
