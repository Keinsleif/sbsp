// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::collections::HashMap;

use crate::{
    BackendSettings, event::BackendEvent, manager::DEFAULT_PROJECT_FOLDER_MODEL_FILENAME, model::{
        ShowModel,
        cue::{
            Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueList, CueParam,
            audio::{AudioCueParam, Decibels, SoundType},
        },
        settings::{CursorAdvanceTrigger, ShowSettings},
    },
};
use tempfile::{NamedTempFile, tempdir};
use tokio::sync::{broadcast, watch};
use uuid::Uuid;

use super::{
    ShowModelHandle, ShowModelManager, command::InsertPosition, project::ProjectStatus,
    project::ProjectType,
};

async fn setup_manager(
    initial_model: Option<ShowModel>,
    project_status: ProjectStatus,
) -> (ShowModelHandle, broadcast::Receiver<BackendEvent>) {
    let (event_tx, event_rx) = broadcast::channel::<BackendEvent>(32);
    let (_, settings_rx) = watch::channel(BackendSettings {
        copy_assets_when_add: true,
        ..Default::default()
    });
    let (model_manager, model_handle) = ShowModelManager::new(event_tx.clone(), settings_rx);
    if let Some(inital) = initial_model {
        let mut model_lock = model_manager.write().await;
        *model_lock = inital;
        drop(model_lock);
    }
    model_manager.set_project_status(project_status).await;
    tokio::spawn(model_manager.run());
    (model_handle, event_rx)
}

#[tokio::test]
async fn update_cue() {
    let temp_dir = tempdir().unwrap();
    let temp_target = NamedTempFile::with_suffix(".mp3").unwrap();
    let temp_target_after = NamedTempFile::with_suffix(".wav").unwrap();
    let cue_id = Uuid::new_v4();
    let (model_handle, mut event_rx) = setup_manager(
        Some(ShowModel {
            name: "test".into(),
            cue_list: CueList {
                cues: HashMap::from([(
                    cue_id,
                    Cue {
                        id: cue_id,
                        number: "1".into(),
                        name: Some("test cue".into()),
                        notes: "note".into(),
                        color: CueColor::None,
                        pre_wait: 0.0,
                        chain: CueChain::DoNotChain,
                        treat_stop_as_completed: false,
                        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
                        parent_id: None,
                        params: CueParam::Audio(AudioCueParam {
                            target: temp_target.path().to_path_buf(),
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
                    },
                )]),
                root_ids: vec![cue_id],
            },
            settings: ShowSettings::default(),
        }),
        ProjectStatus::Saved {
            project_type: ProjectType::ProjectFolder,
            path: temp_dir.path().to_path_buf().join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME),
        },
    )
    .await;

    let new_cue = Cue {
        id: cue_id,
        number: "1".into(),
        name: Some("test cue".into()),
        notes: "note".into(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: CueChain::DoNotChain,
        treat_stop_as_completed: true,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::Override(
            CursorAdvanceTrigger::OnCompleted,
        ),
        parent_id: None,
        params: CueParam::Audio(AudioCueParam {
            target: temp_target_after.path().to_path_buf(),
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
    };
    model_handle.update_cue(new_cue.clone()).await.unwrap();

    let estimated_audio_filename = temp_target_after
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let estimated_audio_target = temp_dir.path().join(".").join(estimated_audio_filename);
    let mut estimated_new_cue = new_cue.clone();
    if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
        audio_param.target = [".", estimated_audio_filename].iter().collect();
    }

    loop {
        if let Ok(BackendEvent::CueListUpdated { cue_list }) = event_rx.recv().await {
            assert_eq!(*cue_list.cues.get(&cue_id).unwrap(), estimated_new_cue);
            break;
        }
    }

    let model = model_handle.read().await;
    assert_eq!(
        *model.cue_list.cues.get(&cue_id).unwrap(),
        estimated_new_cue
    );
    assert!(estimated_audio_target.exists());
    drop(temp_target);
    drop(temp_dir);
}

#[tokio::test]
async fn add_cue() {
    let temp_dir = tempdir().unwrap();
    let temp_target = NamedTempFile::with_suffix(".mp3").unwrap();
    let cue_id = Uuid::new_v4();
    let (model_handle, mut event_rx) = setup_manager(
        Some(ShowModel {
            name: "test".into(),
            cue_list: CueList::default(),
            settings: ShowSettings::default(),
        }),
        ProjectStatus::Saved {
            project_type: ProjectType::ProjectFolder,
            path: temp_dir.path().to_path_buf().join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME),
        },
    )
    .await;

    let new_cue = Cue {
        id: cue_id,
        number: "1".into(),
        name: Some("test cue".into()),
        notes: "note".into(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: CueChain::DoNotChain,
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
        parent_id: None,
        params: CueParam::Audio(AudioCueParam {
            target: temp_target.path().to_path_buf(),
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
    };
    model_handle
        .add_cue(
            new_cue.clone(),
            InsertPosition::Inside {
                target: None,
                index: Some(0),
            },
        )
        .await
        .unwrap();

    let estimated_audio_filename = temp_target.path().file_name().unwrap().to_str().unwrap();
    let estimated_audio_target = temp_dir.path().join(".").join(estimated_audio_filename);
    let mut estimated_new_cue = new_cue.clone();
    if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
        audio_param.target = [".", estimated_audio_filename].iter().collect();
    }

    loop {
        if let Ok(BackendEvent::CueListUpdated { cue_list }) = event_rx.recv().await {
            assert_eq!(*cue_list.cues.get(&cue_id).unwrap(), estimated_new_cue);
            break;
        }
    }

    let model = model_handle.read().await;
    assert_eq!(
        *model.cue_list.cues.get(&cue_id).unwrap(),
        estimated_new_cue
    );
    assert!(estimated_audio_target.exists());
    drop(temp_target);
    drop(temp_dir);
}
