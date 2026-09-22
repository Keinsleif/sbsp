// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::path::PathBuf;

use crate::{
    BackendSettings,
    event::{BackendEvent, CueStatusEventParam},
    executor::{ExecutorCommand, ExecutorEvent, StopMode},
    manager::ShowModelManager,
    model::{
        self,
        cue::{
            Cue, CueColor, CueCursorAdvanceTriggerOverride,
            audio::{AudioCueParam, Decibels, Easing, FadeParam, SoundType},
        },
    },
};

use super::{
    CueController, CueControllerHandle,
    state::{PlaybackStatus, ShowState, StateParam},
};

use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;

async fn setup_controller(
    cue_ids: &[Uuid],
) -> (
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
    {
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
    }
    tokio::spawn(manager.run());

    let (controller, controller_handle) = CueController::new(
        handle.clone(),
        exec_tx,
        playback_event_rx,
        state_tx,
        event_tx,
    );
    tokio::spawn(controller.run());

    (
        controller_handle,
        exec_rx,
        playback_event_tx,
        state_rx,
        event_rx,
    )
}

async fn bring_to_status(
    playback_event_tx: &mpsc::Sender<ExecutorEvent>,
    state_rx: &mut watch::Receiver<ShowState>,
    cue_id: Uuid,
    status: PlaybackStatus,
) {
    match status {
        PlaybackStatus::Loaded => {
            playback_event_tx
                .send(ExecutorEvent::Loaded {
                    cue_id,
                    position: 0.0,
                    duration: 0.0,
                })
                .await
                .unwrap();
        }
        PlaybackStatus::Playing => {
            playback_event_tx
                .send(ExecutorEvent::Started {
                    cue_id,
                    position: 0.0,
                    duration: 50.0,
                    initial_params: StateParam::None,
                })
                .await
                .unwrap();
        }
        PlaybackStatus::Paused => {
            playback_event_tx
                .send(ExecutorEvent::Started {
                    cue_id,
                    position: 0.0,
                    duration: 50.0,
                    initial_params: StateParam::None,
                })
                .await
                .unwrap();
            state_rx.changed().await.unwrap();
            playback_event_tx
                .send(ExecutorEvent::Paused {
                    cue_id,
                    position: 10.0,
                    duration: 50.0,
                })
                .await
                .unwrap();
        }
        PlaybackStatus::PreWaiting => {
            playback_event_tx
                .send(ExecutorEvent::PreWaitStarted {
                    cue_id,
                    duration: 5.0,
                })
                .await
                .unwrap();
        }
        PlaybackStatus::PreWaitPaused => {
            playback_event_tx
                .send(ExecutorEvent::PreWaitStarted {
                    cue_id,
                    duration: 5.0,
                })
                .await
                .unwrap();
            state_rx.changed().await.unwrap();
            playback_event_tx
                .send(ExecutorEvent::PreWaitPaused {
                    cue_id,
                    position: 1.0,
                    duration: 5.0,
                })
                .await
                .unwrap();
        }
        PlaybackStatus::Stopping => {
            playback_event_tx
                .send(ExecutorEvent::Started {
                    cue_id,
                    position: 0.0,
                    duration: 50.0,
                    initial_params: StateParam::None,
                })
                .await
                .unwrap();
            state_rx.changed().await.unwrap();
            playback_event_tx
                .send(ExecutorEvent::Stopping {
                    cue_id,
                    position: 40.0,
                    duration: 50.0,
                })
                .await
                .unwrap();
        }
    }
    state_rx.changed().await.unwrap();
}

#[tokio::test]
async fn go_command() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, _, _, _) = setup_controller(&[cue_id]).await;

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
    let (_, _, playback_event_tx, state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

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
    let (_, _, playback_event_tx, mut state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

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
    } else {
        unreachable!();
    }
    assert!(event_rx.is_empty());
}

#[tokio::test]
async fn pause_n_resume_event() {
    let cue_id = Uuid::new_v4();
    let (_, _, playback_event_tx, mut state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

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
    let (_, _, playback_event_tx, mut state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

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

#[tokio::test]
async fn load_command_sends_load_for_existing_inactive_cue() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, _playback_event_tx, _state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    controller_handle.load(cue_id).await.unwrap();

    assert!(matches!(
        exec_rx.recv().await.unwrap(),
        ExecutorCommand::Load(id) if id == cue_id
    ));
}

#[tokio::test]
async fn load_command_is_noop_when_cue_already_active() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        cue_id,
        PlaybackStatus::Loaded,
    )
    .await;

    controller_handle.load(cue_id).await.unwrap();

    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn load_command_is_noop_when_cue_not_found() {
    let existing_id = Uuid::new_v4();
    let unknown_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, _playback_event_tx, _state_rx, _event_rx) =
        setup_controller(&[existing_id]).await;

    controller_handle.load(unknown_id).await.unwrap();

    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn stop_command_uses_soft_mode_for_playing_cue() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        cue_id,
        PlaybackStatus::Playing,
    )
    .await;

    controller_handle.stop(cue_id).await.unwrap();

    assert!(matches!(
        exec_rx.recv().await.unwrap(),
        ExecutorCommand::Stop(id, StopMode::Soft) if id == cue_id
    ));
}

#[tokio::test]
async fn stop_command_uses_hard_mode_when_already_stopping() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        cue_id,
        PlaybackStatus::Stopping,
    )
    .await;

    controller_handle.stop(cue_id).await.unwrap();

    assert!(matches!(
        exec_rx.recv().await.unwrap(),
        ExecutorCommand::Stop(id, StopMode::Hard) if id == cue_id
    ));
}

#[tokio::test]
async fn stop_command_is_silently_ignored_when_cue_is_not_active() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, _playback_event_tx, _state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    controller_handle.stop(cue_id).await.unwrap();

    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn seek_to_command_sends_seek_for_active_cue() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        cue_id,
        PlaybackStatus::Playing,
    )
    .await;

    controller_handle.seek_to(cue_id, 12.5).await.unwrap();

    assert!(matches!(
        exec_rx.recv().await.unwrap(),
        ExecutorCommand::SeekTo(id, position) if id == cue_id && position == 12.5
    ));
}

#[tokio::test]
async fn seek_to_command_is_noop_when_cue_not_active() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, _playback_event_tx, _state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    controller_handle.seek_to(cue_id, 12.5).await.unwrap();

    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn seek_by_command_sends_seek_for_active_cue() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        cue_id,
        PlaybackStatus::Playing,
    )
    .await;

    controller_handle.seek_by(cue_id, -3.0).await.unwrap();

    assert!(matches!(
        exec_rx.recv().await.unwrap(),
        ExecutorCommand::SeekBy(id, amount) if id == cue_id && amount == -3.0
    ));
}

#[tokio::test]
async fn seek_by_command_is_noop_when_cue_not_found() {
    let existing_id = Uuid::new_v4();
    let unknown_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, _playback_event_tx, _state_rx, _event_rx) =
        setup_controller(&[existing_id]).await;

    controller_handle.seek_by(unknown_id, 1.0).await.unwrap();

    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn perform_action_command_forwards_for_active_cue() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        cue_id,
        PlaybackStatus::Playing,
    )
    .await;

    let action = crate::action::CueAction::Audio(crate::action::AudioAction::ToggleRepeat);

    controller_handle
        .perform_action(cue_id, action)
        .await
        .unwrap();

    assert!(matches!(
        exec_rx.recv().await.unwrap(),
        ExecutorCommand::PerformAction(id, _) if id == cue_id
    ));
}

#[tokio::test]
async fn perform_action_command_is_noop_when_cue_not_active() {
    let cue_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, _playback_event_tx, _state_rx, _event_rx) =
        setup_controller(&[cue_id]).await;

    let action = crate::action::CueAction::Audio(crate::action::AudioAction::ToggleRepeat);

    controller_handle
        .perform_action(cue_id, action)
        .await
        .unwrap();

    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn pause_all_targets_only_playing_and_prewaiting_cues() {
    let playing_id = Uuid::new_v4();
    let prewaiting_id = Uuid::new_v4();
    let paused_id = Uuid::new_v4();
    let loaded_id = Uuid::new_v4();

    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[playing_id, prewaiting_id, paused_id, loaded_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        playing_id,
        PlaybackStatus::Playing,
    )
    .await;
    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        prewaiting_id,
        PlaybackStatus::PreWaiting,
    )
    .await;
    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        paused_id,
        PlaybackStatus::Paused,
    )
    .await;
    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        loaded_id,
        PlaybackStatus::Loaded,
    )
    .await;

    controller_handle.pause_all().await.unwrap();

    let mut paused_targets = std::collections::HashSet::new();
    for _ in 0..2 {
        if let ExecutorCommand::Pause(id) = exec_rx.recv().await.unwrap() {
            paused_targets.insert(id);
        } else {
            panic!("expected ExecutorCommand::Pause");
        }
    }
    assert_eq!(
        paused_targets,
        [playing_id, prewaiting_id].into_iter().collect()
    );
    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn resume_all_targets_only_paused_and_prewait_paused_cues() {
    let paused_id = Uuid::new_v4();
    let prewait_paused_id = Uuid::new_v4();
    let playing_id = Uuid::new_v4();

    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[paused_id, prewait_paused_id, playing_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        paused_id,
        PlaybackStatus::Paused,
    )
    .await;
    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        prewait_paused_id,
        PlaybackStatus::PreWaitPaused,
    )
    .await;
    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        playing_id,
        PlaybackStatus::Playing,
    )
    .await;

    controller_handle.resume_all().await.unwrap();

    let mut resumed_targets = std::collections::HashSet::new();
    for _ in 0..2 {
        if let ExecutorCommand::Resume(id) = exec_rx.recv().await.unwrap() {
            resumed_targets.insert(id);
        } else {
            panic!("expected ExecutorCommand::Resume");
        }
    }
    assert_eq!(
        resumed_targets,
        [paused_id, prewait_paused_id].into_iter().collect()
    );
    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn stop_all_uses_hard_mode_only_for_stopping_cues() {
    let playing_id = Uuid::new_v4();
    let stopping_id = Uuid::new_v4();

    let (controller_handle, mut exec_rx, playback_event_tx, mut state_rx, _event_rx) =
        setup_controller(&[playing_id, stopping_id]).await;

    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        playing_id,
        PlaybackStatus::Playing,
    )
    .await;
    bring_to_status(
        &playback_event_tx,
        &mut state_rx,
        stopping_id,
        PlaybackStatus::Stopping,
    )
    .await;

    controller_handle.stop_all().await.unwrap();

    let mut seen = std::collections::HashMap::new();
    for _ in 0..2 {
        if let ExecutorCommand::Stop(id, mode) = exec_rx.recv().await.unwrap() {
            seen.insert(id, mode);
        } else {
            panic!("expected ExecutorCommand::Stop");
        }
    }
    assert_eq!(seen.get(&playing_id), Some(&StopMode::Soft));
    assert_eq!(seen.get(&stopping_id), Some(&StopMode::Hard));
}

#[tokio::test]
async fn pause_all_ignores_inactive_root_cues() {
    let inactive_id = Uuid::new_v4();
    let (controller_handle, mut exec_rx, _playback_event_tx, _state_rx, _event_rx) =
        setup_controller(&[inactive_id]).await;

    controller_handle.pause_all().await.unwrap();

    assert!(exec_rx.try_recv().is_err());
}

#[tokio::test]
async fn stopping_event_sets_status_and_is_sent_once() {
    let cue_id = Uuid::new_v4();
    let (_, _, playback_event_tx, mut state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();
    state_rx.changed().await.unwrap();
    let _ = event_rx.recv().await.unwrap();

    state_rx.mark_unchanged();

    playback_event_tx
        .send(ExecutorEvent::Stopping {
            cue_id,
            position: 40.0,
            duration: 50.0,
        })
        .await
        .unwrap();

    let _ = state_rx.changed().await;
    if let Some(active_cue) = state_rx.borrow().active_cues.get(&cue_id) {
        assert_eq!(active_cue.status, PlaybackStatus::Stopping);
        assert_eq!(active_cue.position, 40.0);
    } else {
        unreachable!();
    }

    let event = event_rx.recv().await.unwrap();
    assert!(matches!(
        event,
        BackendEvent::CueStatus(CueStatusEventParam::Stopping { cue_id: id, .. }) if id == cue_id
    ));

    playback_event_tx
        .send(ExecutorEvent::Stopping {
            cue_id,
            position: 45.0,
            duration: 50.0,
        })
        .await
        .unwrap();

    let _ = state_rx.changed().await;
    assert_eq!(
        state_rx.borrow().active_cues.get(&cue_id).unwrap().position,
        45.0
    );
    assert!(
        event_rx.try_recv().is_err(),
        "second Stopping event for the same cue must not be re-broadcast"
    );
}

#[tokio::test]
async fn stopping_event_for_unknown_cue_is_ignored() {
    let known_id = Uuid::new_v4();
    let unknown_id = Uuid::new_v4();
    let (_, _, playback_event_tx, mut state_rx, mut event_rx) = setup_controller(&[known_id]).await;

    state_rx.mark_unchanged();

    playback_event_tx
        .send(ExecutorEvent::Stopping {
            cue_id: unknown_id,
            position: 1.0,
            duration: 2.0,
        })
        .await
        .unwrap();

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id: known_id,
            position: 0.0,
            duration: 50.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();

    state_rx.changed().await.unwrap();
    assert!(matches!(
        event_rx.recv().await.unwrap(),
        BackendEvent::CueStatus(CueStatusEventParam::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            params: StateParam::None,
        }) if cue_id == known_id
    ));
    assert!(!state_rx.has_changed().unwrap());
    assert!(event_rx.try_recv().is_err());
}

#[tokio::test]
async fn error_event_removes_active_cue_and_notifies() {
    let cue_id = Uuid::new_v4();
    let (_, _, playback_event_tx, mut state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();
    state_rx.changed().await.unwrap();
    let _ = event_rx.recv().await.unwrap();

    playback_event_tx
        .send(ExecutorEvent::Error {
            cue_id,
            error: "engine failure".to_string(),
        })
        .await
        .unwrap();

    state_rx.changed().await.unwrap();
    assert!(!state_rx.borrow().active_cues.contains_key(&cue_id));

    let event = event_rx.recv().await.unwrap();
    assert!(matches!(
        event,
        BackendEvent::CueStatus(CueStatusEventParam::Error { cue_id: id, .. }) if id == cue_id
    ));
}

#[tokio::test]
async fn seeked_event_updates_position_for_active_cue() {
    let cue_id = Uuid::new_v4();
    let (_, _, playback_event_tx, mut state_rx, mut event_rx) = setup_controller(&[cue_id]).await;

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();
    state_rx.changed().await.unwrap();
    let _ = event_rx.recv().await.unwrap();

    playback_event_tx
        .send(ExecutorEvent::Seeked {
            cue_id,
            position: 33.0,
        })
        .await
        .unwrap();

    state_rx.changed().await.unwrap();
    assert_eq!(
        state_rx.borrow().active_cues.get(&cue_id).unwrap().position,
        33.0
    );

    let event = event_rx.recv().await.unwrap();
    assert!(matches!(
        event,
        BackendEvent::CueStatus(CueStatusEventParam::Seeked { cue_id: id, position }) if id == cue_id && position == 33.0
    ));
}

#[tokio::test]
async fn seeked_event_for_unknown_cue_is_ignored() {
    let known_id = Uuid::new_v4();
    let unknown_id = Uuid::new_v4();
    let (_, _, playback_event_tx, mut state_rx, mut event_rx) = setup_controller(&[known_id]).await;

    state_rx.mark_unchanged();

    playback_event_tx
        .send(ExecutorEvent::Seeked {
            cue_id: unknown_id,
            position: 5.0,
        })
        .await
        .unwrap();

    playback_event_tx
        .send(ExecutorEvent::Started {
            cue_id: known_id,
            position: 0.0,
            duration: 50.0,
            initial_params: StateParam::None,
        })
        .await
        .unwrap();

    state_rx.changed().await.unwrap();
    assert!(matches!(
        event_rx.recv().await.unwrap(),
        BackendEvent::CueStatus(CueStatusEventParam::Started {
            cue_id,
            position: 0.0,
            duration: 50.0,
            params: StateParam::None,
        }) if cue_id == known_id
    ));
    assert!(!state_rx.has_changed().unwrap());
    assert!(event_rx.try_recv().is_err());
}
