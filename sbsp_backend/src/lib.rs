use serde::{Deserialize, Serialize};
#[cfg(feature = "backend")]
use tokio::sync::{broadcast, mpsc, oneshot, watch};

use crate::{model::ShowModel, manager::ProjectStatus, controller::state::ShowState};
#[cfg(feature = "backend")]
use crate::{
    asset_processor::{AssetProcessor, AssetProcessorHandle},
    controller::{CueController, CueControllerHandle},
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
    mod command;
    pub use command::AssetProcessorCommand;
}
#[cfg(feature = "type_export")]
pub mod controller {
    mod command;
    pub mod state;
    pub use command::ControllerCommand;
}
#[cfg(feature = "type_export")]
pub mod manager {
    mod command;
    pub use command::{InsertPosition, ModelCommand};
    mod project;
    pub use project::{ProjectFile, ProjectStatus};
}
#[cfg(feature = "type_export")]
pub mod api;

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FullShowState {
    pub project_status: ProjectStatus,
    pub show_model: ShowModel,
    pub show_state: ShowState,
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
    request_state_sync_tx: mpsc::Sender<()>,
    request_full_state_tx: mpsc::Sender<oneshot::Sender<FullShowState>>,
}

#[cfg(feature = "backend")]
impl BackendHandle {
    pub async fn request_state_sync(&self) {
        if let Err(e) = self.request_state_sync_tx.send(()).await {
            log::error!("Error when sending request state sync e={};", e);
        }
    }

    pub async fn get_full_state(&self) -> anyhow::Result<FullShowState> {
        let (request_responder_tx, request_responder_rx) = oneshot::channel();
        if let Err(e) = self.request_full_state_tx.send(request_responder_tx).await {
            log::error!("Error when sending request state sync e={};", e);
        }
        Ok(request_responder_rx.await?)
    }
}

#[cfg(feature = "backend")]
pub fn start_backend(
    settings_rx: watch::Receiver<BackendSettings>,
    enable_metering: bool,
) -> Result<
    (
        BackendHandle,
        watch::Receiver<ShowState>,
        broadcast::Sender<UiEvent>,
    ),
    anyhow::Error,
> {
    let (executor_command_tx, executor_command_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
    let (wait_tx, wait_rx) = mpsc::channel::<WaitCommand>(32);
    let (executor_event_tx, executor_event_rx) = mpsc::channel::<ExecutorEvent>(32);
    let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);
    let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
    let (event_tx, _) = broadcast::channel::<UiEvent>(32);

    let (model_manager, model_handle) =
        ShowModelManager::new(event_tx.clone(), settings_rx.clone());
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
        let (engine, shared_level) = AudioEngine::new_with_level_meter(
            audio_rx,
            engine_event_tx.clone(),
            ShowAudioSettings::default(),
        )?;
        (engine, Some(shared_level))
    } else {
        let engine = AudioEngine::new(
            audio_rx,
            engine_event_tx.clone(),
            ShowAudioSettings::default(),
        )?;
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

    let request_state_sync_tx = handle_state_sync(state_rx.clone(), event_tx.clone());

    let request_full_state_tx = handle_full_state(model_handle.clone(), state_rx.clone());

    Ok((
        BackendHandle {
            model_handle,
            asset_processor_handle,
            controller_handle,
            level_meter,
            request_state_sync_tx,
            request_full_state_tx,
        },
        state_rx,
        event_tx,
    ))
}

#[cfg(feature = "backend")]
fn handle_state_sync(state_rx: watch::Receiver<ShowState>, event_tx: broadcast::Sender<UiEvent>) -> mpsc::Sender<()> {
    let (sender, mut receiver) = mpsc::channel(8);

    tokio::spawn(async move {
        loop {
            if receiver.recv().await.is_some() {
                use crate::event::SyncData;

                let cues = {
                    use crate::event::CueState;

                    let state = state_rx.borrow();
                    state.active_cues.iter().map(|(id, ac)| CueState { id: *id, position: ac.position }).collect()
                };
                if let Err(e) = event_tx.send(UiEvent::SyncState(SyncData { latency: 0.0, cues })) {
                    log::trace!("No UI clients are listening to playback events. e={}", e);
                }
            } else {
                break;
            }
        }
    });

    sender
}

#[cfg(feature = "backend")]
fn handle_full_state(model_handle: ShowModelHandle, state_rx: watch::Receiver<ShowState>) -> mpsc::Sender<oneshot::Sender<FullShowState>> {
    let (request_full_state_tx, mut request_full_state_rx) = mpsc::channel::<oneshot::Sender<FullShowState>>(8);

    tokio::spawn(async move {
        while let Some(responder) = request_full_state_rx.recv().await {
            if responder.send(FullShowState {
                project_status: model_handle.get_project_state().await.clone(),
                show_model: model_handle.read().await.clone(),
                show_state: state_rx.borrow().clone(),
            }).is_err() {
                log::error!("error on responding full show state.");
                break;
            }
        }
    });
    request_full_state_tx
}