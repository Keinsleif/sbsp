// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `utils.ts`: `buildCueName`, `calculateDuration`,
//! `getDuration`.
//!
//! Unlike the rest of `domain/`, these take the show model (or at least a
//! `&HashMap<Uuid, Cue>`) as an explicit parameter rather than being
//! context-free: `buildCueName` looks up a target cue by id for
//! fade/start/stop/pause/load cues, and may recurse into that cue's own
//! name. This is still pure, UI-independent logic (no mutable state, no
//! iced types), so it lives here per the migration plan's `domain/cue.rs`,
//! not under `state/`.

use std::collections::HashMap;

use sbsp_backend::model::cue::{Cue, CueParam, Uuid, group::GroupMode};
use sbsp_frontend_settings::NameFormatSettings;

use super::text::{camel_to_title_case, format_template};
use super::time::format_human;

/// Builds a cue's display name from the name-format settings. `None`
/// (a missing target, matching the original's `cue ?? null`) renders as an
/// empty string.
pub fn build_cue_name(
    cue: Option<&Cue>,
    cues: &HashMap<Uuid, Cue>,
    name_format: &NameFormatSettings,
) -> String {
    let Some(cue) = cue else {
        return String::new();
    };

    match &cue.params {
        CueParam::Audio(param) => {
            let filename = param
                .target
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default();
            format_template(&name_format.audio, &values([("filename", filename)]))
        }
        CueParam::Wait(param) => format_template(
            &name_format.wait,
            &values([("duration", format_human(param.duration))]),
        ),
        CueParam::Fade(param) => {
            target_named(cues, param.target, &name_format.fade, name_format)
        }
        CueParam::Start(param) => {
            target_named(cues, param.target, &name_format.start, name_format)
        }
        CueParam::Stop(param) => {
            target_named(cues, param.target, &name_format.stop, name_format)
        }
        CueParam::Pause(param) => {
            target_named(cues, param.target, &name_format.pause, name_format)
        }
        CueParam::Load(param) => {
            target_named(cues, param.target, &name_format.load, name_format)
        }
        CueParam::Group { base, .. } => format_template(
            &name_format.group,
            &values([("mode", camel_to_title_case(group_mode_name(&base.mode)))]),
        ),
    }
}

fn target_named(
    cues: &HashMap<Uuid, Cue>,
    target: Uuid,
    template: &str,
    name_format: &NameFormatSettings,
) -> String {
    let target_name = build_cue_name(cues.get(&target), cues, name_format);
    format_template(template, &values([("targetName", target_name)]))
}

fn values<const N: usize>(pairs: [(&str, String); N]) -> HashMap<String, String> {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// The serde tag string for a [`GroupMode`] variant (`"playlist"`,
/// `"concurrency"`, `"startFirst"`), used as the `{mode}` template value.
fn group_mode_name(mode: &GroupMode) -> &'static str {
    match mode {
        GroupMode::Playlist { .. } => "playlist",
        GroupMode::Concurrency => "concurrency",
        GroupMode::StartFirst { .. } => "startFirst",
    }
}

/// An audio cue's effective playback duration given the full asset
/// duration (`None` if unknown, e.g. not analyzed yet): the asset duration
/// trimmed to `endTime` (if earlier) and offset by `startTime`. Non-audio
/// cues with an intrinsic duration (wait, fade) return it directly;
/// everything else (start/stop/pause/load/group) has no duration of its
/// own.
pub fn calculate_duration(param: &CueParam, total_duration: Option<f64>) -> Option<f64> {
    match param {
        CueParam::Audio(audio) => {
            let total = total_duration.filter(|d| !d.is_nan())?;
            let mut duration = total;
            if let Some(end_time) = audio.end_time {
                if end_time < total {
                    duration = end_time;
                }
            }
            if let Some(start_time) = audio.start_time {
                duration -= start_time;
            }
            Some(duration)
        }
        CueParam::Wait(wait) => Some(wait.duration),
        CueParam::Fade(fade) => Some(fade.fade_param.duration),
        CueParam::Start(_)
        | CueParam::Stop(_)
        | CueParam::Pause(_)
        | CueParam::Load(_)
        | CueParam::Group { .. } => None,
    }
}

/// [`calculate_duration`], plus the asset-duration lookup for audio cues.
/// `asset_duration` is a callback rather than a concrete asset store
/// (`state::assets::AssetResults` doesn't exist yet) so this stays testable
/// without one; it is called with the *cue's* id, matching the original
/// (`assetResult.getMetadata(cue.id)`, keyed by cue id, not by asset path).
pub fn get_duration(cue: Option<&Cue>, asset_duration: impl Fn(Uuid) -> Option<f64>) -> Option<f64> {
    let cue = cue?;
    match &cue.params {
        CueParam::Wait(param) => Some(param.duration),
        CueParam::Audio(_) => calculate_duration(&cue.params, asset_duration(cue.id)),
        CueParam::Fade(param) => Some(param.fade_param.duration),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::{
        CueChain, CueColor, CueCursorAdvanceTriggerOverride, StartCueParam, WaitCueParam,
        audio::AudioCueParam,
        group::GroupCueParamBase,
    };
    use std::path::PathBuf;

    fn bare_cue(id: Uuid, parent_id: Option<Uuid>, params: CueParam) -> Cue {
        Cue {
            id,
            number: String::new(),
            name: None,
            notes: String::new(),
            color: CueColor::default(),
            pre_wait: 0.0,
            chain: CueChain::default(),
            treat_stop_as_completed: false,
            cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::default(),
            parent_id,
            params,
        }
    }

    fn name_format() -> NameFormatSettings {
        NameFormatSettings::default()
    }

    #[test]
    fn none_cue_is_empty_name() {
        assert_eq!(build_cue_name(None, &HashMap::new(), &name_format()), "");
    }

    #[test]
    fn audio_cue_uses_filename() {
        let id = Uuid::new_v4();
        let cue = bare_cue(
            id,
            None,
            CueParam::Audio(AudioCueParam {
                target: PathBuf::from("/music/show/kick.wav"),
                start_time: None,
                fade_in_param: None,
                end_time: None,
                fade_out_param: None,
                volume: Default::default(),
                pan: 0.0,
                repeat: false,
                sound_type: Default::default(),
                envelope: Vec::new(),
            }),
        );
        assert_eq!(
            build_cue_name(Some(&cue), &HashMap::new(), &name_format()),
            "kick.wav"
        );
    }

    #[test]
    fn wait_cue_uses_human_duration() {
        let cue = bare_cue(Uuid::new_v4(), None, CueParam::Wait(WaitCueParam { duration: 90.0 }));
        // format_human(90.0) == "01m 30.0s": the minute-branch always
        // appends ".{ms}", even when ms trims down to a single "0" (see
        // domain::time's doc comment / tests) -- this is not "01m 30s".
        assert_eq!(
            build_cue_name(Some(&cue), &HashMap::new(), &name_format()),
            "Wait 01m 30.0s"
        );
    }

    #[test]
    fn start_cue_recurses_into_target_name() {
        let target_id = Uuid::new_v4();
        let target = bare_cue(
            target_id,
            None,
            CueParam::Wait(WaitCueParam { duration: 1.0 }),
        );
        let start_id = Uuid::new_v4();
        let start = bare_cue(start_id, None, CueParam::Start(StartCueParam { target: target_id }));

        let mut cues = HashMap::new();
        cues.insert(target_id, target);
        cues.insert(start_id, start.clone());

        assert_eq!(
            build_cue_name(Some(&start), &cues, &name_format()),
            "Start Wait 1s"
        );
    }

    #[test]
    fn missing_target_recurses_into_empty_name() {
        let start = bare_cue(
            Uuid::new_v4(),
            None,
            CueParam::Start(StartCueParam { target: Uuid::new_v4() }),
        );
        assert_eq!(
            build_cue_name(Some(&start), &HashMap::new(), &name_format()),
            "Start "
        );
    }

    #[test]
    fn group_cue_names_by_mode() {
        let group = bare_cue(
            Uuid::new_v4(),
            None,
            CueParam::Group {
                base: GroupCueParamBase { mode: GroupMode::Concurrency },
                children: Vec::new(),
            },
        );
        assert_eq!(
            build_cue_name(Some(&group), &HashMap::new(), &name_format()),
            "Group"
        );
    }

    #[test]
    fn calculate_duration_audio_trims_to_end_and_start() {
        let param = CueParam::Audio(AudioCueParam {
            target: PathBuf::new(),
            start_time: Some(2.0),
            fade_in_param: None,
            end_time: Some(8.0),
            fade_out_param: None,
            volume: Default::default(),
            pan: 0.0,
            repeat: false,
            sound_type: Default::default(),
            envelope: Vec::new(),
        });
        // total 10s, trimmed to end_time 8s, then offset by start_time 2s.
        assert_eq!(calculate_duration(&param, Some(10.0)), Some(6.0));
    }

    #[test]
    fn calculate_duration_audio_unknown_total_is_none() {
        let param = CueParam::Audio(AudioCueParam {
            target: PathBuf::new(),
            start_time: None,
            fade_in_param: None,
            end_time: None,
            fade_out_param: None,
            volume: Default::default(),
            pan: 0.0,
            repeat: false,
            sound_type: Default::default(),
            envelope: Vec::new(),
        });
        assert_eq!(calculate_duration(&param, None), None);
    }

    #[test]
    fn calculate_duration_non_timed_is_none() {
        assert_eq!(
            calculate_duration(&CueParam::Start(StartCueParam { target: Uuid::new_v4() }), None),
            None
        );
    }
}
