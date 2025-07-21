mod controller;
mod engine;
mod executor;
mod manager;
mod model;

use std::{collections::HashMap, path::PathBuf, time::Duration};

use tokio::{sync::{mpsc, watch}, time::sleep};
use uuid::Uuid;

use crate::{
    controller::{ActiveCue, ControllerCommand, CueController},
    engine::audio_engine::{AudioCommand, AudioEngine},
    executor::{EngineEvent, Executor, ExecutorCommand, PlaybackEvent},
    manager::ShowModelManager,
    model::cue::{AudioCueFadeParam, AudioCueLevels, Cue},
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let (ctrl_tx, ctrl_rx) = mpsc::channel::<ControllerCommand>(32);
    let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
    let (state_tx, state_rx) = watch::channel::<HashMap<Uuid, ActiveCue>>(HashMap::new());
    let (playback_event_tx, playback_event_rx) = mpsc::channel::<PlaybackEvent>(32);
    let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);

    let model_manager = ShowModelManager::new();
    let cue_id = model_manager
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
            id
        })
        .await;

    let controller = CueController::new(model_manager.clone(), exec_tx, ctrl_rx, playback_event_rx, state_tx);

    let executor = Executor::new(
        model_manager.clone(),
        exec_rx,
        audio_tx,
        playback_event_tx,
        engine_event_rx,
    );

    let audio_engine = AudioEngine::new(audio_rx, engine_event_tx)?;

    tokio::spawn(controller.run());
    tokio::spawn(executor.run());
    tokio::spawn(audio_engine.run());

    if let Err(e) = ctrl_tx.send(ControllerCommand::Go { cue_id }).await {
        log::error!("Error while sending GO command: {:?}", e);
    }

    tokio::time::sleep(Duration::from_secs(5)).await;

    loop {
        if let Some(target_cue) = state_rx.borrow().get(&cue_id) {
            if target_cue.status.ne(&controller::PlaybackStatus::Completed) {
                sleep(Duration::from_millis(100)).await;
                continue;
            } else {
                break;
            }
        }
    }
    Ok(())
}
