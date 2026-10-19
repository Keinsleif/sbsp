// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Making a new cue from the settings' templates and deciding where it
//! goes. Ported from the `addEmpty*Cue` actions of `stores/showModel.ts`.
//!
//! The rules, as the original has them:
//! - a new cue goes *after* the selected cue, or at the end of the list
//!   when nothing is selected;
//! - a fade cue targets the selected cue, which must be an audio or group
//!   cue (otherwise nothing is added);
//! - start / stop / pause / load cues target the selected cue (nothing is
//!   added without one); start and load go *before* it (they should fire
//!   first), stop and pause *after* it;
//! - a group cue takes the selected rows into itself afterwards.

use std::collections::HashMap;
use std::path::PathBuf;

use sbsp_backend::manager::InsertPosition;
use sbsp_backend::model::cue::{Cue, CueParam, Uuid};
use sbsp_frontend_settings::TemplateSettings;

/// File extensions the audio file picker offers (the original's list).
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "aiff", "aif", "caf", "mp4", "m4a", "mkv", "mka", "webm", "ogg", "oga", "wav", "aac", "alac",
    "flac", "mp3",
];

/// Whether `path` names a file of a kind an audio cue can play, going by
/// its extension (any letter case).
pub fn is_audio_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            AUDIO_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

/// The kinds of cue the add buttons make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CueKind {
    Audio,
    Wait,
    Fade,
    Group,
    Start,
    Stop,
    Pause,
    Load,
}

/// A cue ready to be added, and where.
#[derive(Debug, Clone)]
pub struct NewCue {
    pub cue: Cue,
    pub position: InsertPosition,
}

/// After `selected`, or at the end of the root list.
pub fn position_after(selected: Option<Uuid>) -> InsertPosition {
    match selected {
        Some(target) => InsertPosition::After { target },
        None => InsertPosition::LAST,
    }
}

/// A new non-audio cue (audio needs files first, see [`audio_cues`]).
/// `None` when the selection does not allow it. `id` becomes the cue's id.
pub fn build(
    kind: CueKind,
    id: Uuid,
    templates: &TemplateSettings,
    selected: Option<Uuid>,
    cues: &HashMap<Uuid, Cue>,
) -> Option<NewCue> {
    let selected_cue = selected.and_then(|id| cues.get(&id));

    let (mut cue, position) = match kind {
        CueKind::Audio => return None,
        CueKind::Wait => (templates.wait.clone(), position_after(selected)),
        CueKind::Group => (templates.group.clone(), position_after(selected)),
        CueKind::Fade => {
            let target = selected_cue.filter(|cue| {
                matches!(cue.params, CueParam::Audio(_) | CueParam::Group { .. })
            })?;
            let mut cue = templates.fade.clone();
            if let CueParam::Fade(fade) = &mut cue.params {
                fade.target = target.id;
            }
            (cue, InsertPosition::After { target: target.id })
        }
        CueKind::Start | CueKind::Stop | CueKind::Pause | CueKind::Load => {
            let target = selected_cue?.id;
            let (mut cue, position) = match kind {
                CueKind::Start => (templates.start.clone(), InsertPosition::Before { target }),
                CueKind::Load => (templates.load.clone(), InsertPosition::Before { target }),
                CueKind::Stop => (templates.stop.clone(), InsertPosition::After { target }),
                _ => (templates.pause.clone(), InsertPosition::After { target }),
            };
            match &mut cue.params {
                CueParam::Start(param) => param.target = target,
                CueParam::Stop(param) => param.target = target,
                CueParam::Pause(param) => param.target = target,
                CueParam::Load(param) => param.target = target,
                _ => {}
            }
            (cue, position)
        }
    };

    cue.id = id;
    Some(NewCue { cue, position })
}

/// One audio cue per file, in the order given, after the selection.
/// `ids` supplies the new cues' ids (one per file; extra files without an
/// id are dropped).
pub fn audio_cues(
    files: Vec<PathBuf>,
    ids: impl IntoIterator<Item = Uuid>,
    templates: &TemplateSettings,
    selected: Option<Uuid>,
) -> Option<(Vec<Cue>, InsertPosition)> {
    let cues: Vec<Cue> = files
        .into_iter()
        .zip(ids)
        .map(|(file, id)| {
            let mut cue = templates.audio.clone();
            cue.id = id;
            if let CueParam::Audio(audio) = &mut cue.params {
                audio.target = file;
            }
            cue
        })
        .collect();

    (!cues.is_empty()).then(|| (cues, position_after(selected)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn audio_files_are_told_by_extension() {
        use super::is_audio_file;
        use std::path::Path;
        assert!(is_audio_file(Path::new("/a/b/song.wav")));
        assert!(is_audio_file(Path::new("SONG.MP3")));
        assert!(!is_audio_file(Path::new("notes.txt")));
        assert!(!is_audio_file(Path::new("wav")));
        assert!(!is_audio_file(Path::new("/a/folder")));
    }

    use super::*;
    use sbsp_backend::model::cue::WaitCueParam;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn cue_with(n: u128, params: CueParam) -> Cue {
        let mut cue = TemplateSettings::default().wait;
        cue.id = id(n);
        cue.params = params;
        cue
    }

    fn model(cues: &[Cue]) -> HashMap<Uuid, Cue> {
        cues.iter().map(|cue| (cue.id, cue.clone())).collect()
    }

    fn audio(n: u128) -> Cue {
        let mut template = TemplateSettings::default().audio;
        template.id = id(n);
        template
    }

    fn wait(n: u128) -> Cue {
        cue_with(n, CueParam::Wait(WaitCueParam { duration: 1.0 }))
    }

    fn is_after(position: &InsertPosition, expected: Uuid) -> bool {
        matches!(position, InsertPosition::After { target } if *target == expected)
    }

    fn is_before(position: &InsertPosition, expected: Uuid) -> bool {
        matches!(position, InsertPosition::Before { target } if *target == expected)
    }

    #[test]
    fn a_wait_cue_goes_after_the_selection_or_at_the_end() {
        let templates = TemplateSettings::default();
        let cues = model(&[wait(1)]);

        let after = build(CueKind::Wait, id(9), &templates, Some(id(1)), &cues).unwrap();
        assert!(is_after(&after.position, id(1)));
        assert_eq!(after.cue.id, id(9));
        assert!(matches!(after.cue.params, CueParam::Wait(_)));

        let last = build(CueKind::Wait, id(9), &templates, None, &cues).unwrap();
        assert!(matches!(
            last.position,
            InsertPosition::Inside { target: None, index: None }
        ));
    }

    #[test]
    fn a_fade_cue_needs_a_selected_audio_or_group_cue() {
        let templates = TemplateSettings::default();
        let cues = model(&[audio(1), wait(2)]);

        let fade = build(CueKind::Fade, id(9), &templates, Some(id(1)), &cues).unwrap();
        assert!(is_after(&fade.position, id(1)));
        match fade.cue.params {
            CueParam::Fade(param) => assert_eq!(param.target, id(1)),
            other => panic!("not a fade cue: {other:?}"),
        }

        assert!(build(CueKind::Fade, id(9), &templates, Some(id(2)), &cues).is_none());
        assert!(build(CueKind::Fade, id(9), &templates, None, &cues).is_none());
    }

    #[test]
    fn start_and_load_go_before_the_target_stop_and_pause_after() {
        let templates = TemplateSettings::default();
        let cues = model(&[audio(1)]);

        let start = build(CueKind::Start, id(9), &templates, Some(id(1)), &cues).unwrap();
        assert!(is_before(&start.position, id(1)));
        let load = build(CueKind::Load, id(9), &templates, Some(id(1)), &cues).unwrap();
        assert!(is_before(&load.position, id(1)));
        let stop = build(CueKind::Stop, id(9), &templates, Some(id(1)), &cues).unwrap();
        assert!(is_after(&stop.position, id(1)));
        let pause = build(CueKind::Pause, id(9), &templates, Some(id(1)), &cues).unwrap();
        assert!(is_after(&pause.position, id(1)));

        match start.cue.params {
            CueParam::Start(param) => assert_eq!(param.target, id(1)),
            other => panic!("not a start cue: {other:?}"),
        }
        match stop.cue.params {
            CueParam::Stop(param) => assert_eq!(param.target, id(1)),
            other => panic!("not a stop cue: {other:?}"),
        }
    }

    #[test]
    fn playback_cues_need_a_selection_that_exists() {
        let templates = TemplateSettings::default();
        let cues = model(&[audio(1)]);

        assert!(build(CueKind::Start, id(9), &templates, None, &cues).is_none());
        assert!(build(CueKind::Stop, id(9), &templates, Some(id(404)), &cues).is_none());
    }

    #[test]
    fn a_group_cue_is_added_like_any_other() {
        let templates = TemplateSettings::default();
        let cues = model(&[audio(1)]);

        let group = build(CueKind::Group, id(9), &templates, Some(id(1)), &cues).unwrap();
        assert!(is_after(&group.position, id(1)));
        assert!(matches!(group.cue.params, CueParam::Group { .. }));
    }

    #[test]
    fn audio_is_not_built_without_files() {
        let templates = TemplateSettings::default();
        assert!(build(CueKind::Audio, id(9), &templates, None, &HashMap::new()).is_none());
        assert!(audio_cues(Vec::new(), [id(1)], &templates, None).is_none());
    }

    #[test]
    fn each_file_becomes_an_audio_cue_with_its_own_id() {
        let templates = TemplateSettings::default();
        let files = vec![PathBuf::from("/a.wav"), PathBuf::from("/b.wav")];

        let (cues, position) =
            audio_cues(files, [id(1), id(2)], &templates, Some(id(7))).unwrap();

        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].id, id(1));
        assert_eq!(cues[1].id, id(2));
        assert!(is_after(&position, id(7)));
        match &cues[1].params {
            CueParam::Audio(audio) => assert_eq!(audio.target, PathBuf::from("/b.wav")),
            other => panic!("not an audio cue: {other:?}"),
        }
    }
}
