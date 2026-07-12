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
            audio::{Decibels, Easing, EnvelopeSegment, FadeParam, SoundType},
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

    (
        manager,
        exec_tx,
        audio_rx,
        engine_event_tx,
        playback_event_rx,
    )
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

fn make_start_cue(id: Uuid, parent_id: Option<Uuid>, target: Uuid) -> Cue {
    Cue {
        id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        parent_id,
        params: model::cue::CueParam::Start(model::cue::StartCueParam { target }),
    }
}

fn make_stop_cue(id: Uuid, parent_id: Option<Uuid>, target: Uuid, hard: bool) -> Cue {
    Cue {
        id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        parent_id,
        params: model::cue::CueParam::Stop(model::cue::StopCueParam { target, hard }),
    }
}

fn make_pause_cue(id: Uuid, parent_id: Option<Uuid>, target: Uuid) -> Cue {
    Cue {
        id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        parent_id,
        params: model::cue::CueParam::Pause(model::cue::PauseCueParam { target }),
    }
}

fn make_load_cue(id: Uuid, parent_id: Option<Uuid>, target: Uuid) -> Cue {
    Cue {
        id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        parent_id,
        params: model::cue::CueParam::Load(model::cue::LoadCueParam { target }),
    }
}

fn with_pre_wait(mut cue: Cue, pre_wait: f64) -> Cue {
    cue.pre_wait = pre_wait;
    cue
}

fn make_concurrency_group_cue(id: Uuid, parent_id: Option<Uuid>, children: Vec<Uuid>) -> Cue {
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
            base: model::cue::group::GroupCueParamBase {
                mode: GroupMode::Concurrency,
            },
            children,
        },
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
    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let cue_id = Uuid::new_v4();
    let cue = Cue {
        id: cue_id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        parent_id: None,
        params: model::cue::CueParam::Audio(AudioCueParam {
            target: path.clone(),
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
            volume: Decibels::from(-10.0),
            pan: 0.5,
            repeat: false,
            sound_type: SoundType::Streaming,
            envelope: vec![EnvelopeSegment {
                start: 0.2,
                end: 5.0,
                volume: Decibels::from(-2.0),
            }],
        }),
    };

    let (_, exec_tx, mut audio_rx, _engine_event_tx, _exec_event_rx) =
        setup_executor_with_cues(vec![cue], vec![cue_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(cue_id))
        .await
        .unwrap();

    if let Some(AudioCommand::Play { id, data }) = audio_rx.recv().await {
        assert_eq!(id, cue_id);
        assert_eq!(data.filepath, temp_target.path().to_path_buf());
        assert_eq!(data.volume, Decibels::from(-10.0));
        assert_eq!(data.pan, 0.5);
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
        assert_eq!(data.sound_type, SoundType::Streaming);
        assert_eq!(
            data.envelope,
            vec![EnvelopeSegment {
                start: 0.2,
                end: 5.0,
                volume: Decibels::from(-2.0)
            }]
        );
    } else {
        panic!();
    }
}

#[tokio::test]
async fn started_event() {
    let temp_target: NamedTempFile = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let orig_cue_id = Uuid::new_v4();
    let cue = make_audio_cue(orig_cue_id, None, path);

    let (_model_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(vec![cue], vec![orig_cue_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(orig_cue_id))
        .await
        .unwrap();

    if let Some(ExecutorEvent::Triggered { cue_id }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
    } else {
        panic!();
    }

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == orig_cue_id));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: orig_cue_id,
            position: 0.0,
            duration: 23.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();

    if let Some(ExecutorEvent::Started {
        cue_id,
        position,
        duration,
        initial_params,
    }) = playback_event_rx.recv().await
    {
        assert_eq!(cue_id, orig_cue_id);
        assert_eq!(position, 0.0);
        assert_eq!(duration, 23.0);
        assert_eq!(
            initial_params,
            StateParam::Audio(AudioStateParam::default())
        );
    } else {
        panic!();
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

    if let Some(ExecutorEvent::Triggered { cue_id }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
    } else {
        panic!();
    }

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == orig_cue_id));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Progress {
            instance_id: orig_cue_id,
            position: 20.0,
            duration: 50.0,
        }))
        .await
        .unwrap();

    if let Some(ExecutorEvent::Progress {
        cue_id,
        position,
        duration,
    }) = playback_event_rx.recv().await
    {
        assert_eq!(cue_id, orig_cue_id);
        assert_eq!(position, 20.0);
        assert_eq!(duration, 50.0);
    } else {
        panic!();
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

    if let Some(ExecutorEvent::Triggered { cue_id }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
    } else {
        panic!();
    }

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == orig_cue_id));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Paused {
            instance_id: orig_cue_id,
            position: 24.0,
            duration: 50.0,
        }))
        .await
        .unwrap();

    if let Some(ExecutorEvent::Paused {
        cue_id,
        position,
        duration,
    }) = playback_event_rx.recv().await
    {
        assert_eq!(cue_id, orig_cue_id);
        assert_eq!(position, 24.0);
        assert_eq!(duration, 50.0);
    } else {
        panic!();
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

    if let Some(ExecutorEvent::Triggered { cue_id }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
    } else {
        panic!();
    }

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == orig_cue_id));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Resumed {
            instance_id: orig_cue_id,
        }))
        .await
        .unwrap();

    if let Some(ExecutorEvent::Resumed { cue_id }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
    } else {
        panic!();
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

    if let Some(ExecutorEvent::Triggered { cue_id }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
    } else {
        panic!();
    }

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == orig_cue_id));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id: orig_cue_id,
        }))
        .await
        .unwrap();

    if let Some(ExecutorEvent::Completed { cue_id }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
    } else {
        panic!();
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

    if let Some(ExecutorEvent::Triggered { cue_id }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
    } else {
        panic!();
    }

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == orig_cue_id));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Error {
            instance_id: orig_cue_id,
            error: "Error".to_string(),
        }))
        .await
        .unwrap();

    if let Some(ExecutorEvent::Error { cue_id, error }) = playback_event_rx.recv().await {
        assert_eq!(cue_id, orig_cue_id);
        assert_eq!(error, "Error".to_string());
    } else {
        panic!();
    }
}

#[tokio::test]
async fn start_cue_executes_inactive_target() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let target_id = Uuid::new_v4();
    let start_id = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(target_id, None, path),
        make_start_cue(start_id, None, target_id),
    ];

    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![target_id, start_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(start_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == start_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == target_id
    ));

    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == target_id));

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == start_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == start_id
    ));
}

#[tokio::test]
async fn start_cue_resumes_paused_target() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let target_id = Uuid::new_v4();
    let start_id = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(target_id, None, path),
        make_start_cue(start_id, None, target_id),
    ];

    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![target_id, start_id]).await;

    // targetを先に再生・一時停止状態にする
    exec_tx
        .send(ExecutorCommand::Execute(target_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == target_id
    ));

    assert!(
        matches!(audio_rx.recv().await.unwrap(), AudioCommand::Play { id, .. } if id == target_id)
    );

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: target_id,
            position: 0.0,
            duration: 1.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == target_id
    ));
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Paused {
            instance_id: target_id,
            position: 0.5,
            duration: 1.0,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Paused { cue_id, .. } if cue_id == target_id
    ));

    // Startキューでtargetを再開させる
    exec_tx
        .send(ExecutorCommand::Execute(start_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == start_id
    ));

    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Resume { id } if id == target_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == start_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == start_id
    ));
}

#[tokio::test]
async fn stop_cue_stops_active_target() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let target_id = Uuid::new_v4();
    let stop_id = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(target_id, None, path),
        make_stop_cue(stop_id, None, target_id, false),
    ];

    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![target_id, stop_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(target_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == target_id
    ));

    assert!(
        matches!(audio_rx.recv().await.unwrap(), AudioCommand::Play { id, .. } if id == target_id)
    );

    exec_tx
        .send(ExecutorCommand::Execute(stop_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == stop_id
    ));

    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::SoftStop { id } if id == target_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == stop_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == stop_id
    ));
}

#[tokio::test]
async fn pause_cue_pauses_active_target() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let target_id = Uuid::new_v4();
    let pause_id = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(target_id, None, path),
        make_pause_cue(pause_id, None, target_id),
    ];

    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![target_id, pause_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(target_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == target_id
    ));
    assert!(
        matches!(audio_rx.recv().await.unwrap(), AudioCommand::Play { id, .. } if id == target_id)
    );

    exec_tx
        .send(ExecutorCommand::Execute(pause_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == pause_id
    ));

    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Pause { id } if id == target_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == pause_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == pause_id
    ));
}

#[tokio::test]
async fn load_cue_loads_unloaded_target() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let target_id = Uuid::new_v4();
    let load_id = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(target_id, None, path),
        make_load_cue(load_id, None, target_id),
    ];

    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![target_id, load_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(load_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == load_id
    ));
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Load { id, .. } if id == target_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == load_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == load_id
    ));
}

#[tokio::test]
async fn pause_cue_noop_when_target_already_paused() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let target_id = Uuid::new_v4();
    let pause_id = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(target_id, None, path),
        make_pause_cue(pause_id, None, target_id),
    ];

    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![target_id, pause_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(target_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == target_id
    ));
    assert!(
        matches!(audio_rx.recv().await.unwrap(), AudioCommand::Play { id, .. } if id == target_id)
    );
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: target_id,
            position: 0.0,
            duration: 1.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == target_id
    ));
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Paused {
            instance_id: target_id,
            position: 0.5,
            duration: 1.0,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Paused { cue_id, .. } if cue_id == target_id
    ));

    exec_tx
        .send(ExecutorCommand::Execute(pause_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == pause_id
    ));

    // targetへのAudioCommandは送られない。PauseキューのStarted+Completedのみ。
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == pause_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == pause_id
    ));
    assert!(
        audio_rx.try_recv().is_err(),
        "no AudioCommand should be sent to an already-paused target"
    );
}

#[tokio::test]
async fn playback_cue_as_last_group_child_completes_group() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let group_id = Uuid::new_v4();
    let load_cue_id = Uuid::new_v4();
    let target_id = Uuid::new_v4(); // Groupの外にある、Loadの対象

    let cues = vec![
        make_playlist_group_cue(group_id, None, vec![load_cue_id], false),
        make_load_cue(load_cue_id, Some(group_id), target_id),
        make_audio_cue(target_id, None, path),
    ];

    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![group_id, target_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(group_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == group_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == group_id
    ));

    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Load { id, .. } if id == target_id
    ));

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == load_cue_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == load_cue_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == load_cue_id
    ));

    // load_cueがGroup唯一の子だったため、直後にGroup自身も完了する
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == group_id
    ));
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
        make_playlist_group_cue(
            inner_id,
            Some(outer_id),
            vec![child_a_id, child_b_id],
            false,
        ),
        make_audio_cue(child_a_id, Some(inner_id), path.clone()),
        make_audio_cue(child_b_id, Some(inner_id), path.clone()),
        make_audio_cue(sibling_id, Some(outer_id), path.clone()),
    ];

    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![outer_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(outer_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == outer_id
    ));
    // Group cueのStartedは execute_cue 内で即座に(同期的に)発行される
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == outer_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == inner_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == inner_id
    ));

    // child_a の再生開始をEngineが通知
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == child_a_id
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
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == child_b_id
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

    // child_b 完了 -> inner が最後の子として完了 -> outerのPlaylist上でsiblingへ連鎖
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id: child_b_id,
        }))
        .await
        .unwrap();

    // child_bのCompletedも送られる
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == child_b_id
    ));

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == sibling_id
    ));
    // sibling が起動される (innerのCompleted解決による)
    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == sibling_id));

    // inner自身のCompletedが正しく1回だけ送られる
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == inner_id
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

    exec_tx
        .send(ExecutorCommand::Execute(group_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == group_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == group_id
    ));

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == child_a_id
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

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == child_a_id
    ));

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == child_b_id
    ));
    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == child_b_id));

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
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == child_a_id
    ));
    assert!(
        playback_event_rx.try_recv().is_err(),
        "group's Completed must not be sent when the repeat chain reactivates a child cue"
    );
}

#[tokio::test]
async fn pre_wait_defers_started_until_actual_start() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();
    let cue_id = Uuid::new_v4();

    let cue = with_pre_wait(make_audio_cue(cue_id, None, path), 2.0);
    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(vec![cue], vec![cue_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(cue_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == cue_id
    ));
    // ロードは即座に行われる
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Load { id, .. } if id == cue_id
    ));

    // PreWaitタイマー開始の通知
    engine_event_tx
        .send(EngineEvent::PreWait(WaitEvent::Started {
            instance_id: cue_id,
            position: 0.0,
            duration: 2.0,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::PreWaitStarted { cue_id: id, duration } if id == cue_id && duration == 2.0
    ));

    // まだ本編のStartedは来ていない
    assert!(playback_event_rx.try_recv().is_err());

    // PreWait完了 -> PreWaitCompleted、続けて本編再生が開始される
    engine_event_tx
        .send(EngineEvent::PreWait(WaitEvent::Completed {
            instance_id: cue_id,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::PreWaitCompleted { cue_id: id } if id == cue_id
    ));
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Play { id, .. } if id == cue_id
    ));

    // Engineからの実際のStartedで初めてExecutorEvent::Startedが発行される
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: cue_id,
            position: 0.0,
            duration: 10.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id: id, .. } if id == cue_id
    ));
}

#[tokio::test]
async fn stop_during_prewait_audio_cue_emits_stopped_once() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();
    let cue_id = Uuid::new_v4();

    let cue = with_pre_wait(make_audio_cue(cue_id, None, path), 5.0);
    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(vec![cue], vec![cue_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(cue_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == cue_id
    ));
    assert!(
        matches!(audio_rx.recv().await.unwrap(), AudioCommand::Load { id, .. } if id == cue_id)
    );

    engine_event_tx
        .send(EngineEvent::PreWait(WaitEvent::Started {
            instance_id: cue_id,
            position: 0.0,
            duration: 5.0,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::PreWaitStarted { cue_id: id, .. } if id == cue_id
    ));

    exec_tx
        .send(ExecutorCommand::Stop(cue_id, StopMode::Soft))
        .await
        .unwrap();

    // ロード済みリソースの停止コマンドが送られる
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::HardStop { id } if id == cue_id
    ));

    // AudioEngineからの応答 -> emit_stoppedが発火(親への通知含む)
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Stopped {
            instance_id: cue_id,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Stopped { cue_id: id } if id == cue_id
    ));

    // PreWaitタイマー自体の停止応答も届くが、既にactive_instancesから消えているため
    // 重複イベントは発行されない
    engine_event_tx
        .send(EngineEvent::PreWait(WaitEvent::Stopped {
            instance_id: cue_id,
        }))
        .await
        .unwrap();
    assert!(
        playback_event_rx.try_recv().is_err(),
        "no duplicate Stopped event should be emitted for the same cue"
    );
}

#[tokio::test]
async fn stop_during_prewait_transport_cue_waits_for_prewait_engine_response() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let target_id = Uuid::new_v4();
    let load_id = Uuid::new_v4();

    let load_cue = with_pre_wait(make_load_cue(load_id, None, target_id), 3.0);
    let cues = vec![load_cue, make_audio_cue(target_id, None, path)];

    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![load_id, target_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(load_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == load_id
    ));

    engine_event_tx
        .send(EngineEvent::PreWait(WaitEvent::Started {
            instance_id: load_id,
            position: 0.0,
            duration: 3.0,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::PreWaitStarted { cue_id, .. } if cue_id == load_id
    ));

    exec_tx
        .send(ExecutorCommand::Stop(load_id, StopMode::Soft))
        .await
        .unwrap();

    // Playback型はエンジンへ何も送らないため、この時点ではまだ何も発行されない
    assert!(playback_event_rx.try_recv().is_err());
    assert!(audio_rx.try_recv().is_err());

    // PreWaitタイマーの停止応答が届いて初めてStoppedが発行される
    engine_event_tx
        .send(EngineEvent::PreWait(WaitEvent::Stopped {
            instance_id: load_id,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Stopped { cue_id } if cue_id == load_id
    ));
}

#[tokio::test]
async fn stop_prewaiting_child_does_not_affect_active_sibling_group() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let group_id = Uuid::new_v4();
    let sibling_id = Uuid::new_v4();
    let prewait_child_id = Uuid::new_v4();

    let cues = vec![
        make_concurrency_group_cue(group_id, None, vec![sibling_id, prewait_child_id]),
        make_audio_cue(sibling_id, Some(group_id), path.clone()),
        with_pre_wait(make_audio_cue(prewait_child_id, Some(group_id), path), 5.0),
    ];

    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![group_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(group_id))
        .await
        .unwrap();

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == group_id
    ));
    assert!(
        matches!(audio_rx.recv().await.unwrap(), AudioCommand::Play { id, .. } if id == sibling_id)
    );
    assert!(
        matches!(audio_rx.recv().await.unwrap(), AudioCommand::Load { id, .. } if id == prewait_child_id)
    );

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: sibling_id,
            position: 0.0,
            duration: 10.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == group_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == sibling_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == prewait_child_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == sibling_id
    ));

    engine_event_tx
        .send(EngineEvent::PreWait(WaitEvent::Started {
            instance_id: prewait_child_id,
            position: 0.0,
            duration: 5.0,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::PreWaitStarted { cue_id, .. } if cue_id == prewait_child_id
    ));

    exec_tx
        .send(ExecutorCommand::Stop(prewait_child_id, StopMode::Soft))
        .await
        .unwrap();
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::HardStop { id } if id == prewait_child_id
    ));
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Stopped {
            instance_id: prewait_child_id,
        }))
        .await
        .unwrap();

    // prewait_child自身のStoppedは発行されるが、
    // siblingがまだ再生中なのでGroup自身はStopped/Completedにならない
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Stopped { cue_id } if cue_id == prewait_child_id
    ));
    assert!(
        playback_event_rx.try_recv().is_err(),
        "group must remain active while sibling is still playing"
    );

    // PreWaitタイマー自体の停止応答が来ても重複イベントは発行されない
    engine_event_tx
        .send(EngineEvent::PreWait(WaitEvent::Stopped {
            instance_id: prewait_child_id,
        }))
        .await
        .unwrap();
    assert!(playback_event_rx.try_recv().is_err());
}
