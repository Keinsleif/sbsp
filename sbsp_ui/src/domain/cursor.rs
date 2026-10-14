// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Playback-cursor advance: where the cursor goes when the cue under it
//! is triggered or completes. Ported from the `cueStatus` case of
//! `useBackendEvent` in `MainView.vue`.

use std::collections::HashMap;

use sbsp_backend::model::cue::{
    Cue, CueCursorAdvanceTriggerOverride, CueParam, Uuid, group::GroupMode,
};
use sbsp_backend::model::settings::CursorAdvanceTrigger;

use crate::domain::flat_list::next_cue_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueEvent {
    Triggered,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMove {
    /// The event does not advance the cursor.
    Stay,
    /// Move the cursor here; `None` means past the end of the list, which
    /// clears it (as the original does).
    To(Option<Uuid>),
}

/// What `event` on `cue` does to the playback cursor, assuming the cursor
/// is on `cue` (checking that is the caller's job). The cue's own
/// override wins over the show-wide `default_trigger`.
pub fn advance(
    cue: &Cue,
    cues: &HashMap<Uuid, Cue>,
    root_ids: &[Uuid],
    default_trigger: CursorAdvanceTrigger,
    event: CueEvent,
) -> CursorMove {
    let trigger = match cue.cursor_advance_trigger_override {
        CueCursorAdvanceTriggerOverride::None => default_trigger,
        CueCursorAdvanceTriggerOverride::Override(trigger) => trigger,
    };

    let advances = matches!(
        (trigger, event),
        (CursorAdvanceTrigger::OnTriggered, CueEvent::Triggered)
            | (CursorAdvanceTrigger::OnCompleted, CueEvent::Completed)
    );
    if !advances {
        return CursorMove::Stay;
    }

    let next = match &cue.params {
        // Entering a start-first group: its first child is what the group
        // itself starts, so the cursor goes to the second.
        CueParam::Group { base, children }
            if matches!(base.mode, GroupMode::StartFirst { enter: true }) =>
        {
            children
                .get(1)
                .copied()
                .or_else(|| next_cue_id(cue.id, cues, root_ids))
        }
        _ => next_cue_id(cue.id, cues, root_ids),
    };

    CursorMove::To(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::{CueChain, CueColor, WaitCueParam, group::GroupCueParamBase};

    fn leaf(id: Uuid, parent_id: Option<Uuid>) -> Cue {
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
            params: CueParam::Wait(WaitCueParam { duration: 1.0 }),
        }
    }

    /// root: a, b
    fn two() -> (HashMap<Uuid, Cue>, Vec<Uuid>, Uuid, Uuid) {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let cues = HashMap::from([(a, leaf(a, None)), (b, leaf(b, None))]);
        (cues, vec![a, b], a, b)
    }

    #[test]
    fn manual_never_advances() {
        let (cues, roots, a, _b) = two();
        for event in [CueEvent::Triggered, CueEvent::Completed] {
            assert_eq!(
                advance(
                    &cues[&a],
                    &cues,
                    &roots,
                    CursorAdvanceTrigger::Manual,
                    event
                ),
                CursorMove::Stay
            );
        }
    }

    #[test]
    fn on_triggered_advances_on_trigger_only() {
        let (cues, roots, a, b) = two();
        let go = |event| {
            advance(
                &cues[&a],
                &cues,
                &roots,
                CursorAdvanceTrigger::OnTriggered,
                event,
            )
        };

        assert_eq!(go(CueEvent::Triggered), CursorMove::To(Some(b)));
        assert_eq!(go(CueEvent::Completed), CursorMove::Stay);
    }

    #[test]
    fn on_completed_advances_on_completion_only() {
        let (cues, roots, a, b) = two();
        let go = |event| {
            advance(
                &cues[&a],
                &cues,
                &roots,
                CursorAdvanceTrigger::OnCompleted,
                event,
            )
        };

        assert_eq!(go(CueEvent::Completed), CursorMove::To(Some(b)));
        assert_eq!(go(CueEvent::Triggered), CursorMove::Stay);
    }

    #[test]
    fn the_cues_own_override_beats_the_show_setting() {
        let (mut cues, roots, a, b) = two();
        cues.get_mut(&a).unwrap().cursor_advance_trigger_override =
            CueCursorAdvanceTriggerOverride::Override(CursorAdvanceTrigger::OnTriggered);

        assert_eq!(
            advance(
                &cues[&a],
                &cues,
                &roots,
                CursorAdvanceTrigger::Manual,
                CueEvent::Triggered
            ),
            CursorMove::To(Some(b))
        );
    }

    #[test]
    fn past_the_last_cue_the_cursor_clears() {
        let (cues, roots, _a, b) = two();

        assert_eq!(
            advance(
                &cues[&b],
                &cues,
                &roots,
                CursorAdvanceTrigger::OnTriggered,
                CueEvent::Triggered
            ),
            CursorMove::To(None)
        );
    }

    #[test]
    fn entering_a_start_first_group_skips_its_first_child() {
        let (g, c1, c2, after) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let group = Cue {
            params: CueParam::Group {
                base: GroupCueParamBase {
                    mode: GroupMode::StartFirst { enter: true },
                },
                children: vec![c1, c2],
            },
            ..leaf(g, None)
        };
        let cues = HashMap::from([
            (g, group),
            (c1, leaf(c1, Some(g))),
            (c2, leaf(c2, Some(g))),
            (after, leaf(after, None)),
        ]);
        let roots = vec![g, after];

        assert_eq!(
            advance(
                &cues[&g],
                &cues,
                &roots,
                CursorAdvanceTrigger::OnTriggered,
                CueEvent::Triggered
            ),
            CursorMove::To(Some(c2))
        );
    }

    #[test]
    fn a_plain_group_advances_to_the_next_sibling() {
        let (g, c1, after) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let group = Cue {
            params: CueParam::Group {
                base: GroupCueParamBase {
                    mode: GroupMode::Concurrency,
                },
                children: vec![c1],
            },
            ..leaf(g, None)
        };
        let cues = HashMap::from([
            (g, group),
            (c1, leaf(c1, Some(g))),
            (after, leaf(after, None)),
        ]);
        let roots = vec![g, after];

        assert_eq!(
            advance(
                &cues[&g],
                &cues,
                &roots,
                CursorAdvanceTrigger::OnTriggered,
                CueEvent::Triggered
            ),
            CursorMove::To(Some(after))
        );
    }
}
