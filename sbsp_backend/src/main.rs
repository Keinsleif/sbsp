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
    model_manager
        .write_with(|model| {
            let id = Uuid::new_v4();
            model.name = "TestShowModel".to_string();
            model.cues.push(Cue {
                id,
                number: "1".to_string(),
                name: "Play IGY".to_string(),
                notes: "".to_string(),
                pre_wait: 0.0,
                post_wait: 0.0,
                sequence: model::cue::CueSequence::DoNotContinue,
                param: model::cue::CueParam::Audio {
                    target: PathBuf::from("./I.G.Y.flac"),
                    start_time: Some(5.0),
                    fade_in_param: Some(AudioCueFadeParam {
                        duration: 2.0,
                        easing: kira::Easing::Linear,
                    }),
                    end_time: Some(15.0),
                    fade_out_param: Some(AudioCueFadeParam {
                        duration: 5.0,
                        easing: kira::Easing::InPowi(2),
                    }),
                    levels: AudioCueLevels { master: 0.0 },
                    loop_region: None,
                },
            });
        })
        .await;

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

    tokio::spawn(controller.run());
    tokio::spawn(executor.run());
    tokio::spawn(audio_engine.run());

    let app = apiserver::create_api_router(ctrl_tx.clone(), state_rx, event_tx, model_handle.clone()).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8888").await?;
    log::info!("ApiServer listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
