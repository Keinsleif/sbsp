// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The cue tree as a flat list, for list rendering and cursor movement.
//!
//! The cue list is stored as a tree: a map of cues by id, a list of root
//! ids, and a group cue holding the ids of its children. A list view wants
//! one row per cue in display order, so [`flatten`] walks the tree depth
//! first (a group's children follow it) and yields a [`FlatCueEntry`] per
//! cue with what a row needs: its nesting level, parent, index among its
//! siblings, whether an ancestor group hides it, and the chain setting that
//! applies to it. [`next_cue_id`] finds the cue after a given one in the
//! same tree order, climbing out of a group when its last child is reached.
//!
//! A [`FlatCueEntry`] holds the cue's id rather than a reference or a copy.
//! The flat list is recomputed from the model and kept across `update` and
//! `view` calls, so it must not borrow from the cue map; the caller looks the
//! cue up by id when it draws the row. Rows under a collapsed group stay in
//! the list, marked `is_hidden`, so that selection and the playback cursor
//! keep pointing at cues that are not on screen.

use std::collections::{HashMap, HashSet};

use sbsp_backend::model::cue::{Cue, CueChain, CueParam, Uuid, group::GroupMode};

#[derive(Debug, Clone, Copy)]
pub struct GroupInfo {
    pub is_expanded: bool,
}

#[derive(Debug, Clone)]
pub struct FlatCueEntry {
    pub cue_id: Uuid,
    pub level: usize,
    pub parent: Option<Uuid>,
    /// This cue's index within its parent's (or the root list's) children.
    pub inner_index: usize,
    /// True once an ancestor group is collapsed, so the row exists in the
    /// flat list (keeping selection/cursor state stable) but should not be
    /// drawn.
    pub is_hidden: bool,
    /// The cue's effective chain: its own stored `chain`, unless an
    /// enclosing playlist/concurrency group overrides it (see
    /// `is_chain_overridden`).
    pub chain: CueChain,
    pub is_chain_overridden: bool,
    /// `Some` for a group cue (carrying its expanded state), `None`
    /// otherwise.
    pub group: Option<GroupInfo>,
}

/// Flattens the cue tree (depth-first, children following their parent) for
/// list rendering.
pub fn flatten(
    root_ids: &[Uuid],
    cues: &HashMap<Uuid, Cue>,
    expanded: &HashSet<Uuid>,
) -> Vec<FlatCueEntry> {
    flatten_level(root_ids, cues, expanded, 0, false, None)
}

fn flatten_level(
    ids: &[Uuid],
    cues: &HashMap<Uuid, Cue>,
    expanded: &HashSet<Uuid>,
    level: usize,
    is_hidden: bool,
    parent: Option<&Cue>,
) -> Vec<FlatCueEntry> {
    let mut result = Vec::with_capacity(ids.len());

    for (index, cue_id) in ids.iter().enumerate() {
        let Some(cue) = cues.get(cue_id) else {
            // An id with no cue behind it (the model should not produce one) is skipped.
            continue;
        };

        let chain_override = chain_override(parent, ids, index);

        let group = if let CueParam::Group { children, .. } = &cue.params {
            Some((children, expanded.contains(&cue.id)))
        } else {
            None
        };

        result.push(FlatCueEntry {
            cue_id: cue.id,
            level,
            parent: parent.map(|p| p.id),
            inner_index: index,
            is_hidden,
            chain: chain_override.unwrap_or(cue.chain),
            is_chain_overridden: chain_override.is_some(),
            group: group.map(|(_, is_expanded)| GroupInfo { is_expanded }),
        });

        if let Some((children, is_expanded)) = group {
            result.extend(flatten_level(
                children,
                cues,
                expanded,
                level + 1,
                !is_expanded || is_hidden,
                Some(cue),
            ));
        }
    }

    result
}

/// The chain a playlist or concurrency group imposes on its `index`-th
/// child out of `ids` (a playlist chains each child to the next and, with
/// `repeat`, the last back to the first; a concurrency group chains
/// nothing). `None` when there is no parent or the group does not override
/// chaining (`StartFirst`), so the cue keeps its own chain.
fn chain_override(parent: Option<&Cue>, ids: &[Uuid], index: usize) -> Option<CueChain> {
    let parent = parent?;
    let CueParam::Group { base, .. } = &parent.params else {
        return None;
    };

    match &base.mode {
        GroupMode::Playlist { repeat } => {
            if index + 1 == ids.len() {
                if *repeat {
                    Some(CueChain::AfterComplete {
                        target_id: ids.first().copied(),
                    })
                } else {
                    Some(CueChain::DoNotChain)
                }
            } else {
                Some(CueChain::AfterComplete { target_id: None })
            }
        }
        GroupMode::Concurrency => Some(CueChain::DoNotChain),
        GroupMode::StartFirst { .. } => None,
    }
}

/// The cue that would be selected next after `start`, for cursor-advance:
/// the next sibling, or (if `start` is the last child of its group) the
/// next sibling of the nearest ancestor that has one, walking up to the
/// root. `None` if there is no next cue in that direction.
pub fn next_cue_id(start: Uuid, cues: &HashMap<Uuid, Cue>, root_ids: &[Uuid]) -> Option<Uuid> {
    let mut current = start;

    loop {
        let cue = cues.get(&current)?;

        if let Some(parent_id) = cue.parent_id {
            let parent = cues.get(&parent_id)?;
            if let CueParam::Group { children, .. } = &parent.params
                && let Some(idx) = children.iter().position(|id| *id == current)
            {
                if let Some(&next_id) = children.get(idx + 1) {
                    return Some(next_id);
                }
                current = parent_id;
                continue;
            }
            return None;
        }

        let idx = root_ids.iter().position(|id| *id == current)?;
        return root_ids.get(idx + 1).copied();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::{
        CueColor, CueCursorAdvanceTriggerOverride, WaitCueParam, group::GroupCueParamBase,
    };

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

    fn group(id: Uuid, parent_id: Option<Uuid>, mode: GroupMode, children: Vec<Uuid>) -> Cue {
        Cue {
            params: CueParam::Group {
                base: GroupCueParamBase { mode },
                children,
            },
            ..leaf(id, parent_id)
        }
    }

    /// a, b (group: c, d)
    fn sample() -> (Vec<Uuid>, HashMap<Uuid, Cue>, Uuid, Uuid, Uuid, Uuid) {
        let (a, b, c, d) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let mut cues = HashMap::new();
        cues.insert(a, leaf(a, None));
        cues.insert(
            b,
            group(b, None, GroupMode::Playlist { repeat: false }, vec![c, d]),
        );
        cues.insert(c, leaf(c, Some(b)));
        cues.insert(d, leaf(d, Some(b)));
        (vec![a, b], cues, a, b, c, d)
    }

    #[test]
    fn flatten_depth_first_order() {
        let (root_ids, cues, a, b, c, d) = sample();
        let flat = flatten(&root_ids, &cues, &HashSet::new());
        let ids: Vec<Uuid> = flat.iter().map(|e| e.cue_id).collect();
        assert_eq!(ids, vec![a, b, c, d]);
    }

    #[test]
    fn collapsed_group_hides_children_not_itself() {
        let (root_ids, cues, _a, b, _c, _d) = sample();
        let flat = flatten(&root_ids, &cues, &HashSet::new());
        let group_entry = flat.iter().find(|e| e.cue_id == b).unwrap();
        assert!(!group_entry.is_hidden);
        assert!(flat.iter().filter(|e| e.level == 1).all(|e| e.is_hidden));
    }

    #[test]
    fn expanded_group_shows_children() {
        let (root_ids, cues, _a, b, _c, _d) = sample();
        let expanded = HashSet::from([b]);
        let flat = flatten(&root_ids, &cues, &expanded);
        assert!(flat.iter().filter(|e| e.level == 1).all(|e| !e.is_hidden));
    }

    #[test]
    fn playlist_chains_non_last_child_after_complete() {
        let (root_ids, cues, _a, _b, c, _d) = sample();
        let flat = flatten(&root_ids, &cues, &HashSet::new());
        let c_entry = flat.iter().find(|e| e.cue_id == c).unwrap();
        assert!(c_entry.is_chain_overridden);
        assert_eq!(c_entry.chain, CueChain::AfterComplete { target_id: None });
    }

    #[test]
    fn playlist_last_child_without_repeat_does_not_chain() {
        let (root_ids, cues, _a, _b, _c, d) = sample();
        let flat = flatten(&root_ids, &cues, &HashSet::new());
        let d_entry = flat.iter().find(|e| e.cue_id == d).unwrap();
        assert_eq!(d_entry.chain, CueChain::DoNotChain);
    }

    #[test]
    fn playlist_last_child_with_repeat_chains_to_first() {
        let (a, b, c, d) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let mut cues = HashMap::new();
        cues.insert(
            b,
            group(b, None, GroupMode::Playlist { repeat: true }, vec![c, d]),
        );
        cues.insert(c, leaf(c, Some(b)));
        cues.insert(d, leaf(d, Some(b)));
        let _ = a;

        let flat = flatten(&[b], &cues, &HashSet::new());
        let d_entry = flat.iter().find(|e| e.cue_id == d).unwrap();
        assert_eq!(
            d_entry.chain,
            CueChain::AfterComplete { target_id: Some(c) }
        );
    }

    #[test]
    fn concurrency_never_chains() {
        let (a, b, c, d) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let mut cues = HashMap::new();
        cues.insert(b, group(b, None, GroupMode::Concurrency, vec![c, d]));
        cues.insert(c, leaf(c, Some(b)));
        cues.insert(d, leaf(d, Some(b)));
        let _ = a;

        let flat = flatten(&[b], &cues, &HashSet::new());
        for entry in flat.iter().filter(|e| e.level == 1) {
            assert_eq!(entry.chain, CueChain::DoNotChain);
        }
    }

    #[test]
    fn start_first_group_does_not_override_chain() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let mut cues = HashMap::new();
        let mut own_chain_child = leaf(c, Some(b));
        own_chain_child.chain = CueChain::AfterStart { target_id: None };
        cues.insert(
            b,
            group(b, None, GroupMode::StartFirst { enter: true }, vec![c]),
        );
        cues.insert(c, own_chain_child);
        let _ = a;

        let flat = flatten(&[b], &cues, &HashSet::new());
        let c_entry = flat.iter().find(|e| e.cue_id == c).unwrap();
        assert!(!c_entry.is_chain_overridden);
        assert_eq!(c_entry.chain, CueChain::AfterStart { target_id: None });
    }

    #[test]
    fn next_cue_id_within_root() {
        let (root_ids, cues, a, b, _c, _d) = sample();
        assert_eq!(next_cue_id(a, &cues, &root_ids), Some(b));
        assert_eq!(next_cue_id(b, &cues, &root_ids), None);
    }

    #[test]
    fn next_cue_id_within_group() {
        let (root_ids, cues, _a, _b, c, d) = sample();
        assert_eq!(next_cue_id(c, &cues, &root_ids), Some(d));
    }

    #[test]
    fn next_cue_id_walks_up_past_last_child_of_group() {
        // a, group(c, d), e -- next after d (last child of the group)
        // should be e, the group's own next sibling at the root.
        let (a, b, c, d) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let e = Uuid::new_v4();
        let mut cues = HashMap::new();
        cues.insert(a, leaf(a, None));
        cues.insert(
            b,
            group(b, None, GroupMode::Playlist { repeat: false }, vec![c, d]),
        );
        cues.insert(c, leaf(c, Some(b)));
        cues.insert(d, leaf(d, Some(b)));
        cues.insert(e, leaf(e, None));
        let root_ids = vec![a, b, e];

        assert_eq!(next_cue_id(d, &cues, &root_ids), Some(e));
    }
}
