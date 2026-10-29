// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! What copy, cut, paste and duplicate do with cues. Ported from
//! `CueList.vue`.
//!
//! The clipboard is the app's own list of cues, not the system's: cues
//! are not text, and the original kept them in a variable too. A pasted
//! cue is a copy with a new id; the backend gives it its place in the
//! tree (its parent, and an empty child list if it is a group), so a copied
//! group arrives without its children, as in the original.

use std::collections::HashMap;

use sbsp_backend::manager::InsertPosition;
use sbsp_backend::model::cue::{Cue, Uuid};

use super::flat_list::FlatCueEntry;

/// The selected cues in the order the list shows them (not the order they
/// were selected in).
pub fn selected_cues(
    flat: &[FlatCueEntry],
    cues: &HashMap<Uuid, Cue>,
    selected: &[Uuid],
) -> Vec<Cue> {
    flat.iter()
        .filter(|entry| selected.contains(&entry.cue_id))
        .filter_map(|entry| cues.get(&entry.cue_id).cloned())
        .collect()
}

/// The clipboard's cues as new ones: same content, new ids.
pub fn for_paste(cues: &[Cue], mut new_id: impl FnMut() -> Uuid) -> Vec<Cue> {
    cues.iter()
        .map(|cue| Cue {
            id: new_id(),
            ..cue.clone()
        })
        .collect()
}

/// Where duplicates go: after the last of the selected cues in list order.
pub fn duplicate_position(flat: &[FlatCueEntry], selected: &[Uuid]) -> Option<InsertPosition> {
    flat.iter()
        .rev()
        .find(|entry| selected.contains(&entry.cue_id))
        .map(|entry| InsertPosition::After {
            target: entry.cue_id,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::flat_list::flatten;
    use sbsp_backend::model::cue::{
        CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, WaitCueParam,
    };
    use std::collections::HashSet;

    fn cue(id: Uuid, name: &str) -> Cue {
        Cue {
            id,
            number: String::new(),
            name: Some(name.to_string()),
            notes: String::new(),
            color: CueColor::default(),
            pre_wait: 0.0,
            chain: CueChain::default(),
            treat_stop_as_completed: false,
            cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::default(),
            parent_id: None,
            params: CueParam::Wait(WaitCueParam { duration: 1.0 }),
        }
    }

    fn list() -> (Vec<Uuid>, HashMap<Uuid, Cue>) {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let cues = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, cue(*id, &format!("cue {i}"))))
            .collect();
        (ids, cues)
    }

    #[test]
    fn the_selection_is_copied_in_list_order() {
        let (ids, cues) = list();
        let flat = flatten(&ids, &cues, &HashSet::new());

        // Selected last-first; copied top-first.
        let copied = selected_cues(&flat, &cues, &[ids[2], ids[0]]);
        let copied_ids: Vec<Uuid> = copied.iter().map(|cue| cue.id).collect();
        assert_eq!(copied_ids, vec![ids[0], ids[2]]);

        assert!(selected_cues(&flat, &cues, &[]).is_empty());
    }

    #[test]
    fn pasted_cues_are_the_same_with_new_ids() {
        let (ids, cues) = list();
        let flat = flatten(&ids, &cues, &HashSet::new());
        let copied = selected_cues(&flat, &cues, &ids);

        let pasted = for_paste(&copied, Uuid::new_v4);
        assert_eq!(pasted.len(), 3);
        for (before, after) in copied.iter().zip(&pasted) {
            assert_ne!(before.id, after.id);
            assert_eq!(before.name, after.name);
            assert_eq!(before.params, after.params);
        }
        let unique: HashSet<Uuid> = pasted.iter().map(|cue| cue.id).collect();
        assert_eq!(unique.len(), 3, "each gets its own id");
    }

    #[test]
    fn duplicates_go_after_the_last_selected_cue() {
        let (ids, cues) = list();
        let flat = flatten(&ids, &cues, &HashSet::new());

        let position = duplicate_position(&flat, &[ids[1], ids[0]]);
        assert!(matches!(
            position,
            Some(InsertPosition::After { target }) if target == ids[1]
        ));
        assert!(duplicate_position(&flat, &[]).is_none());
    }
}
