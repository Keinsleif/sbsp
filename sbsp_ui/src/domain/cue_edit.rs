// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Changing one property of a cue, as the bottom editor's fields do.
//!
//! Every function takes the cue as it is and returns the edited copy, or
//! `None` when the edit would change nothing (so an unchanged field does
//! not send an update to the backend). The backend replaces a cue as a
//! whole (`update_cue`), as the original editor does.

use std::path::PathBuf;

use sbsp_backend::model::cue::{
    Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid,
    audio::{AudioCueParam, Decibels, Easing, FadeParam, SoundType},
    group::GroupMode,
};

use crate::domain::easing::{Curve, CurveKind};
use crate::domain::envelope::{self, Segment};
use crate::domain::field::{
    Commit, NumberSpec, TimeSpec, commit_number, commit_time, step_number,
};
use crate::domain::time::format_seconds;

fn edited(cue: &Cue, change: impl FnOnce(&mut Cue)) -> Option<Cue> {
    let mut new = cue.clone();
    change(&mut new);
    (new != *cue).then_some(new)
}

pub fn set_number(cue: &Cue, text: &str) -> Option<Cue> {
    edited(cue, |cue| cue.number = text.to_string())
}

/// An empty name means "use the generated one".
pub fn set_name(cue: &Cue, text: &str) -> Option<Cue> {
    edited(cue, |cue| {
        cue.name = (!text.is_empty()).then(|| text.to_string());
    })
}

pub fn set_notes(cue: &Cue, text: &str) -> Option<Cue> {
    edited(cue, |cue| cue.notes = text.to_string())
}

pub fn set_pre_wait(cue: &Cue, text: &str) -> Option<Cue> {
    match commit_time(text, Some(cue.pre_wait), &TimeSpec::default()) {
        Commit::Value(value) => edited(cue, |cue| cue.pre_wait = value),
        Commit::Unchanged | Commit::Null => None,
    }
}

/// Whether the duration can be typed: only wait and fade cues own one
/// (an audio cue's follows from its file and start/end times).
pub fn duration_is_editable(cue: &Cue) -> bool {
    matches!(cue.params, CueParam::Wait(_) | CueParam::Fade(_))
}

pub fn set_duration(cue: &Cue, text: &str) -> Option<Cue> {
    let current = match &cue.params {
        CueParam::Wait(wait) => wait.duration,
        CueParam::Fade(fade) => fade.fade_param.duration,
        _ => return None,
    };
    let Commit::Value(value) = commit_time(text, Some(current), &TimeSpec::default()) else {
        return None;
    };
    edited(cue, |cue| match &mut cue.params {
        CueParam::Wait(wait) => wait.duration = value,
        CueParam::Fade(fade) => fade.fade_param.duration = value,
        _ => {}
    })
}

/// The chain mode without its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainKind {
    DoNotChain,
    AfterStart,
    AfterComplete,
}

pub fn chain_kind(chain: &CueChain) -> ChainKind {
    match chain {
        CueChain::DoNotChain => ChainKind::DoNotChain,
        CueChain::AfterStart { .. } => ChainKind::AfterStart,
        CueChain::AfterComplete { .. } => ChainKind::AfterComplete,
    }
}

/// The cue a chain continues with (`None`: the next one in the list).
pub fn chain_target(chain: &CueChain) -> Option<Uuid> {
    match chain {
        CueChain::DoNotChain => None,
        CueChain::AfterStart { target_id } | CueChain::AfterComplete { target_id } => *target_id,
    }
}

/// Switching between the two chaining modes keeps the target; "do not
/// chain" has none.
pub fn set_chain_kind(cue: &Cue, kind: ChainKind) -> Option<Cue> {
    let target_id = chain_target(&cue.chain);
    edited(cue, |cue| {
        cue.chain = match kind {
            ChainKind::DoNotChain => CueChain::DoNotChain,
            ChainKind::AfterStart => CueChain::AfterStart { target_id },
            ChainKind::AfterComplete => CueChain::AfterComplete { target_id },
        }
    })
}

/// Has no effect on a cue that does not chain.
pub fn set_chain_target(cue: &Cue, target: Option<Uuid>) -> Option<Cue> {
    edited(cue, |cue| match &mut cue.chain {
        CueChain::DoNotChain => {}
        CueChain::AfterStart { target_id } | CueChain::AfterComplete { target_id } => {
            *target_id = target;
        }
    })
}

pub fn set_color(cue: &Cue, color: CueColor) -> Option<Cue> {
    edited(cue, |cue| cue.color = color)
}

pub fn set_cursor_advance_override(
    cue: &Cue,
    value: CueCursorAdvanceTriggerOverride,
) -> Option<Cue> {
    edited(cue, |cue| cue.cursor_advance_trigger_override = value)
}

pub fn set_treat_stop_as_completed(cue: &Cue, value: bool) -> Option<Cue> {
    edited(cue, |cue| cue.treat_stop_as_completed = value)
}

/// Adds `[mm:ss.cc] ` on a line of its own at the end of the notes.
pub fn append_timestamp(cue: &Cue, position: f64) -> Option<Cue> {
    let stamp = format!("[{}] ", format_seconds(Some(position)));
    edited(cue, |cue| {
        if !cue.notes.is_empty() && !cue.notes.ends_with('\n') {
            cue.notes.push('\n');
        }
        cue.notes.push_str(&stamp);
    })
}

// ---------------------------------------------------------------------
// Audio

fn with_audio(cue: &Cue, change: impl FnOnce(&mut AudioCueParam)) -> Option<Cue> {
    if !matches!(cue.params, CueParam::Audio(_)) {
        return None;
    }
    edited(cue, |cue| {
        if let CueParam::Audio(audio) = &mut cue.params {
            change(audio);
        }
    })
}

pub fn set_audio_target(cue: &Cue, target: PathBuf) -> Option<Cue> {
    with_audio(cue, |audio| audio.target = target)
}

/// "Load the entire file on memory" (static) or stream it.
pub fn set_sound_static(cue: &Cue, is_static: bool) -> Option<Cue> {
    with_audio(cue, |audio| {
        audio.sound_type = if is_static {
            SoundType::Static
        } else {
            SoundType::Streaming
        };
    })
}

// ---------------------------------------------------------------------
// Time & levels

/// The start and end of the played part as the editor shows them: no
/// start is 0, no end is the file's duration (when known).
pub fn audio_start(cue: &Cue) -> Option<f64> {
    match &cue.params {
        CueParam::Audio(audio) => Some(audio.start_time.unwrap_or(0.0)),
        _ => None,
    }
}

pub fn audio_end(cue: &Cue, duration: Option<f64>) -> Option<f64> {
    match &cue.params {
        CueParam::Audio(audio) => audio.end_time.or(duration),
        _ => None,
    }
}

/// A start of 0 is stored as "no start", as the original does.
pub fn set_start_time(cue: &Cue, text: &str, duration: Option<f64>) -> Option<Cue> {
    let start = audio_start(cue)?;
    let spec = TimeSpec {
        min: Some(0.0),
        max: audio_end(cue, duration),
        ..TimeSpec::default()
    };
    match commit_time(text, Some(start), &spec) {
        Commit::Value(value) => with_audio(cue, |audio| {
            audio.start_time = (value > 0.0).then_some(value);
        }),
        Commit::Unchanged | Commit::Null => None,
    }
}

/// An end at the file's duration (or an emptied field) is stored as
/// "no end".
pub fn set_end_time(cue: &Cue, text: &str, duration: Option<f64>) -> Option<Cue> {
    let start = audio_start(cue)?;
    if text.trim().is_empty() {
        return with_audio(cue, |audio| audio.end_time = None);
    }
    let spec = TimeSpec {
        min: Some(start),
        max: duration,
        ..TimeSpec::default()
    };
    match commit_time(text, audio_end(cue, duration), &spec) {
        Commit::Value(value) => with_audio(cue, |audio| {
            audio.end_time = match duration {
                Some(duration) if value >= duration - 1e-9 => None,
                _ => Some(value),
            };
        }),
        Commit::Unchanged | Commit::Null => None,
    }
}

/// The volume (dB) of an audio or fade cue: what the volume fader shows.
pub fn level(cue: &Cue) -> Option<f64> {
    match &cue.params {
        CueParam::Audio(audio) => Some(f64::from(f32::from(audio.volume))),
        CueParam::Fade(_) => fade_volume(cue),
        _ => None,
    }
}

pub fn set_level(cue: &Cue, decibels: f64) -> Option<Cue> {
    match &cue.params {
        CueParam::Audio(_) => with_audio(cue, |audio| {
            audio.volume = Decibels::from(decibels as f32);
        }),
        CueParam::Fade(_) => set_fade_volume(cue, decibels),
        _ => None,
    }
}

/// The volume that brings a file of `integrated_lufs` to `target`.
pub fn volume_for_lufs(target: f64, integrated_lufs: f64) -> f64 {
    target - integrated_lufs
}

/// The volume that brings a file's `peak` (dBFS) to 0.
pub fn volume_for_peak(peak: f64) -> f64 {
    -peak
}

pub fn pan(cue: &Cue) -> Option<f64> {
    match &cue.params {
        CueParam::Audio(audio) => Some(f64::from(audio.pan)),
        _ => None,
    }
}

pub fn set_pan(cue: &Cue, pan: f64) -> Option<Cue> {
    with_audio(cue, |audio| audio.pan = pan.clamp(-1.0, 1.0) as f32)
}

/// The pan field: -1 (left) to 1 (right).
pub const PAN_SPEC: NumberSpec = NumberSpec {
    min: Some(-1.0),
    max: Some(1.0),
    step: 1.0 / 8.0,
    precision: 3,
    accept_null: false,
};

pub fn set_pan_text(cue: &Cue, text: &str) -> Option<Cue> {
    match commit_number(text, pan(cue), &PAN_SPEC) {
        Commit::Value(value) => set_pan(cue, value),
        Commit::Unchanged | Commit::Null => None,
    }
}

pub fn set_repeat(cue: &Cue, repeat: bool) -> Option<Cue> {
    with_audio(cue, |audio| audio.repeat = repeat)
}

/// The cue's envelope as the waveform editor shows it.
pub fn envelope(cue: &Cue) -> Vec<Segment> {
    match &cue.params {
        CueParam::Audio(audio) => envelope::normalize(&audio.envelope),
        _ => Vec::new(),
    }
}

pub fn set_envelope(cue: &Cue, segments: &[Segment]) -> Option<Cue> {
    with_audio(cue, |audio| {
        audio.envelope = segments.iter().copied().map(Into::into).collect();
    })
}

/// Sets the played part from fractions of the file: the very start is "no
/// start", the very end "no end". `duration` is the file's (1 if unknown,
/// as in the original).
pub fn set_range(cue: &Cue, start: f64, end: f64, duration: Option<f64>) -> Option<Cue> {
    let duration = duration.unwrap_or(1.0);
    with_audio(cue, |audio| {
        audio.start_time = (start != 0.0).then_some(start * duration);
        audio.end_time = (end != 1.0).then_some(end * duration);
    })
}

/// Both at once, as one drag of the waveform editor ends.
pub fn set_envelope_and_range(
    cue: &Cue,
    segments: &[Segment],
    start: f64,
    end: f64,
    duration: Option<f64>,
) -> Option<Cue> {
    let with_envelope = set_envelope(cue, segments);
    let base = with_envelope.as_ref().unwrap_or(cue);
    match set_range(base, start, end, duration) {
        Some(edited) => Some(edited),
        None => with_envelope,
    }
}

// ---------------------------------------------------------------------
// Fades

/// The three places a cue holds a fade parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeSlot {
    AudioIn,
    AudioOut,
    /// A fade cue's own fade.
    Fade,
}

pub fn fade_param(cue: &Cue, slot: FadeSlot) -> Option<FadeParam> {
    match (&cue.params, slot) {
        (CueParam::Audio(audio), FadeSlot::AudioIn) => audio.fade_in_param,
        (CueParam::Audio(audio), FadeSlot::AudioOut) => audio.fade_out_param,
        (CueParam::Fade(fade), FadeSlot::Fade) => Some(fade.fade_param),
        _ => None,
    }
}

/// Stores `param` in `slot`. A fade cue always has its fade, so `None`
/// is refused there.
pub fn set_fade_param(cue: &Cue, slot: FadeSlot, param: Option<FadeParam>) -> Option<Cue> {
    match (&cue.params, slot) {
        (CueParam::Audio(_), FadeSlot::AudioIn) => {
            with_audio(cue, |audio| audio.fade_in_param = param)
        }
        (CueParam::Audio(_), FadeSlot::AudioOut) => {
            with_audio(cue, |audio| audio.fade_out_param = param)
        }
        (CueParam::Fade(_), FadeSlot::Fade) => {
            let param = param?;
            edited(cue, |cue| {
                if let CueParam::Fade(fade) = &mut cue.params {
                    fade.fade_param = param;
                }
            })
        }
        _ => None,
    }
}

/// What a newly enabled fade starts as.
pub const DEFAULT_FADE: FadeParam = FadeParam {
    duration: 3.0,
    easing: Easing::InOutPow(2.0),
};

/// How the intensity field behaves: at least 1, in steps of 1.
pub const FADE_POWER: NumberSpec = NumberSpec {
    min: Some(1.0),
    max: None,
    step: 1.0,
    precision: 1,
    accept_null: false,
};

pub fn fade_enabled(current: Option<FadeParam>, enabled: bool) -> Option<FadeParam> {
    if enabled {
        current.or(Some(DEFAULT_FADE))
    } else {
        None
    }
}

pub fn fade_with_duration(param: FadeParam, text: &str) -> Option<FadeParam> {
    match commit_time(text, Some(param.duration), &TimeSpec::default()) {
        Commit::Value(duration) => Some(FadeParam { duration, ..param }),
        Commit::Unchanged | Commit::Null => None,
    }
}

/// Changing the curve keeps the intensity (2 if there was none yet);
/// a linear fade has none.
pub fn fade_with_kind(param: FadeParam, kind: CurveKind) -> Option<FadeParam> {
    let current = Curve::from(param.easing);
    if current.kind == kind {
        return None;
    }
    let power = match kind {
        CurveKind::Linear => None,
        _ => Some(current.power.unwrap_or(2.0)),
    };
    Some(FadeParam {
        easing: Curve { kind, power }.into(),
        ..param
    })
}

pub fn fade_with_power(param: FadeParam, text: &str) -> Option<FadeParam> {
    let current = Curve::from(param.easing);
    if current.kind == CurveKind::Linear {
        return None;
    }
    match commit_number(text, current.power, &FADE_POWER) {
        Commit::Value(power) => Some(FadeParam {
            easing: Curve {
                kind: current.kind,
                power: Some(power),
            }
            .into(),
            ..param
        }),
        Commit::Unchanged | Commit::Null => None,
    }
}

/// The intensity field's +/- buttons.
pub fn fade_step_power(param: FadeParam, direction: i8) -> Option<FadeParam> {
    let current = Curve::from(param.easing);
    if current.kind == CurveKind::Linear {
        return None;
    }
    let power = step_number(current.power, &FADE_POWER, direction);
    (Some(power) != current.power).then(|| FadeParam {
        easing: Curve {
            kind: current.kind,
            power: Some(power),
        }
        .into(),
        ..param
    })
}

// ---------------------------------------------------------------------
// Fade cue, playback cues, group

pub fn set_fade_target(cue: &Cue, target: Uuid) -> Option<Cue> {
    if !matches!(cue.params, CueParam::Fade(_)) {
        return None;
    }
    edited(cue, |cue| {
        if let CueParam::Fade(fade) = &mut cue.params {
            fade.target = target;
        }
    })
}

/// "Stop the target when the fade completes".
pub fn set_fade_stop_on_complete(cue: &Cue, stop: bool) -> Option<Cue> {
    if !matches!(cue.params, CueParam::Fade(_)) {
        return None;
    }
    edited(cue, |cue| {
        if let CueParam::Fade(fade) = &mut cue.params {
            fade.stop_target_on_complete = stop;
        }
    })
}

pub fn set_fade_volume(cue: &Cue, decibels: f64) -> Option<Cue> {
    if !matches!(cue.params, CueParam::Fade(_)) {
        return None;
    }
    edited(cue, |cue| {
        if let CueParam::Fade(fade) = &mut cue.params {
            fade.volume = Decibels::from(decibels as f32);
        }
    })
}

pub fn fade_volume(cue: &Cue) -> Option<f64> {
    match &cue.params {
        CueParam::Fade(fade) => Some(f64::from(f32::from(fade.volume))),
        _ => None,
    }
}

/// The target of a start, stop, pause or load cue.
pub fn playback_target(cue: &Cue) -> Option<Uuid> {
    match &cue.params {
        CueParam::Start(param) => Some(param.target),
        CueParam::Stop(param) => Some(param.target),
        CueParam::Pause(param) => Some(param.target),
        CueParam::Load(param) => Some(param.target),
        _ => None,
    }
}

pub fn set_playback_target(cue: &Cue, target: Uuid) -> Option<Cue> {
    playback_target(cue)?;
    edited(cue, |cue| match &mut cue.params {
        CueParam::Start(param) => param.target = target,
        CueParam::Stop(param) => param.target = target,
        CueParam::Pause(param) => param.target = target,
        CueParam::Load(param) => param.target = target,
        _ => {}
    })
}

/// "Stop without fading".
pub fn set_stop_hard(cue: &Cue, hard: bool) -> Option<Cue> {
    if !matches!(cue.params, CueParam::Stop(_)) {
        return None;
    }
    edited(cue, |cue| {
        if let CueParam::Stop(param) = &mut cue.params {
            param.hard = hard;
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupModeKind {
    Playlist,
    Concurrency,
    StartFirst,
}

pub fn group_mode(cue: &Cue) -> Option<GroupMode> {
    match &cue.params {
        CueParam::Group { base, .. } => Some(base.mode.clone()),
        _ => None,
    }
}

pub fn group_mode_kind(mode: &GroupMode) -> GroupModeKind {
    match mode {
        GroupMode::Playlist { .. } => GroupModeKind::Playlist,
        GroupMode::Concurrency => GroupModeKind::Concurrency,
        GroupMode::StartFirst { .. } => GroupModeKind::StartFirst,
    }
}

/// A new mode starts with its own defaults: a playlist repeats, "start
/// first" does not advance the cursor into the group.
pub fn set_group_mode_kind(cue: &Cue, kind: GroupModeKind) -> Option<Cue> {
    let current = group_mode(cue)?;
    if group_mode_kind(&current) == kind {
        return None;
    }
    set_group_mode(
        cue,
        match kind {
            GroupModeKind::Playlist => GroupMode::Playlist { repeat: true },
            GroupModeKind::Concurrency => GroupMode::Concurrency,
            GroupModeKind::StartFirst => GroupMode::StartFirst { enter: false },
        },
    )
}

pub fn set_group_repeat(cue: &Cue, repeat: bool) -> Option<Cue> {
    match group_mode(cue)? {
        GroupMode::Playlist { .. } => set_group_mode(cue, GroupMode::Playlist { repeat }),
        _ => None,
    }
}

pub fn set_group_enter(cue: &Cue, enter: bool) -> Option<Cue> {
    match group_mode(cue)? {
        GroupMode::StartFirst { .. } => set_group_mode(cue, GroupMode::StartFirst { enter }),
        _ => None,
    }
}

fn set_group_mode(cue: &Cue, mode: GroupMode) -> Option<Cue> {
    edited(cue, |cue| {
        if let CueParam::Group { base, .. } = &mut cue.params {
            base.mode = mode;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::{CueCursorAdvanceTriggerOverride as Advance, WaitCueParam};
    use sbsp_backend::model::settings::CursorAdvanceTrigger;

    fn wait(duration: f64) -> Cue {
        Cue {
            id: Uuid::from_u128(1),
            number: "1".to_string(),
            name: Some("Intro".to_string()),
            notes: String::new(),
            color: CueColor::default(),
            pre_wait: 0.0,
            chain: CueChain::default(),
            treat_stop_as_completed: false,
            cursor_advance_trigger_override: Advance::default(),
            parent_id: None,
            params: CueParam::Wait(WaitCueParam { duration }),
        }
    }

    #[test]
    fn an_unchanged_value_makes_no_update() {
        let cue = wait(1.0);
        assert!(set_number(&cue, "1").is_none());
        assert!(set_name(&cue, "Intro").is_none());
        assert!(set_color(&cue, CueColor::None).is_none());
        assert!(set_chain_kind(&cue, ChainKind::DoNotChain).is_none());
    }

    #[test]
    fn number_and_notes_are_stored_as_typed() {
        let cue = wait(1.0);
        assert_eq!(set_number(&cue, "2.5A").unwrap().number, "2.5A");
        assert_eq!(set_number(&cue, "").unwrap().number, "");
        assert_eq!(set_notes(&cue, "a\nb").unwrap().notes, "a\nb");
    }

    #[test]
    fn an_empty_name_goes_back_to_the_generated_one() {
        let cue = wait(1.0);
        assert_eq!(set_name(&cue, "Outro").unwrap().name.as_deref(), Some("Outro"));
        assert_eq!(set_name(&cue, "").unwrap().name, None);
    }

    #[test]
    fn pre_wait_parses_a_time() {
        let cue = wait(1.0);
        assert_eq!(set_pre_wait(&cue, "00:02.50").unwrap().pre_wait, 2.5);
        assert!(set_pre_wait(&cue, "00:00.00").is_none(), "same as now");
    }

    #[test]
    fn only_wait_and_fade_cues_have_a_typable_duration() {
        let cue = wait(1.0);
        assert!(duration_is_editable(&cue));
        assert_eq!(set_duration(&cue, "00:05.00").unwrap().params, {
            CueParam::Wait(WaitCueParam { duration: 5.0 })
        });

        let mut start = cue.clone();
        start.params = CueParam::Start(sbsp_backend::model::cue::StartCueParam {
            target: Uuid::from_u128(2),
        });
        assert!(!duration_is_editable(&start));
        assert!(set_duration(&start, "00:05.00").is_none());
    }

    #[test]
    fn chain_mode_changes_keep_the_target_between_chaining_modes() {
        let target = Uuid::from_u128(9);
        let mut cue = wait(1.0);
        cue.chain = CueChain::AfterStart {
            target_id: Some(target),
        };

        let after_complete = set_chain_kind(&cue, ChainKind::AfterComplete).unwrap();
        assert_eq!(chain_kind(&after_complete.chain), ChainKind::AfterComplete);
        assert_eq!(chain_target(&after_complete.chain), Some(target));

        let none = set_chain_kind(&cue, ChainKind::DoNotChain).unwrap();
        assert_eq!(chain_target(&none.chain), None);
        let back = set_chain_kind(&none, ChainKind::AfterStart).unwrap();
        assert_eq!(chain_target(&back.chain), None, "gone with the old mode");
    }

    #[test]
    fn a_chain_target_needs_a_chaining_mode() {
        let target = Uuid::from_u128(9);
        let cue = wait(1.0);
        assert!(set_chain_target(&cue, Some(target)).is_none());

        let mut chained = cue.clone();
        chained.chain = CueChain::AfterComplete { target_id: None };
        let set = set_chain_target(&chained, Some(target)).unwrap();
        assert_eq!(chain_target(&set.chain), Some(target));
        let cleared = set_chain_target(&set, None).unwrap();
        assert_eq!(chain_target(&cleared.chain), None);
    }

    #[test]
    fn trigger_settings_are_stored() {
        let cue = wait(1.0);
        let advance = Advance::Override(CursorAdvanceTrigger::OnCompleted);
        assert_eq!(
            set_cursor_advance_override(&cue, advance)
                .unwrap()
                .cursor_advance_trigger_override,
            advance
        );
        assert!(set_treat_stop_as_completed(&cue, true).unwrap().treat_stop_as_completed);
    }

    #[test]
    fn a_timestamp_starts_a_new_line() {
        let cue = wait(1.0);
        assert_eq!(append_timestamp(&cue, 65.5).unwrap().notes, "[01:05.50] ");

        let mut noted = cue.clone();
        noted.notes = "first".to_string();
        assert_eq!(
            append_timestamp(&noted, 1.0).unwrap().notes,
            "first\n[00:01.00] "
        );

        noted.notes = "first\n".to_string();
        assert_eq!(
            append_timestamp(&noted, 1.0).unwrap().notes,
            "first\n[00:01.00] "
        );
    }

    fn audio() -> Cue {
        let mut cue = wait(1.0);
        cue.params = CueParam::Audio(AudioCueParam {
            target: PathBuf::from("/a.wav"),
            start_time: None,
            fade_in_param: None,
            end_time: None,
            fade_out_param: None,
            volume: Decibels::default(),
            pan: 0.0,
            repeat: false,
            sound_type: SoundType::default(),
            envelope: Vec::new(),
        });
        cue
    }

    #[test]
    fn a_fade_can_stop_its_target_when_it_completes() {
        let cue = fade_cue();
        let stopping = set_fade_stop_on_complete(&cue, true).unwrap();
        assert!(matches!(&stopping.params, CueParam::Fade(f) if f.stop_target_on_complete));
        assert!(set_fade_stop_on_complete(&wait(1.0), true).is_none());
    }

    fn fade_cue() -> Cue {
        let mut cue = wait(1.0);
        cue.params = CueParam::Fade(sbsp_backend::model::cue::FadeCueParam {
            target: Uuid::from_u128(5),
            volume: Decibels::default(),
            fade_param: DEFAULT_FADE,
            stop_target_on_complete: false,
        });
        cue
    }

    #[test]
    fn start_and_end_follow_the_files_duration() {
        let cue = audio();
        assert_eq!(audio_start(&cue), Some(0.0));
        assert_eq!(audio_end(&cue, Some(10.0)), Some(10.0));
        assert_eq!(audio_end(&cue, None), None);

        let started = set_start_time(&cue, "2", Some(10.0)).unwrap();
        assert!(matches!(&started.params, CueParam::Audio(a) if a.start_time == Some(2.0)));
        assert!(set_start_time(&started, "2", Some(10.0)).is_none(), "unchanged");
        // Back to 0 is "no start".
        let reset = set_start_time(&started, "0", Some(10.0)).unwrap();
        assert!(matches!(&reset.params, CueParam::Audio(a) if a.start_time.is_none()));
        // The start cannot pass the end.
        let clamped = set_start_time(&cue, "99", Some(10.0)).unwrap();
        assert!(matches!(&clamped.params, CueParam::Audio(a) if a.start_time == Some(10.0)));

        let ended = set_end_time(&cue, "8", Some(10.0)).unwrap();
        assert!(matches!(&ended.params, CueParam::Audio(a) if a.end_time == Some(8.0)));
        // The duration itself, or an empty field, is "no end".
        let full = set_end_time(&ended, "10", Some(10.0)).unwrap();
        assert!(matches!(&full.params, CueParam::Audio(a) if a.end_time.is_none()));
        let emptied = set_end_time(&ended, "", Some(10.0)).unwrap();
        assert!(matches!(&emptied.params, CueParam::Audio(a) if a.end_time.is_none()));
        // The end cannot go before the start.
        let before = set_end_time(&started, "1", Some(10.0)).unwrap();
        assert!(matches!(&before.params, CueParam::Audio(a) if a.end_time == Some(2.0)));
        assert!(set_start_time(&wait(1.0), "1", None).is_none());
    }

    #[test]
    fn level_is_the_volume_of_audio_and_fade_cues() {
        let cue = audio();
        assert_eq!(level(&cue), Some(0.0));
        let quieter = set_level(&cue, -6.0).unwrap();
        assert_eq!(level(&quieter), Some(-6.0));
        assert!(set_level(&quieter, -6.0).is_none());

        let fade = fade_cue();
        assert_eq!(level(&fade), Some(0.0));
        assert_eq!(level(&set_level(&fade, -3.0).unwrap()), Some(-3.0));
        assert_eq!(level(&wait(1.0)), None);
        assert!(set_level(&wait(1.0), -3.0).is_none());
    }

    #[test]
    fn volume_matches_lufs_and_peak() {
        assert_eq!(volume_for_lufs(-14.0, -20.0), 6.0);
        assert_eq!(volume_for_lufs(-14.0, -10.0), -4.0);
        assert_eq!(volume_for_peak(-6.0), 6.0);
        assert_eq!(volume_for_peak(0.0), 0.0);
    }

    #[test]
    fn pan_and_repeat_edit_audio_cues() {
        let cue = audio();
        assert_eq!(pan(&cue), Some(0.0));
        let right = set_pan(&cue, 0.5).unwrap();
        assert_eq!(pan(&right), Some(0.5));
        assert_eq!(pan(&set_pan(&cue, 3.0).unwrap()), Some(1.0), "clamped");
        assert!(set_pan(&right, 0.5).is_none());
        assert_eq!(pan(&set_pan_text(&cue, "-0.25").unwrap()), Some(-0.25));
        assert!(set_pan_text(&cue, "abc").is_none());

        let repeated = set_repeat(&cue, true).unwrap();
        assert!(matches!(&repeated.params, CueParam::Audio(a) if a.repeat));
        assert!(set_repeat(&repeated, true).is_none());
        assert!(set_repeat(&wait(1.0), true).is_none());
    }

    #[test]
    fn the_envelope_and_the_played_part_are_stored_as_the_editor_shows_them() {
        let cue = audio();
        assert!(envelope(&cue).is_empty());

        let segments = [
            Segment { start: 0.0, end: 0.5, volume: -3.0 },
            Segment { start: 0.6, end: 1.0, volume: -9.0 },
        ];
        let with = set_envelope(&cue, &segments).unwrap();
        assert_eq!(envelope(&with), segments.to_vec());
        assert!(set_envelope(&with, &segments).is_none());
        assert!(set_envelope(&wait(1.0), &segments).is_none());

        // 0.25 .. 0.75 of a 10 s file.
        let ranged = set_range(&cue, 0.25, 0.75, Some(10.0)).unwrap();
        assert!(matches!(&ranged.params, CueParam::Audio(a) if a.start_time == Some(2.5) && a.end_time == Some(7.5)));
        // The very start and end are "none".
        let full = set_range(&ranged, 0.0, 1.0, Some(10.0)).unwrap();
        assert!(matches!(&full.params, CueParam::Audio(a) if a.start_time.is_none() && a.end_time.is_none()));
        assert!(set_range(&cue, 0.0, 1.0, Some(10.0)).is_none());

        // One drag changes both; one of them changing is enough.
        let both = set_envelope_and_range(&cue, &segments, 0.5, 1.0, Some(10.0)).unwrap();
        assert_eq!(envelope(&both), segments.to_vec());
        assert!(matches!(&both.params, CueParam::Audio(a) if a.start_time == Some(5.0)));
        let only_range = set_envelope_and_range(&both, &segments, 0.4, 1.0, Some(10.0)).unwrap();
        assert!(matches!(&only_range.params, CueParam::Audio(a) if a.start_time == Some(4.0)));
        let only_envelope = set_envelope_and_range(&both, &[], 0.5, 1.0, Some(10.0)).unwrap();
        assert!(envelope(&only_envelope).is_empty());
        assert!(set_envelope_and_range(&both, &segments, 0.5, 1.0, Some(10.0)).is_none());
    }

    #[test]
    fn audio_target_and_sound_type_apply_to_audio_cues_only() {
        let cue = audio();
        let moved = set_audio_target(&cue, PathBuf::from("/b.wav")).unwrap();
        assert!(matches!(&moved.params, CueParam::Audio(a) if a.target == *"/b.wav"));
        assert!(set_audio_target(&cue, PathBuf::from("/a.wav")).is_none());
        assert!(set_audio_target(&wait(1.0), PathBuf::from("/b.wav")).is_none());

        let loaded = set_sound_static(&cue, true).unwrap();
        assert!(matches!(&loaded.params, CueParam::Audio(a) if a.sound_type == SoundType::Static));
        assert!(set_sound_static(&cue, false).is_none(), "streaming already");
    }

    #[test]
    fn an_audio_cue_can_switch_its_fades_on_and_off() {
        let cue = audio();
        assert_eq!(fade_param(&cue, FadeSlot::AudioIn), None);

        let on = set_fade_param(&cue, FadeSlot::AudioIn, fade_enabled(None, true)).unwrap();
        assert_eq!(fade_param(&on, FadeSlot::AudioIn), Some(DEFAULT_FADE));
        assert_eq!(fade_param(&on, FadeSlot::AudioOut), None, "the other slot is untouched");

        let off = set_fade_param(&on, FadeSlot::AudioIn, fade_enabled(Some(DEFAULT_FADE), false))
            .unwrap();
        assert_eq!(fade_param(&off, FadeSlot::AudioIn), None);
    }

    #[test]
    fn a_fade_cue_always_keeps_its_fade() {
        let cue = fade_cue();
        assert!(set_fade_param(&cue, FadeSlot::Fade, None).is_none());
        let longer = FadeParam {
            duration: 5.0,
            ..DEFAULT_FADE
        };
        let edited = set_fade_param(&cue, FadeSlot::Fade, Some(longer)).unwrap();
        assert_eq!(fade_param(&edited, FadeSlot::Fade), Some(longer));
        assert!(set_fade_param(&cue, FadeSlot::AudioIn, Some(longer)).is_none());
    }

    #[test]
    fn fade_duration_parses_a_time() {
        let param = fade_with_duration(DEFAULT_FADE, "00:07.50").unwrap();
        assert_eq!(param.duration, 7.5);
        assert!(fade_with_duration(DEFAULT_FADE, "00:03.00").is_none());
    }

    #[test]
    fn changing_the_curve_keeps_or_drops_the_intensity() {
        let linear = fade_with_kind(DEFAULT_FADE, CurveKind::Linear).unwrap();
        assert_eq!(linear.easing, Easing::Linear);

        let back = fade_with_kind(linear, CurveKind::OutPow).unwrap();
        assert_eq!(back.easing, Easing::OutPow(2.0), "linear has none, so the default");

        let steeper = fade_with_power(back, "3").unwrap();
        let in_pow = fade_with_kind(steeper, CurveKind::InPow).unwrap();
        assert_eq!(in_pow.easing, Easing::InPow(3.0), "kept");

        assert!(fade_with_kind(in_pow, CurveKind::InPow).is_none());
    }

    #[test]
    fn the_intensity_is_at_least_one_and_linear_has_none() {
        assert_eq!(
            fade_with_power(DEFAULT_FADE, "0.2").unwrap().easing,
            Easing::InOutPow(1.0)
        );
        let linear = FadeParam {
            easing: Easing::Linear,
            ..DEFAULT_FADE
        };
        assert!(fade_with_power(linear, "3").is_none());
        assert!(fade_step_power(linear, 1).is_none());

        assert_eq!(fade_step_power(DEFAULT_FADE, 1).unwrap().easing, Easing::InOutPow(3.0));
        let one = FadeParam {
            easing: Easing::InOutPow(1.0),
            ..DEFAULT_FADE
        };
        assert!(fade_step_power(one, -1).is_none(), "already at the floor");
    }

    #[test]
    fn fade_cue_target_and_volume() {
        let cue = fade_cue();
        let target = Uuid::from_u128(6);
        assert!(matches!(
            &set_fade_target(&cue, target).unwrap().params,
            CueParam::Fade(f) if f.target == target
        ));
        assert!(set_fade_target(&cue, Uuid::from_u128(5)).is_none());

        let quieter = set_fade_volume(&cue, -12.0).unwrap();
        assert_eq!(fade_volume(&quieter), Some(-12.0));
        assert!(set_fade_volume(&quieter, -12.0).is_none());
        assert!(set_fade_volume(&wait(1.0), -12.0).is_none());
    }

    #[test]
    fn playback_cues_have_a_target_and_stop_has_hard() {
        let mut cue = wait(1.0);
        cue.params = CueParam::Stop(sbsp_backend::model::cue::StopCueParam {
            target: Uuid::from_u128(5),
            hard: false,
        });
        assert_eq!(playback_target(&cue), Some(Uuid::from_u128(5)));

        let retargeted = set_playback_target(&cue, Uuid::from_u128(6)).unwrap();
        assert_eq!(playback_target(&retargeted), Some(Uuid::from_u128(6)));

        assert!(set_fade_stop_on_complete(&cue, true).is_none());
        let hard = set_stop_hard(&cue, true).unwrap();
        assert!(matches!(&hard.params, CueParam::Stop(s) if s.hard));

        let mut start = cue.clone();
        start.params = CueParam::Start(sbsp_backend::model::cue::StartCueParam {
            target: Uuid::from_u128(5),
        });
        assert!(set_stop_hard(&start, true).is_none());
        assert!(set_playback_target(&wait(1.0), Uuid::from_u128(6)).is_none());
    }

    fn group(mode: GroupMode) -> Cue {
        let mut cue = wait(1.0);
        cue.params = CueParam::Group {
            base: sbsp_backend::model::cue::group::GroupCueParamBase { mode },
            children: Vec::new(),
        };
        cue
    }

    #[test]
    fn a_new_group_mode_starts_with_its_defaults() {
        let playlist = group(GroupMode::Playlist { repeat: false });

        let start_first = set_group_mode_kind(&playlist, GroupModeKind::StartFirst).unwrap();
        assert_eq!(group_mode(&start_first), Some(GroupMode::StartFirst { enter: false }));

        let back = set_group_mode_kind(&start_first, GroupModeKind::Playlist).unwrap();
        assert_eq!(group_mode(&back), Some(GroupMode::Playlist { repeat: true }));

        assert!(set_group_mode_kind(&back, GroupModeKind::Playlist).is_none());
        assert!(set_group_mode_kind(&wait(1.0), GroupModeKind::Concurrency).is_none());
    }

    #[test]
    fn repeat_and_enter_belong_to_their_own_modes() {
        let playlist = group(GroupMode::Playlist { repeat: true });
        let once = set_group_repeat(&playlist, false).unwrap();
        assert_eq!(group_mode(&once), Some(GroupMode::Playlist { repeat: false }));
        assert!(set_group_enter(&playlist, true).is_none());

        let start_first = group(GroupMode::StartFirst { enter: false });
        let entering = set_group_enter(&start_first, true).unwrap();
        assert_eq!(group_mode(&entering), Some(GroupMode::StartFirst { enter: true }));
        assert!(set_group_repeat(&start_first, true).is_none());
    }
}
