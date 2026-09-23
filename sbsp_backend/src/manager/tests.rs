// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Duration,
};

use crate::{
    BackendSettings,
    event::{BackendError, BackendEvent},
    manager::DEFAULT_PROJECT_FOLDER_MODEL_FILENAME,
    model::{
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

fn build_model(cues: Vec<Cue>, root_ids: Vec<Uuid>) -> ShowModel {
    ShowModel {
        name: "test".into(),
        cue_list: CueList {
            cues: cues.into_iter().map(|c| (c.id, c)).collect(),
            root_ids,
        },
        settings: ShowSettings::default(),
    }
}

fn make_audio_cue(id: Uuid, parent_id: Option<Uuid>, number: &str, target: PathBuf) -> Cue {
    Cue {
        id,
        number: number.to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: CueChain::DoNotChain,
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
        parent_id,
        params: CueParam::Audio(AudioCueParam {
            target,
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

fn make_group_cue(id: Uuid, parent_id: Option<Uuid>, number: &str, children: Vec<Uuid>) -> Cue {
    Cue {
        id,
        number: number.to_string(),
        name: None,
        notes: "".to_string(),
        color: CueColor::None,
        pre_wait: 0.0,
        chain: CueChain::DoNotChain,
        treat_stop_as_completed: false,
        cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::None,
        parent_id,
        params: CueParam::Group {
            base: crate::model::cue::group::GroupCueParamBase {
                mode: crate::model::cue::group::GroupMode::Concurrency,
            },
            children,
        },
    }
}

async fn recv_event_matching<F, T>(
    event_rx: &mut broadcast::Receiver<BackendEvent>,
    mut extract: F,
) -> T
where
    F: FnMut(&BackendEvent) -> Option<T>,
{
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    if let Some(v) = extract(&event) {
                        return v;
                    }
                }
                Err(e) => panic!("event channel closed unexpectedly: {e}"),
            }
        }
    })
    .await
    .expect("timed out waiting for expected event")
}

async fn recv_event_matching_result<F, T>(
    event_rx: &mut broadcast::Receiver<BackendEvent>,
    mut extract: F,
) -> anyhow::Result<T>
where
    F: FnMut(&BackendEvent) -> Option<anyhow::Result<T>>,
{
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    if let Some(v) = extract(&event) {
                        return v;
                    }
                }
                Err(e) => panic!("event channel closed unexpectedly: {e}"),
            }
        }
    })
    .await
    .expect("timed out waiting for expected event")
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
            path: temp_dir
                .path()
                .to_path_buf()
                .join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME),
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

    let estimated_audio_target = PathBuf::from(temp_target_after.path().file_name().unwrap());
    let mut estimated_new_cue = new_cue.clone();
    if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
        audio_param.target = estimated_audio_target.clone();
    }

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match event_rx.recv().await {
                Ok(BackendEvent::CueListUpdated { cue_list }) => {
                    assert_eq!(*cue_list.cues.get(&cue_id).unwrap(), estimated_new_cue);
                    break;
                }
                Ok(_) => continue,
                Err(e) => panic!("event channel closed before CueListUpdated: {e}"),
            }
        }
    })
    .await
    .expect("timed out waiting for CueListUpdated");

    let model = model_handle.read().await;
    assert_eq!(
        *model.cue_list.cues.get(&cue_id).unwrap(),
        estimated_new_cue
    );
    assert!(temp_dir.path().join(estimated_audio_target).exists());
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
            path: temp_dir
                .path()
                .to_path_buf()
                .join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME),
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

    let estimated_audio_target = PathBuf::from(temp_target.path().file_name().unwrap());
    let mut estimated_new_cue = new_cue.clone();
    if let CueParam::Audio(audio_param) = &mut estimated_new_cue.params {
        audio_param.target = estimated_audio_target.clone();
    }

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match event_rx.recv().await {
                Ok(BackendEvent::CueListUpdated { cue_list }) => {
                    assert_eq!(*cue_list.cues.get(&cue_id).unwrap(), estimated_new_cue);
                    break;
                }
                Ok(_) => continue,
                Err(e) => panic!("event channel closed before CueListUpdated: {e}"),
            }
        }
    })
    .await
    .expect("timed out waiting for CueListUpdated");

    let model = model_handle.read().await;
    assert_eq!(
        *model.cue_list.cues.get(&cue_id).unwrap(),
        estimated_new_cue
    );
    assert!(temp_dir.path().join(estimated_audio_target).exists());
    drop(temp_target);
    drop(temp_dir);
}

#[tokio::test]
async fn remove_cue_removes_single_root_cue() {
    let cue_id = Uuid::new_v4();
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(
            vec![make_audio_cue(cue_id, None, "1", PathBuf::from("a.mp3"))],
            vec![cue_id],
        )),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle.remove_cue(cue_id).await.unwrap();

    let removed_ids: HashSet<Uuid> = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::CueRemoved { cue_ids } => Some(cue_ids.clone()),
        _ => None,
    })
    .await;
    assert_eq!(removed_ids, HashSet::from([cue_id]));

    recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::CueListUpdated { cue_list } if !cue_list.cues.contains_key(&cue_id) => {
            Some(())
        }
        _ => None,
    })
    .await;

    let model = model_handle.read().await;
    assert!(!model.cue_list.cues.contains_key(&cue_id));
    assert!(!model.cue_list.root_ids.contains(&cue_id));
}

#[tokio::test]
async fn remove_cue_cascades_to_all_descendants() {
    let group_id = Uuid::new_v4();
    let child_a = Uuid::new_v4();
    let inner_group = Uuid::new_v4();
    let grandchild = Uuid::new_v4();

    let cues = vec![
        make_group_cue(group_id, None, "1", vec![child_a, inner_group]),
        make_audio_cue(child_a, Some(group_id), "1.1", PathBuf::from("a.mp3")),
        make_group_cue(inner_group, Some(group_id), "1.2", vec![grandchild]),
        make_audio_cue(
            grandchild,
            Some(inner_group),
            "1.2.1",
            PathBuf::from("b.mp3"),
        ),
    ];

    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(cues, vec![group_id])),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle.remove_cue(group_id).await.unwrap();

    let removed_ids: HashSet<Uuid> = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::CueRemoved { cue_ids } => Some(cue_ids.clone()),
        _ => None,
    })
    .await;
    assert_eq!(
        removed_ids,
        HashSet::from([group_id, child_a, inner_group, grandchild])
    );

    let model = model_handle.read().await;
    assert!(model.cue_list.cues.is_empty());
    assert!(model.cue_list.root_ids.is_empty());
}

#[tokio::test]
async fn remove_cue_not_found_sends_operation_failed_only() {
    let existing_id = Uuid::new_v4();
    let unknown_id = Uuid::new_v4();
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(
            vec![make_audio_cue(
                existing_id,
                None,
                "1",
                PathBuf::from("a.mp3"),
            )],
            vec![existing_id],
        )),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle.remove_cue(unknown_id).await.unwrap();

    let message = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::OperationFailed {
            error: BackendError::CueEdit { message },
        } => Some(message.clone()),
        _ => None,
    })
    .await;
    assert!(message.contains("not found"));

    assert!(
        tokio::time::timeout(Duration::from_millis(200), event_rx.recv())
            .await
            .is_err()
    );

    let model = model_handle.read().await;
    assert!(model.cue_list.cues.contains_key(&existing_id));
}

#[tokio::test]
async fn remove_cues_skips_unknown_ids_without_failing_whole_operation() {
    let known_id = Uuid::new_v4();
    let unknown_id = Uuid::new_v4();
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(
            vec![make_audio_cue(known_id, None, "1", PathBuf::from("a.mp3"))],
            vec![known_id],
        )),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .remove_cues(HashSet::from([known_id, unknown_id]))
        .await
        .unwrap();

    let removed_ids: HashSet<Uuid> = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::CueRemoved { cue_ids } => Some(cue_ids.clone()),
        _ => None,
    })
    .await;
    assert_eq!(removed_ids, HashSet::from([known_id]));
}

#[tokio::test]
async fn move_cue_reparents_between_groups() {
    let group_a = Uuid::new_v4();
    let group_b = Uuid::new_v4();
    let child = Uuid::new_v4();

    let cues = vec![
        make_group_cue(group_a, None, "1", vec![child]),
        make_audio_cue(child, Some(group_a), "1.1", PathBuf::from("a.mp3")),
        make_group_cue(group_b, None, "2", vec![]),
    ];
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(cues, vec![group_a, group_b])),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .move_cue(
            child,
            InsertPosition::Inside {
                target: Some(group_b),
                index: None,
            },
        )
        .await
        .unwrap();

    recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::CueListUpdated { .. } => Some(()),
        _ => None,
    })
    .await;

    let model = model_handle.read().await;
    let CueParam::Group {
        children: a_children,
        ..
    } = &model.cue_list.cues[&group_a].params
    else {
        panic!();
    };
    assert!(a_children.is_empty());
    let CueParam::Group {
        children: b_children,
        ..
    } = &model.cue_list.cues[&group_b].params
    else {
        panic!();
    };
    assert_eq!(b_children, &vec![child]);
    assert_eq!(model.cue_list.cues[&child].parent_id, Some(group_b));
}

#[tokio::test]
async fn move_cue_rolls_back_when_target_not_found() {
    let group_a = Uuid::new_v4();
    let child1 = Uuid::new_v4();
    let child2 = Uuid::new_v4();
    let nonexistent_target = Uuid::new_v4();

    let cues = vec![
        make_group_cue(group_a, None, "1", vec![child1, child2]),
        make_audio_cue(child1, Some(group_a), "1.1", PathBuf::from("a.mp3")),
        make_audio_cue(child2, Some(group_a), "1.2", PathBuf::from("b.mp3")),
    ];
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(cues, vec![group_a])),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .move_cue(
            child1,
            InsertPosition::Before {
                target: nonexistent_target,
            },
        )
        .await
        .unwrap();

    let message = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::OperationFailed {
            error: BackendError::CueEdit { message },
        } => Some(message.clone()),
        _ => None,
    })
    .await;
    assert!(message.contains("Invalid tree structure"));

    let model = model_handle.read().await;
    let CueParam::Group { children, .. } = &model.cue_list.cues[&group_a].params else {
        panic!();
    };
    assert_eq!(
        children,
        &vec![child1, child2],
        "move should be fully rolled back on failure"
    );
    assert_eq!(model.cue_list.cues[&child1].parent_id, Some(group_a));
}

#[tokio::test]
async fn move_cue_rolls_back_when_moving_into_own_descendant() {
    let group_a = Uuid::new_v4();
    let group_b = Uuid::new_v4();

    let cues = vec![
        make_group_cue(group_a, None, "1", vec![group_b]),
        make_group_cue(group_b, Some(group_a), "1.1", vec![]),
    ];
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(cues, vec![group_a])),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .move_cue(
            group_a,
            InsertPosition::Inside {
                target: Some(group_b),
                index: None,
            },
        )
        .await
        .unwrap();

    let message = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::OperationFailed {
            error: BackendError::CueEdit { message },
        } => Some(message.clone()),
        _ => None,
    })
    .await;
    assert!(message.contains("own descendant"));

    let model = model_handle.read().await;
    assert_eq!(
        model.cue_list.root_ids,
        vec![group_a],
        "group_a should remain at root"
    );
    let CueParam::Group { children, .. } = &model.cue_list.cues[&group_a].params else {
        panic!();
    };
    assert_eq!(
        children,
        &vec![group_b],
        "group_b should remain group_a's child"
    );
}

#[tokio::test]
async fn move_cue_fails_when_target_id_does_not_exist_in_model() {
    let existing_id = Uuid::new_v4();
    let unknown_id = Uuid::new_v4();
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(
            vec![make_audio_cue(
                existing_id,
                None,
                "1",
                PathBuf::from("a.mp3"),
            )],
            vec![existing_id],
        )),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .move_cue(
            unknown_id,
            InsertPosition::Inside {
                target: None,
                index: None,
            },
        )
        .await
        .unwrap();

    let message = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::OperationFailed {
            error: BackendError::CueEdit { message },
        } => Some(message.clone()),
        _ => None,
    })
    .await;
    assert!(message.contains("No valid cues found to move"));

    let model = model_handle.read().await;
    assert_eq!(model.cue_list.root_ids, vec![existing_id]);
}

#[tokio::test]
async fn move_cues_batch_moves_preserving_relative_order() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(a, None, "1", PathBuf::from("a.mp3")),
        make_audio_cue(b, None, "2", PathBuf::from("b.mp3")),
        make_audio_cue(c, None, "3", PathBuf::from("c.mp3")),
    ];
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(cues, vec![a, b, c])),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .move_cues(HashSet::from([a, c]), InsertPosition::After { target: b })
        .await
        .unwrap();

    recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::CueListUpdated { .. } => Some(()),
        _ => None,
    })
    .await;

    let model = model_handle.read().await;
    assert_eq!(model.cue_list.root_ids, vec![b, a, c]);
}

#[tokio::test]
async fn renumber_cues_applies_sequential_numbers_with_prefix_and_suffix() {
    let c1 = Uuid::new_v4();
    let c2 = Uuid::new_v4();
    let c3 = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(c1, None, "x", PathBuf::from("a.mp3")),
        make_audio_cue(c2, None, "y", PathBuf::from("b.mp3")),
        make_audio_cue(c3, None, "z", PathBuf::from("c.mp3")),
    ];
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(cues, vec![c1, c2, c3])),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .renumber_cues(
            vec![c1, c2, c3],
            10,
            5,
            Some("Q".to_string()),
            Some("-A".to_string()),
        )
        .await
        .unwrap();

    recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::CueListUpdated { .. } => Some(()),
        _ => None,
    })
    .await;

    let model = model_handle.read().await;
    assert_eq!(model.cue_list.cues[&c1].number, "Q10-A");
    assert_eq!(model.cue_list.cues[&c2].number, "Q15-A");
    assert_eq!(model.cue_list.cues[&c3].number, "Q20-A");
}

#[tokio::test]
async fn renumber_cues_sends_no_event_when_numbers_are_unchanged() {
    let c1 = Uuid::new_v4();
    let c2 = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(c1, None, "1", PathBuf::from("a.mp3")),
        make_audio_cue(c2, None, "2", PathBuf::from("b.mp3")),
    ];
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(cues, vec![c1, c2])),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .renumber_cues(vec![c1, c2], 1, 1, None, None)
        .await
        .unwrap();

    assert!(
        tokio::time::timeout(Duration::from_millis(200), event_rx.recv())
            .await
            .is_err(),
        "no CueListUpdated should be sent when nothing actually changed"
    );
}

#[tokio::test]
async fn renumber_cues_only_affects_targets_sharing_the_first_matched_parent() {
    let root_cue = Uuid::new_v4();
    let group = Uuid::new_v4();
    let nested_child = Uuid::new_v4();

    let cues = vec![
        make_audio_cue(root_cue, None, "orig-root", PathBuf::from("a.mp3")),
        make_group_cue(group, None, "orig-group", vec![nested_child]),
        make_audio_cue(
            nested_child,
            Some(group),
            "orig-nested",
            PathBuf::from("b.mp3"),
        ),
    ];
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(cues, vec![root_cue, group])),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .renumber_cues(vec![root_cue, nested_child], 1, 1, None, None)
        .await
        .unwrap();

    recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::CueListUpdated { .. } => Some(()),
        _ => None,
    })
    .await;

    let model = model_handle.read().await;
    assert_eq!(model.cue_list.cues[&root_cue].number, "1");
    assert_eq!(model.cue_list.cues[&nested_child].number, "orig-nested");
}

#[tokio::test]
async fn save_to_file_writes_single_file_and_emits_saved_event() {
    let temp_dir = tempdir().unwrap();
    let dest_path = temp_dir.path().join("myshow.sbsp");
    let cue_id = Uuid::new_v4();

    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(
            vec![make_audio_cue(cue_id, None, "1", PathBuf::from("a.mp3"))],
            vec![cue_id],
        )),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle.save_as(dest_path.clone()).await.unwrap();

    let (project_type, path) = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::ShowModelSaved { project_type, path } => Some((*project_type, path.clone())),
        _ => None,
    })
    .await;
    assert_eq!(project_type, ProjectType::SingleFile);
    assert_eq!(path, dest_path);

    assert!(dest_path.exists());
    let content = tokio::fs::read_to_string(&dest_path).await.unwrap();
    assert!(content.contains(&cue_id.to_string()));

    assert!(
        tokio::time::timeout(Duration::from_millis(200), event_rx.recv())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn save_command_fails_when_project_status_is_unsaved() {
    let (model_handle, mut event_rx) = setup_manager(None, ProjectStatus::Unsaved).await;

    model_handle.save().await.unwrap();

    let (path, message) = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::OperationFailed {
            error: BackendError::SaveToFile { path, message },
        } => Some((path.clone(), message.clone())),
        _ => None,
    })
    .await;
    assert_eq!(path, PathBuf::new());
    assert!(message.contains("no file path is set"));
}

#[tokio::test]
async fn save_command_persists_to_existing_saved_path_without_cuelistupdated() {
    let temp_dir = tempdir().unwrap();
    let model_path = temp_dir.path().join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME);
    let cue_id = Uuid::new_v4();

    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(
            vec![make_audio_cue(
                cue_id,
                None,
                "1",
                PathBuf::from("audio/a.mp3"),
            )],
            vec![cue_id],
        )),
        ProjectStatus::Saved {
            project_type: ProjectType::ProjectFolder,
            path: model_path.clone(),
        },
    )
    .await;

    model_handle.save().await.unwrap();

    let (project_type, path) = recv_event_matching_result(&mut event_rx, |e| match e {
        BackendEvent::ShowModelSaved { project_type, path } => {
            Some(Ok((*project_type, path.clone())))
        }
        BackendEvent::CueListUpdated { .. } => Some(Err(anyhow::anyhow!(
            "CueListUpdated event is wrongly emitted."
        ))),
        _ => None,
    })
    .await
    .unwrap();
    assert_eq!(project_type, ProjectType::ProjectFolder);
    assert_eq!(path, model_path);
    assert!(model_path.exists());

    assert!(
        tokio::time::timeout(Duration::from_millis(200), event_rx.recv())
            .await
            .is_err(),
        "Unexpected event received."
    );
}

#[tokio::test]
async fn export_to_folder_fails_when_path_is_not_a_directory() {
    let not_a_dir = NamedTempFile::new().unwrap();
    let (model_handle, mut event_rx) = setup_manager(None, ProjectStatus::Unsaved).await;

    model_handle
        .export_to_folder(not_a_dir.path().to_path_buf())
        .await
        .unwrap();

    let message = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::OperationFailed {
            error: BackendError::ExportToFolder { message, .. },
        } => Some(message.clone()),
        _ => None,
    })
    .await;
    assert!(message.contains("not directory"));
}

#[tokio::test]
async fn export_to_folder_writes_model_file_into_directory() {
    let temp_dir = tempdir().unwrap();
    let cue_id = Uuid::new_v4();
    let (model_handle, mut event_rx) = setup_manager(
        Some(build_model(
            vec![make_audio_cue(
                cue_id,
                None,
                "1",
                PathBuf::from("audio/a.mp3"),
            )],
            vec![cue_id],
        )),
        ProjectStatus::Unsaved,
    )
    .await;

    model_handle
        .export_to_folder(temp_dir.path().to_path_buf())
        .await
        .unwrap();

    let expected_path = temp_dir.path().join(DEFAULT_PROJECT_FOLDER_MODEL_FILENAME);
    let (project_type, path) = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::ShowModelSaved { project_type, path } => Some((*project_type, path.clone())),
        _ => None,
    })
    .await;
    assert_eq!(project_type, ProjectType::ProjectFolder);
    assert_eq!(path, expected_path);
    assert!(expected_path.exists());
}

#[tokio::test]
async fn load_from_file_round_trips_a_previously_saved_model() {
    let temp_dir = tempdir().unwrap();
    let dest_path = temp_dir.path().join("myshow.sbsp");
    let cue_id = Uuid::new_v4();

    let (writer_handle, mut writer_event_rx) = setup_manager(
        Some(build_model(
            vec![make_audio_cue(cue_id, None, "1", PathBuf::from("a.mp3"))],
            vec![cue_id],
        )),
        ProjectStatus::Unsaved,
    )
    .await;
    writer_handle.save_as(dest_path.clone()).await.unwrap();
    recv_event_matching(&mut writer_event_rx, |e| match e {
        BackendEvent::ShowModelSaved { .. } => Some(()),
        _ => None,
    })
    .await;

    let (reader_handle, mut reader_event_rx) = setup_manager(None, ProjectStatus::Unsaved).await;
    reader_handle
        .load_from_file(dest_path.clone())
        .await
        .unwrap();

    let (project_type, path) = recv_event_matching(&mut reader_event_rx, |e| match e {
        BackendEvent::ShowModelLoaded {
            project_type, path, ..
        } => Some((*project_type, path.clone())),
        _ => None,
    })
    .await;
    assert_eq!(project_type, ProjectType::SingleFile);
    assert_eq!(path, dest_path);

    let model = reader_handle.read().await;
    assert_eq!(model.cue_list.cues[&cue_id].number, "1");
}

#[tokio::test]
async fn load_from_file_fails_when_file_does_not_exist() {
    let temp_dir = tempdir().unwrap();
    let missing_path = temp_dir.path().join("does_not_exist.sbsp");
    let (model_handle, mut event_rx) = setup_manager(None, ProjectStatus::Unsaved).await;

    model_handle
        .load_from_file(missing_path.clone())
        .await
        .unwrap();

    let (path, _message) = recv_event_matching(&mut event_rx, |e| match e {
        BackendEvent::OperationFailed {
            error: BackendError::LoadFromFile { path, message },
        } => Some((path.clone(), message.clone())),
        _ => None,
    })
    .await;
    assert_eq!(path, missing_path);
}
