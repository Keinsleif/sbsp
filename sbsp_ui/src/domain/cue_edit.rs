// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Changing one property of a cue, as the bottom editor's fields do.
//!
//! Every function takes the cue as it is and returns the edited copy, or
//! `None` when the edit would change nothing (so an unchanged field does
//! not send an update to the backend). The backend replaces a cue as a
//! whole (`update_cue`), as the original editor does.

use sbsp_backend::model::cue::{
    Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid,
};

use crate::domain::field::{Commit, TimeSpec, commit_time};
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
}
