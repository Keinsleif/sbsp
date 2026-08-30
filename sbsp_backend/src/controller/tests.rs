
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::path::PathBuf;

use crate::{
    BackendSettings, event::{BackendEvent, CueStatusEventParam}, executor::{ExecutorCommand, ExecutorEvent}, manager::ShowModelManager, model::{
        self,
        cue::{
            Cue, CueColor, CueCursorAdvanceTriggerOverride,
            audio::{AudioCueParam, Decibels, Easing, FadeParam, SoundType},
        },
    },
};

use super::{state::{ShowState, StateParam, PlaybackStatus}, CueController, CueControllerHandle};

use tokio::sync::{
    mpsc,
    watch,broadcast
};
use uuid::Uuid;

async fn setup_controller(
    cue_ids: &[Uuid],
) -> (
    CueController,
    CueControllerHandle,
    mpsc::Receiver<ExecutorCommand>,
    mpsc::Sender<ExecutorEvent>,
    watch::Receiver<ShowState>,
    broadcast::Receiver<BackendEvent>,
) {
    let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (playback_event_tx, playback_event_rx) = mpsc::channel::<ExecutorEvent>(32);
    let (state_tx, state_rx) = watch::channel::<ShowState>(ShowState::new());
    let (event_tx, event_rx) = broadcast::channel::<BackendEvent>(32);

    let (_, settings_rx) = watch::channel(BackendSettings::default());

    let (manager, handle) = ShowModelManager::new(event_tx.clone(), settings_rx.clone());
    let mut write_lock = manager.write().await;
    write_lock.name = "TestShowModel".to_string();
    for cue_id in cue_ids {
        write_lock.cue_list.root_ids.push(*cue_id);
        write_lock.cue_list.cues.insert(
            *cue_id,
            Cue {
                id: *cue_id,
                number: "1".to_string(),
                name: None,
                notes: "".to_string(),
                color: CueColor::None,
                pre_wait: 0.0,
                chain: model::cue::CueChain::DoNotChain,
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                parent_id: None,
                params: model::cue::CueParam::Audio(AudioCueParam {
                    target: PathBuf::from("./I.G.Y.flac"),
                    start_time: Some(5.0),
                    fade_in_param: Some(FadeParam {
                        duration: 2.0,
                        easing: Easing::Linear,
                    }),
                    end_time: Some(50.0),
                    fade_out_param: Some(FadeParam {
                        duration: 5.0,
                        easing: Easing::InPow(2.0),
                    }),
                    volume: Decibels::IDENTITY,
                    pan: 0.0,
                    repeat: false,
                    sound_type: SoundType::Streaming,
                    envelope: Vec::new(),
                }),
            },
        );
    }
    let (controller, controller_handle) = CueController::new(
        handle.clone(),
        exec_tx,
        playback_event_rx,
        state_tx,
        event_tx,
    );

    (
        controller,
        controller_handle,
        exec_rx,
        playback_event_tx,
        state_rx,
        event_rx,
    )
}

#[tokio::test]
async fn go_command() {
    let cue_id = Uuid::new_v4();
    let (controller, controller_handle, mut exec_rx, _, _, _) =
        setup_controller(&[cue_id]).await;

    tokio::spawn(controller.run());

    controller_handle.execute(cue_id).await.unwrap();

    if let Some(ExecutorCommand::Execute(id)) = exec_rx.recv().await {
        assert_eq!(id, cue_id);
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn started_event() {
    let cue_id = Uuid::new_v4();
    let (controller, _, _, playback_event_tx, state_rx, mut event_rx) =
        setup_controller(&[cue_id]).await;

    tokio::spawn(controller.run());

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id,
            position: 0.0,
            duration: 43.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();

    let event = event_rx.recv().await.unwrap();
    assert!(
        event.eq(&BackendEvent::CueStatus(CueStatusEventParam::Started {
            cue_id,
            position: 0.0,
            duration: 43.0,
            params: StateParam::None
        }))
    );
    if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
        assert_eq!(active_cue.cue_id, cue_id);
        assert_eq!(active_cue.status, PlaybackStatus::Playing);
        assert_eq!(active_cue.duration, 43.0);
        assert_eq!(active_cue.position, 0.0);
        assert_eq!(active_cue.params, StateParam::None);
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn progress_event() {
    let cue_id = Uuid::new_v4();
    let (controller, _, _, playback_event_tx, mut state_rx, mut event_rx) =
        setup_controller(&[cue_id]).await;
    tokio::spawn(controller.run());

    state_rx.mark_unchanged();

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();

    let _ = state_rx.changed().await;
    assert_eq!(
        event_rx.recv().await.unwrap(),
        BackendEvent::CueStatus(CueStatusEventParam::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            params: StateParam::None
        })
    );

    playback_event_tx
        .send(ExecutorEvent::Progress {
            cue_id,
            position: 20.0,
            duration: 50.0,
        })
        .await
        .unwrap();

    let _ = state_rx.changed().await;
    if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
        assert_eq!(active_cue.cue_id, cue_id);
        assert_eq!(active_cue.status, PlaybackStatus::Playing);
        assert_eq!(active_cue.position, 20.0);
        assert_eq!(active_cue.duration, 50.0);
    }
    assert!(event_rx.is_empty());
}

#[tokio::test]
async fn pause_n_resume_event() {
    let cue_id = Uuid::new_v4();
    let (controller, _, _, playback_event_tx, mut state_rx, mut event_rx) =
        setup_controller(&[cue_id]).await;

    tokio::spawn(controller.run());

    state_rx.mark_unchanged();

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();

    let _ = state_rx.changed().await;
    let event = event_rx.recv().await.unwrap();
    assert_eq!(
        event,
        BackendEvent::CueStatus(CueStatusEventParam::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            params: StateParam::None
        })
    );

    playback_event_tx
        .send(ExecutorEvent::Paused {
            cue_id,
            position: 21.0,
            duration: 50.0,
        })
        .await
        .unwrap();

    let event = event_rx.recv().await.unwrap();
    assert_eq!(
        event,
        BackendEvent::CueStatus(CueStatusEventParam::Paused {
            cue_id,
            position: 21.0
        })
    );
    if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
        assert_eq!(active_cue.cue_id, cue_id);
        assert_eq!(active_cue.status, PlaybackStatus::Paused);
        assert_eq!(active_cue.position, 21.0);
        assert_eq!(active_cue.duration, 50.0);
    } else {
        unreachable!();
    }

    playback_event_tx
        .send(ExecutorEvent::Resumed { cue_id })
        .await
        .unwrap();

    let event = event_rx.recv().await.unwrap();
    assert!(
        event.eq(&BackendEvent::CueStatus(CueStatusEventParam::Resumed {
            cue_id
        }))
    );
    if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
        assert_eq!(active_cue.cue_id, cue_id);
        assert_eq!(active_cue.status, PlaybackStatus::Playing);
        assert_eq!(active_cue.position, 21.0);
        assert_eq!(active_cue.duration, 50.0);
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn completed_event() {
    let cue_id = Uuid::new_v4();
    let (controller, _, _, playback_event_tx, mut state_rx, mut event_rx) =
        setup_controller(&[cue_id]).await;

    tokio::spawn(controller.run());

    state_rx.mark_unchanged();

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();

    let _ = state_rx.changed().await;
    let event = event_rx.recv().await.unwrap();
    assert_eq!(
        event,
        BackendEvent::CueStatus(CueStatusEventParam::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            params: StateParam::None
        })
    );

    playback_event_tx
        .send(ExecutorEvent::Completed { cue_id })
        .await
        .unwrap();

    let event = event_rx.recv().await.unwrap();
    assert!(
        event.eq(&BackendEvent::CueStatus(CueStatusEventParam::Completed {
            cue_id
        }))
    );
    assert!(!state_rx.borrow().active_cues.contains_key(&cue_id));
}
