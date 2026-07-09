use super::*;
use std::path::PathBuf;

use tempfile::NamedTempFile;
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver, Sender},
    watch,
};
use uuid::Uuid;

use crate::{
    BackendSettings,
    controller::state::AudioStateParam,
    engine::audio_engine::{AudioCommand, AudioEngineEvent},
    event::BackendEvent,
    manager::ShowModelManager,
    model::{
        self,
        cue::{
            CueColor,
            audio::{Decibels, Easing, FadeParam, SoundType},
        },
    },
};

async fn setup_executor(
    cue_id: Uuid,
    path: PathBuf,
) -> (
    ShowModelManager,
    Sender<ExecutorCommand>,
    Receiver<AudioCommand>,
    Sender<EngineEvent>,
    Receiver<ExecutorEvent>,
) {
    let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
    let (wait_tx, _wait_rx) = mpsc::channel::<WaitCommand>(32);
    let (playback_event_tx, playback_event_rx) = mpsc::channel::<ExecutorEvent>(32);
    let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);
    let (event_tx, _) = broadcast::channel::<BackendEvent>(32);
    let (_, settings_rx) = watch::channel(BackendSettings::default());

    let (manager, handle) = ShowModelManager::new(event_tx.clone(), settings_rx);
    let mut write_lock = manager.write().await;
    write_lock.name = "TestShowModel".to_string();
    write_lock.cue_list.root_ids.push(cue_id);
    write_lock.cue_list.cues.insert(
        cue_id,
        Cue {
            id: cue_id,
            number: "1".to_string(),
            name: None,
            notes: "".to_string(),
            color: CueColor::None,
            pre_wait: 0.0,
            chain: model::cue::CueChain::DoNotChain,
            parent_id: None,
            params: model::cue::CueParam::Audio(AudioCueParam {
                target: path,
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
    drop(write_lock);

    let executor = Executor::new(
        handle.clone(),
        exec_rx,
        audio_tx,
        wait_tx,
        playback_event_tx,
        engine_event_rx,
    );

    tokio::spawn(executor.run());

    (
        manager,
        exec_tx,
        audio_rx,
        engine_event_tx,
        playback_event_rx,
    )
}

async fn setup_executor_with_cues(
    cues: Vec<Cue>,
    root_ids: Vec<Uuid>,
) -> (
    ShowModelManager,
    Sender<ExecutorCommand>,
    Receiver<AudioCommand>,
    Sender<EngineEvent>,
    Receiver<ExecutorEvent>,
) {
    let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
    let (wait_tx, _wait_rx) = mpsc::channel::<WaitCommand>(32);
    let (playback_event_tx, playback_event_rx) = mpsc::channel::<ExecutorEvent>(32);
    let (engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(32);
    let (event_tx, _) = broadcast::channel::<BackendEvent>(32);
    let (_, settings_rx) = watch::channel(BackendSettings::default());

    let (manager, handle) = ShowModelManager::new(event_tx.clone(), settings_rx);
    let mut write_lock = manager.write().await;
    write_lock.name = "TestShowModel".to_string();
    write_lock.cue_list.root_ids = root_ids;
    for cue in cues {
        write_lock.cue_list.cues.insert(cue.id, cue);
    }
    drop(write_lock);

    let executor = Executor::new(
        handle.clone(),
        exec_rx,
        audio_tx,
        wait_tx,
        playback_event_tx,
        engine_event_rx,
    );
    tokio::spawn(executor.run());

    (manager, exec_tx, audio_rx, engine_event_tx, playback_event_rx)
}

fn make_audio_cue(id: Uuid, parent_id: Option<Uuid>, path: PathBuf) -> Cue {
    Cue {
        id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        parent_id,
        params: model::cue::CueParam::Audio(AudioCueParam {
            target: path,
            start_time: None,
            fade_in_param: None,
            end_time: None,
            fade_out_param: None,
            volume: Decibels::IDENTITY,
            pan: 0.0,
            repeat: false,
            sound_type: SoundType::Streaming,
            envelope: Vec::new(),
        }),
    }
}

fn make_playlist_group_cue(
    id: Uuid,
    parent_id: Option<Uuid>,
    children: Vec<Uuid>,
    repeat: bool,
) -> Cue {
    Cue {
        id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        parent_id,
        params: model::cue::CueParam::Group {
            base: crate::model::cue::group::GroupCueParamBase {
                mode: GroupMode::Playlist { repeat },
            },
            children,
        },
    }
}

#[tokio::test]
async fn play_command() {
    let cue_id = Uuid::new_v4();

    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let (_, exec_tx, mut audio_rx, _, _) =
        setup_executor(cue_id, temp_target.path().to_path_buf()).await;

    exec_tx
        .send(ExecutorCommand::Execute(cue_id))
        .await
        .unwrap();

    loop {
        if let Some(command) = audio_rx.recv().await {
            if let AudioCommand::Play { data, .. } = command {
                assert_eq!(data.filepath, temp_target.path().to_path_buf());
                assert_eq!(data.volume, Decibels::IDENTITY);
                assert_eq!(data.pan, 0.0);
                assert_eq!(data.start_time, Some(5.0));
                assert_eq!(
                    data.fade_in_param,
                    Some(FadeParam {
                        duration: 2.0,
                        easing: Easing::Linear
                    })
                );
                assert_eq!(data.end_time, Some(50.0));
                assert_eq!(
                    data.fade_out_param,
                    Some(FadeParam {
                        duration: 5.0,
                        easing: Easing::InPow(2.0)
                    })
                );
                assert!(!data.repeat);
                break;
            }
        } else {
            panic!("audio_tx dropped.");
        }
    }
}

#[tokio::test]
async fn started_event() {
    let orig_cue_id = Uuid::new_v4();

    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor(orig_cue_id, temp_target.path().to_path_buf()).await;

    exec_tx
        .send(ExecutorCommand::Execute(orig_cue_id))
        .await
        .unwrap();

    let command = audio_rx.recv().await.unwrap();

    let instance_id = if let AudioCommand::Play { id, .. } = command {
        id
    } else {
        unreachable!();
    };

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id,
            position: 0.0,
            duration: 23.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();

    if let Some(event) = playback_event_rx.recv().await {
        if let ExecutorEvent::Started {
            cue_id,
            position,
            duration,
            initial_params,
        } = event
        {
            assert_eq!(cue_id, orig_cue_id);
            assert_eq!(position, 0.0);
            assert_eq!(duration, 23.0);
            assert_eq!(
                initial_params,
                StateParam::Audio(AudioStateParam::default())
            );
        } else {
            panic!("Wrong Playback Event emitted.");
        }
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn progress_event() {
    let orig_cue_id = Uuid::new_v4();

    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor(orig_cue_id, temp_target.path().to_path_buf()).await;

    exec_tx
        .send(ExecutorCommand::Execute(orig_cue_id))
        .await
        .unwrap();

    let command = audio_rx.recv().await.unwrap();

    let instance_id = if let AudioCommand::Play { id, .. } = command {
        id
    } else {
        unreachable!();
    };

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Progress {
            instance_id,
            position: 20.0,
            duration: 50.0,
        }))
        .await
        .unwrap();

    if let Some(event) = playback_event_rx.recv().await {
        if let ExecutorEvent::Progress {
            cue_id,
            position,
            duration,
        } = event
        {
            assert_eq!(cue_id, orig_cue_id);
            assert_eq!(position, 20.0);
            assert_eq!(duration, 50.0);
        } else {
            panic!("Wrong Playback Event emitted.");
        }
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn pause_event() {
    let orig_cue_id = Uuid::new_v4();

    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor(orig_cue_id, temp_target.path().to_path_buf()).await;

    exec_tx
        .send(ExecutorCommand::Execute(orig_cue_id))
        .await
        .unwrap();

    let command = audio_rx.recv().await.unwrap();

    let instance_id = if let AudioCommand::Play { id, .. } = command {
        id
    } else {
        unreachable!();
    };

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Paused {
            instance_id,
            position: 24.0,
            duration: 50.0,
        }))
        .await
        .unwrap();

    if let Some(event) = playback_event_rx.recv().await {
        if let ExecutorEvent::Paused {
            cue_id,
            position,
            duration,
        } = event
        {
            assert_eq!(cue_id, orig_cue_id);
            assert_eq!(position, 24.0);
            assert_eq!(duration, 50.0);
        } else {
            panic!("Wrong Playback Event emitted.");
        }
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn resume_event() {
    let orig_cue_id = Uuid::new_v4();

    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor(orig_cue_id, temp_target.path().to_path_buf()).await;

    exec_tx
        .send(ExecutorCommand::Execute(orig_cue_id))
        .await
        .unwrap();

    let instance_id = loop {
        if let Some(command) = audio_rx.recv().await {
            if let AudioCommand::Play { id, .. } = command {
                break id;
            }
        } else {
            panic!("audio_tx dropped.")
        }
    };

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Resumed {
            instance_id,
        }))
        .await
        .unwrap();

    if let Some(event) = playback_event_rx.recv().await {
        if let ExecutorEvent::Resumed { cue_id } = event {
            assert_eq!(cue_id, orig_cue_id);
        } else {
            panic!("Wrong Playback Event emitted.");
        }
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn completed_event() {
    let orig_cue_id = Uuid::new_v4();

    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor(orig_cue_id, temp_target.path().to_path_buf()).await;

    exec_tx
        .send(ExecutorCommand::Execute(orig_cue_id))
        .await
        .unwrap();

    let command = audio_rx.recv().await.unwrap();

    let instance_id = if let AudioCommand::Play { id, .. } = command {
        id
    } else {
        unreachable!();
    };

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id,
        }))
        .await
        .unwrap();

    if let Some(event) = playback_event_rx.recv().await {
        if let ExecutorEvent::Completed { cue_id } = event {
            assert_eq!(cue_id, orig_cue_id);
        } else {
            panic!("Wrong Playback Event emitted.");
        }
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn error_event() {
    let orig_cue_id = Uuid::new_v4();

    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let (_, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor(orig_cue_id, temp_target.path().to_path_buf()).await;

    exec_tx
        .send(ExecutorCommand::Execute(orig_cue_id))
        .await
        .unwrap();

    let command = audio_rx.recv().await.unwrap();

    let instance_id = if let AudioCommand::Play { id, .. } = command {
        id
    } else {
        unreachable!();
    };

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Error {
            instance_id,
            error: "Error".to_string(),
        }))
        .await
        .unwrap();

    if let Some(event) = playback_event_rx.recv().await {
        if let ExecutorEvent::Error { cue_id, error } = event {
            assert_eq!(cue_id, orig_cue_id);
            assert_eq!(error, "Error".to_string());
        } else {
            panic!("Wrong Playback Event emitted.");
        }
    } else {
        unreachable!();
    }
}

#[tokio::test]
async fn nested_group_completed_chain_propagates_to_sibling() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let outer_id = Uuid::new_v4();
    let inner_id = Uuid::new_v4();
    let child_a_id = Uuid::new_v4();
    let child_b_id = Uuid::new_v4();
    let sibling_id = Uuid::new_v4();

    let cues = vec![
        make_playlist_group_cue(outer_id, None, vec![inner_id, sibling_id], false),
        make_playlist_group_cue(inner_id, Some(outer_id), vec![child_a_id, child_b_id], false),
        make_audio_cue(child_a_id, Some(inner_id), path.clone()),
        make_audio_cue(child_b_id, Some(inner_id), path.clone()),
        make_audio_cue(sibling_id, Some(outer_id), path.clone()),
    ];

    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![outer_id]).await;

    exec_tx.send(ExecutorCommand::Execute(outer_id)).await.unwrap();

    // Group cueのStartedは execute_cue 内で即座に(同期的に)発行される
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == outer_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == inner_id
    ));

    // child_a の再生開始をEngineが通知
    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == child_a_id));
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: child_a_id,
            position: 0.0,
            duration: 1.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == child_a_id
    ));

    // child_a 完了 -> inner内でchild_bへ連鎖。inner自身はまだCompletedにならない
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id: child_a_id,
        }))
        .await
        .unwrap();

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == child_b_id));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == child_a_id
    ));

    // child_b の開始
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: child_b_id,
            position: 0.0,
            duration: 1.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == child_b_id
    ));

    // child_b 完了 -> inner が最後の子として完了 -> outerのPlaylist上でsiblingへ連鎖
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id: child_b_id,
        }))
        .await
        .unwrap();

    // sibling が起動される (innerのCompleted解決による)
    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == sibling_id));

    // inner自身のCompletedが正しく1回だけ送られる
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == inner_id
    ));

    // child_bのCompletedも送られる
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == child_b_id
    ));

    // outer自身はまだ完了していないので、Completedイベントは来ない
    assert!(
        playback_event_rx.try_recv().is_err(),
        "outer group should not be completed while sibling is still playing"
    );
}

#[tokio::test]
async fn playlist_repeat_chain_reactivation_suppresses_group_completed() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let group_id = Uuid::new_v4();
    let child_a_id = Uuid::new_v4();
    let child_b_id = Uuid::new_v4();

    let cues = vec![
        make_playlist_group_cue(group_id, None, vec![child_a_id, child_b_id], true), // repeat: true
        make_audio_cue(child_a_id, Some(group_id), path.clone()),
        make_audio_cue(child_b_id, Some(group_id), path.clone()),
    ];

    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![group_id]).await;

    exec_tx.send(ExecutorCommand::Execute(group_id)).await.unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == group_id
    ));

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == child_a_id));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: child_a_id,
            position: 0.0,
            duration: 1.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == child_a_id
    ));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id: child_a_id,
        }))
        .await
        .unwrap();

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == child_b_id));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == child_a_id
    ));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: child_b_id,
            position: 0.0,
            duration: 1.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == child_b_id
    ));

    // child_b 完了 -> repeatによりchild_aが再実行される
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id: child_b_id,
        }))
        .await
        .unwrap();

    // child_a が再びPlayされる(連鎖によりresolve_after_complete_chainがcheck_and_stop_parentsより先に実行される)
    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == child_a_id));

    // child_bのCompletedは送られるが、グループのCompletedは送られてはいけない
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == child_b_id
    ));
    assert!(
        playback_event_rx.try_recv().is_err(),
        "group's Completed must not be sent when the repeat chain reactivates a child cue"
    );
}
