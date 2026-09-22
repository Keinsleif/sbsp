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
    action::AudioAction,
    controller::state::AudioStateParam,
    engine::audio_engine::{AudioCommand, AudioEngineEvent},
    event::BackendEvent,
    manager::ShowModelManager,
    model::{
        self,
        cue::{
            CueColor, CueCursorAdvanceTriggerOverride,
            audio::{Decibels, Easing, EnvelopeSegment, FadeParam, SoundType},
        },
        settings::ShowSettings,
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
    let (wait_tx, mut wait_rx) = mpsc::channel::<WaitCommand>(32);
    tokio::spawn(async move { while wait_rx.recv().await.is_some() {} });
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
            treat_stop_as_completed: false,
            cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
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
    let (wait_tx, mut wait_rx) = mpsc::channel::<WaitCommand>(32);
    tokio::spawn(async move { while wait_rx.recv().await.is_some() {} });
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

async fn setup_executor_with_cues_and_wait_rx(
    cues: Vec<Cue>,
    root_ids: Vec<Uuid>,
) -> (
    ShowModelManager,
    Sender<ExecutorCommand>,
    Receiver<AudioCommand>,
    Receiver<WaitCommand>,
    Sender<EngineEvent>,
    Receiver<ExecutorEvent>,
) {
    let (exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(32);
    let (audio_tx, audio_rx) = mpsc::channel::<AudioCommand>(32);
    let (wait_tx, wait_rx) = mpsc::channel::<WaitCommand>(32);
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
        wait_rx,
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
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
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

fn make_fade_cue(
    id: Uuid,
    parent_id: Option<Uuid>,
    target: Uuid,
    volume: Decibels,
    fade_param: FadeParam,
) -> Cue {
    Cue {
        id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
        parent_id,
        params: model::cue::CueParam::Fade(model::cue::FadeCueParam {
            target,
            volume,
            fade_param,
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
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
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
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
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
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
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
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
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
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
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
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
        parent_id,
        params: model::cue::CueParam::Group {
            base: crate::model::cue::group::GroupCueParamBase {
                mode: GroupMode::Playlist { repeat },
            },
            children,
        },
    }
}

fn make_start_first_group_cue(id: Uuid, parent_id: Option<Uuid>, children: Vec<Uuid>) -> Cue {
    Cue {
        id,
        number: "1".to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: model::cue::CueChain::DoNotChain,
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
        parent_id,
        params: model::cue::CueParam::Group {
            base: model::cue::group::GroupCueParamBase {
                mode: GroupMode::StartFirst { enter: true },
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
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
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
        ExecutorEvent::Started { cue_id, .. } if cue_id == start_id
    ));

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == target_id
    ));
    let cmd = audio_rx.recv().await.unwrap();
    assert!(matches!(cmd, AudioCommand::Play { id, .. } if id == target_id));
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
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == load_cue_id
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == load_cue_id
    ));
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Load { id, .. } if id == target_id
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

    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == child_a_id
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
        ExecutorEvent::Triggered { cue_id: id } if id == cue_id
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
        ExecutorEvent::Triggered { cue_id: id } if id == cue_id
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
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == group_id
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

#[tokio::test]
async fn self_referencing_group_child_is_detected_as_cycle() {
    let group_id = Uuid::new_v4();
    let cues = vec![make_playlist_group_cue(
        group_id,
        None,
        vec![group_id],
        false,
    )];

    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![group_id]).await;

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        exec_tx.send(ExecutorCommand::Execute(group_id)),
    )
    .await;
    assert!(result.is_ok(), "send should not hang");
    result.unwrap().unwrap();

    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(3), playback_event_rx.recv())
            .await
            .expect("must not hang")
            .unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == group_id
    ));
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(3), playback_event_rx.recv())
            .await
            .expect("must not hang")
            .unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == group_id
    ));

    exec_tx
        .send(ExecutorCommand::ReconfigureEngines(Default::default()))
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(3), audio_rx.recv())
            .await
            .expect("must not hang")
            .unwrap(),
        AudioCommand::Reconfigure(_)
    ));
    assert!(playback_event_rx.try_recv().is_err());
    assert!(audio_rx.try_recv().is_err());
}

#[tokio::test]
async fn mutually_referencing_groups_do_not_deadlock() {
    let group_a_id = Uuid::new_v4();
    let group_b_id = Uuid::new_v4();

    let cues = vec![
        make_playlist_group_cue(group_a_id, None, vec![group_b_id], false),
        make_playlist_group_cue(group_b_id, Some(group_a_id), vec![group_a_id], false),
    ];

    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![group_a_id]).await;

    let send_result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        exec_tx.send(ExecutorCommand::Execute(group_a_id)),
    )
    .await;
    assert!(send_result.is_ok(), "send should not hang");
    send_result.unwrap().unwrap();

    for expected_id in [group_a_id, group_b_id] {
        assert!(matches!(
            tokio::time::timeout(std::time::Duration::from_secs(3), playback_event_rx.recv())
                .await
                .expect("must not hang")
                .unwrap(),
            ExecutorEvent::Triggered { cue_id } if cue_id == expected_id
        ));
        assert!(matches!(
            tokio::time::timeout(std::time::Duration::from_secs(3), playback_event_rx.recv())
                .await
                .expect("must not hang")
                .unwrap(),
            ExecutorEvent::Started { cue_id, .. } if cue_id == expected_id
        ));
    }

    exec_tx
        .send(ExecutorCommand::ReconfigureEngines(Default::default()))
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(3), audio_rx.recv())
            .await
            .expect("must not hang")
            .unwrap(),
        AudioCommand::Reconfigure(_)
    ));
    assert!(playback_event_rx.try_recv().is_err());
    assert!(audio_rx.try_recv().is_err());
}

#[tokio::test]
async fn chain_trigger_history_blocks_after_max_in_window() {
    let (event_tx, _) = broadcast::channel::<BackendEvent>(1);
    let (_, settings_rx) = watch::channel(BackendSettings::default());
    let (_manager, handle) = ShowModelManager::new(event_tx, settings_rx);
    let (_exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(1);
    let (audio_tx, _audio_rx) = mpsc::channel::<AudioCommand>(1);
    let (wait_tx, _wait_rx) = mpsc::channel::<WaitCommand>(1);
    let (playback_event_tx, _playback_event_rx) = mpsc::channel::<ExecutorEvent>(1);
    let (_engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(1);

    let mut executor = Executor::new(
        handle,
        exec_rx,
        audio_tx,
        wait_tx,
        playback_event_tx,
        engine_event_rx,
    );

    let target = Uuid::new_v4();

    // MAX_TRIGGERS_IN_WINDOW = 8 回までは許可される
    for i in 0..8 {
        assert!(
            executor.record_and_check_chain_trigger(target),
            "call #{} should still be allowed",
            i + 1
        );
    }

    // 9回目でウィンドウ内の上限を超え、拒否される
    assert!(
        !executor.record_and_check_chain_trigger(target),
        "9th call within the window should be rejected"
    );

    // 拒否された後も履歴には積まれ続けるため、直後の呼び出しも拒否されたままになる
    assert!(!executor.record_and_check_chain_trigger(target));

    // 別の cue_id は独立したカウンタを持つので影響を受けない
    let other_target = Uuid::new_v4();
    assert!(executor.record_and_check_chain_trigger(other_target));
}

#[tokio::test]
async fn chain_trigger_history_allows_again_after_window_elapses() {
    let (event_tx, _) = broadcast::channel::<BackendEvent>(1);
    let (_, settings_rx) = watch::channel(BackendSettings::default());
    let (_manager, handle) = ShowModelManager::new(event_tx, settings_rx);
    let (_exec_tx, exec_rx) = mpsc::channel::<ExecutorCommand>(1);
    let (audio_tx, _audio_rx) = mpsc::channel::<AudioCommand>(1);
    let (wait_tx, _wait_rx) = mpsc::channel::<WaitCommand>(1);
    let (playback_event_tx, _playback_event_rx) = mpsc::channel::<ExecutorEvent>(1);
    let (_engine_event_tx, engine_event_rx) = mpsc::channel::<EngineEvent>(1);

    let mut executor = Executor::new(
        handle,
        exec_rx,
        audio_tx,
        wait_tx,
        playback_event_tx,
        engine_event_rx,
    );

    let target = Uuid::new_v4();
    for _ in 0..8 {
        assert!(executor.record_and_check_chain_trigger(target));
    }
    assert!(!executor.record_and_check_chain_trigger(target));

    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;

    assert!(
        executor.record_and_check_chain_trigger(target),
        "after the window elapses, triggering should be allowed again"
    );
}

#[tokio::test]
async fn start_first_group_loads_and_executes_only_first_child() {
    let temp_target_a = NamedTempFile::with_suffix(".flac").unwrap();
    let temp_target_b = NamedTempFile::with_suffix(".flac").unwrap();
    let path_a = temp_target_a.path().to_path_buf();
    let path_b = temp_target_b.path().to_path_buf();

    let group_id = Uuid::new_v4();
    let child_a = Uuid::new_v4();
    let child_b = Uuid::new_v4();

    let cues = vec![
        make_start_first_group_cue(group_id, None, vec![child_a, child_b]),
        make_audio_cue(child_a, Some(group_id), path_a),
        make_audio_cue(child_b, Some(group_id), path_b),
    ];

    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
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
        ExecutorEvent::Triggered { cue_id } if cue_id == child_a
    ));

    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Play { id, .. } if id == child_a
    ));

    exec_tx
        .send(ExecutorCommand::ReconfigureEngines(Default::default()))
        .await
        .unwrap();
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Reconfigure(_)
    ));
    assert!(audio_rx.try_recv().is_err());
    assert!(playback_event_rx.try_recv().is_err());
}

#[tokio::test]
async fn concurrency_group_starts_all_children_and_completes_only_after_all_finish() {
    let temp_target_a = NamedTempFile::with_suffix(".flac").unwrap();
    let temp_target_b = NamedTempFile::with_suffix(".flac").unwrap();
    let path_a = temp_target_a.path().to_path_buf();
    let path_b = temp_target_b.path().to_path_buf();

    let group_id = Uuid::new_v4();
    let child_a = Uuid::new_v4();
    let child_b = Uuid::new_v4();

    let cues = vec![
        make_concurrency_group_cue(group_id, None, vec![child_a, child_b]),
        make_audio_cue(child_a, Some(group_id), path_a),
        make_audio_cue(child_b, Some(group_id), path_b),
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

    let mut triggered = std::collections::HashSet::new();
    for _ in 0..2 {
        if let ExecutorEvent::Triggered { cue_id } = playback_event_rx.recv().await.unwrap() {
            triggered.insert(cue_id);
        } else {
            panic!("expected Triggered event");
        }
    }
    assert_eq!(triggered, [child_a, child_b].into_iter().collect());

    let mut played = std::collections::HashSet::new();
    for _ in 0..2 {
        if let AudioCommand::Play { id, .. } = audio_rx.recv().await.unwrap() {
            played.insert(id);
        } else {
            panic!("expected AudioCommand::Play");
        }
    }
    assert_eq!(played, [child_a, child_b].into_iter().collect());

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: child_a,
            position: 0.0,
            duration: 10.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == child_a
    ));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: child_b,
            position: 0.0,
            duration: 10.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == child_b
    ));

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id: child_a,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == child_a
    ));
    assert!(
        playback_event_rx.try_recv().is_err(),
        "group must stay active while child_b is still running"
    );

    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Completed {
            instance_id: child_b,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == child_b
    ));
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == group_id
    ));
}

#[tokio::test]
async fn fade_cue_sends_fade_volume_to_active_audio_target() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();

    let target_id = Uuid::new_v4();
    let fade_id = Uuid::new_v4();
    let fade_param = FadeParam {
        duration: 1.5,
        easing: Easing::Linear,
    };

    let cues = vec![
        make_audio_cue(target_id, None, path),
        make_fade_cue(fade_id, None, target_id, Decibels::from(-6.0), fade_param),
    ];

    let (_manager, exec_tx, mut audio_rx, mut wait_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues_and_wait_rx(cues, vec![target_id, fade_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(target_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == target_id
    ));
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Play { id, .. } if id == target_id
    ));
    engine_event_tx
        .send(EngineEvent::Audio(AudioEngineEvent::Started {
            instance_id: target_id,
            position: 0.0,
            duration: 10.0,
            initial_params: AudioStateParam::default(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == target_id
    ));

    exec_tx
        .send(ExecutorCommand::Execute(fade_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == fade_id
    ));

    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::FadeVolume { id, volume, fade_param: fp }
            if id == target_id && volume == Decibels::from(-6.0) && fp == fade_param
    ));

    assert!(matches!(
        wait_rx.recv().await.unwrap(),
        WaitCommand::Start { wait_type: WaitType::FadeWait, instance_id, duration }
            if instance_id == fade_id && duration == fade_param.duration
    ));

    engine_event_tx
        .send(EngineEvent::Fade(WaitEvent::Started {
            instance_id: fade_id,
            position: 0.0,
            duration: fade_param.duration,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Started { cue_id, .. } if cue_id == fade_id
    ));

    engine_event_tx
        .send(EngineEvent::Fade(WaitEvent::Completed {
            instance_id: fade_id,
        }))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Completed { cue_id } if cue_id == fade_id
    ));
}

#[tokio::test]
async fn fade_cue_targeting_group_fades_all_descendant_audio_children() {
    let temp_target_a = NamedTempFile::with_suffix(".flac").unwrap();
    let temp_target_b = NamedTempFile::with_suffix(".flac").unwrap();
    let path_a = temp_target_a.path().to_path_buf();
    let path_b = temp_target_b.path().to_path_buf();

    let group_id = Uuid::new_v4();
    let child_a = Uuid::new_v4();
    let child_b = Uuid::new_v4();
    let fade_id = Uuid::new_v4();
    let fade_param = FadeParam {
        duration: 2.0,
        easing: Easing::Linear,
    };

    let cues = vec![
        make_concurrency_group_cue(group_id, None, vec![child_a, child_b]),
        make_audio_cue(child_a, Some(group_id), path_a),
        make_audio_cue(child_b, Some(group_id), path_b),
        make_fade_cue(fade_id, None, group_id, Decibels::from(-12.0), fade_param),
    ];

    let (_manager, exec_tx, mut audio_rx, _wait_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues_and_wait_rx(cues, vec![group_id, fade_id]).await;

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
    for _ in 0..2 {
        assert!(matches!(
            playback_event_rx.recv().await.unwrap(),
            ExecutorEvent::Triggered { .. }
        ));
    }
    let mut played = std::collections::HashSet::new();
    for _ in 0..2 {
        if let AudioCommand::Play { id, .. } = audio_rx.recv().await.unwrap() {
            played.insert(id);
        }
    }
    assert_eq!(played, [child_a, child_b].into_iter().collect());

    exec_tx
        .send(ExecutorCommand::Execute(fade_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id } if cue_id == fade_id
    ));

    let mut faded = std::collections::HashSet::new();
    for _ in 0..2 {
        if let AudioCommand::FadeVolume { id, volume, .. } = audio_rx.recv().await.unwrap() {
            assert_eq!(volume, Decibels::from(-12.0));
            faded.insert(id);
        } else {
            panic!("expected AudioCommand::FadeVolume");
        }
    }
    assert_eq!(faded, [child_a, child_b].into_iter().collect());
}

#[tokio::test]
async fn seek_to_sends_audio_command_for_active_audio_cue() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();
    let cue_id = Uuid::new_v4();

    let cues = vec![make_audio_cue(cue_id, None, path)];
    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![cue_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(cue_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id: id } if id == cue_id
    ));
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Play { id, .. } if id == cue_id
    ));
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

    exec_tx
        .send(ExecutorCommand::SeekTo(cue_id, 12.5))
        .await
        .unwrap();
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::SeekTo { id, position } if id == cue_id && position == 12.5
    ));
}

#[tokio::test]
async fn seek_by_sends_audio_command_for_active_audio_cue() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();
    let cue_id = Uuid::new_v4();

    let cues = vec![make_audio_cue(cue_id, None, path)];
    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![cue_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(cue_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id: id } if id == cue_id
    ));
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Play { id, .. } if id == cue_id
    ));
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

    exec_tx
        .send(ExecutorCommand::SeekBy(cue_id, 3.0))
        .await
        .unwrap();
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::SeekBy { id, amount } if id == cue_id && amount == 3.0
    ));
}

#[tokio::test]
async fn seek_to_is_noop_for_active_group_cue() {
    let temp_target_a = NamedTempFile::with_suffix(".flac").unwrap();
    let temp_target_b = NamedTempFile::with_suffix(".flac").unwrap();
    let path_a = temp_target_a.path().to_path_buf();
    let path_b = temp_target_b.path().to_path_buf();

    let group_id = Uuid::new_v4();
    let child_a = Uuid::new_v4();
    let child_b = Uuid::new_v4();

    let cues = vec![
        make_concurrency_group_cue(group_id, None, vec![child_a, child_b]),
        make_audio_cue(child_a, Some(group_id), path_a),
        make_audio_cue(child_b, Some(group_id), path_b),
    ];
    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
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
    for _ in 0..2 {
        assert!(matches!(
            playback_event_rx.recv().await.unwrap(),
            ExecutorEvent::Triggered { .. }
        ));
    }
    for _ in 0..2 {
        assert!(matches!(
            audio_rx.recv().await.unwrap(),
            AudioCommand::Play { .. }
        ));
    }

    exec_tx
        .send(ExecutorCommand::SeekTo(group_id, 5.0))
        .await
        .unwrap();

    exec_tx
        .send(ExecutorCommand::ReconfigureEngines(Default::default()))
        .await
        .unwrap();
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Reconfigure(_)
    ));
    assert!(audio_rx.try_recv().is_err());
    assert!(playback_event_rx.try_recv().is_err());
}

#[tokio::test]
async fn perform_action_forwards_to_audio_engine_when_types_match() {
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();
    let cue_id = Uuid::new_v4();

    let cues = vec![make_audio_cue(cue_id, None, path)];
    let (_manager, exec_tx, mut audio_rx, engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![cue_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(cue_id))
        .await
        .unwrap();
    assert!(matches!(
        playback_event_rx.recv().await.unwrap(),
        ExecutorEvent::Triggered { cue_id: id } if id == cue_id
    ));
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Play { id, .. } if id == cue_id
    ));
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

    let action = crate::action::CueAction::Audio(AudioAction::ToggleRepeat);

    exec_tx
        .send(ExecutorCommand::PerformAction(cue_id, action))
        .await
        .unwrap();

    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::PerformAction { id, action } if id == cue_id && matches!(action, AudioAction::ToggleRepeat)
    ));
}

#[tokio::test]
async fn perform_action_is_ignored_when_action_type_does_not_match_active_engine() {
    let cue_id = Uuid::new_v4();
    let cues = vec![make_start_cue(cue_id, None, Uuid::new_v4())];
    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, mut playback_event_rx) =
        setup_executor_with_cues(cues, vec![cue_id]).await;

    exec_tx
        .send(ExecutorCommand::Execute(cue_id))
        .await
        .unwrap();

    let action = crate::action::CueAction::Audio(AudioAction::ToggleRepeat);
    exec_tx
        .send(ExecutorCommand::PerformAction(cue_id, action))
        .await
        .unwrap();

    exec_tx
        .send(ExecutorCommand::ReconfigureEngines(Default::default()))
        .await
        .unwrap();
    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Reconfigure(_)
    ));
    assert!(audio_rx.try_recv().is_err());
    let _ = playback_event_rx.try_recv();
}

#[tokio::test]
async fn reconfigure_engines_forwards_audio_settings() {
    let cue_id = Uuid::new_v4();
    let temp_target = NamedTempFile::with_suffix(".flac").unwrap();
    let path = temp_target.path().to_path_buf();
    let cues = vec![make_audio_cue(cue_id, None, path)];
    let (_manager, exec_tx, mut audio_rx, _engine_event_tx, _playback_event_rx) =
        setup_executor_with_cues(cues, vec![cue_id]).await;

    let settings = ShowSettings::default();

    exec_tx
        .send(ExecutorCommand::ReconfigureEngines(Box::new(
            settings.clone(),
        )))
        .await
        .unwrap();

    assert!(matches!(
        audio_rx.recv().await.unwrap(),
        AudioCommand::Reconfigure(audio_settings) if audio_settings == settings.audio
    ));
}
