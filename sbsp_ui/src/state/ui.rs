// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `stores/uiState.ts`. Scoped to permission/mode, selection,
//! the playback cursor, and expand/collapse -- the parts the migration
//! plan's Phase 2 explicitly calls out ("選択・カーソル制御"). Everything
//! else in the original (dialog-open flags, the file list resolver, the
//! sidebar/bottom-tab toggles, envelope/waveform display flags, the
//! last-update-check date) is left for whichever later phase actually
//! implements that screen or feature, rather than guessing its shape now.
//!
//! `lock_cursor_to_selection` (read from `uiSettings` in the original, via
//! another Pinia store) is taken as an explicit parameter instead, per the
//! earlier discussion on cross-cutting settings: pass the value in rather
//! than reach across to another piece of state.
//!
//! `selected_rows` is a `Vec<Uuid>`, not a `HashSet`: `remove_from_selected`
//! needs "the first-inserted remaining id" when the removed id was the
//! active selection, which relies on JS `Set`'s insertion-order iteration.
//! `std::collections::HashSet` has no such guarantee, so a `Vec` (checked
//! linearly for membership) is used instead to keep that exact behavior.
//! `expanded_rows` has no such ordering need and stays a `HashSet`.

use std::collections::HashMap;
use std::collections::HashSet;

use sbsp_backend::api::Permissions;
use sbsp_backend::model::cue::{Cue, Uuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Edit,
    Run,
    View,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Elapsed,
    Remain,
}

impl DisplayMode {
    pub fn toggle(self) -> Self {
        match self {
            DisplayMode::Elapsed => DisplayMode::Remain,
            DisplayMode::Remain => DisplayMode::Elapsed,
        }
    }
}

pub struct UiState {
    pub permission: Permissions,
    pub mode: Mode,
    pub playback_cursor: Option<Uuid>,
    pub selected: Option<Uuid>,
    pub selected_rows: Vec<Uuid>,
    pub expanded_rows: HashSet<Uuid>,
    pub pre_wait_display_mode: DisplayMode,
    pub duration_display_mode: DisplayMode,
}

impl UiState {
    /// `is_host` selects the original's `INITIAL_PERMISSION`: a host starts
    /// with every permission (it *is* the backend); a remote starts with
    /// none, until the host grants some over the connection.
    pub fn new(is_host: bool) -> Self {
        let permission = if is_host {
            Permissions::READ | Permissions::CONTROL | Permissions::EDIT
        } else {
            Permissions::empty()
        };

        Self {
            permission,
            mode: Mode::Edit,
            playback_cursor: None,
            selected: None,
            selected_rows: Vec::new(),
            expanded_rows: HashSet::new(),
            pre_wait_display_mode: DisplayMode::Elapsed,
            duration_display_mode: DisplayMode::Elapsed,
        }
    }

    /// The permission `self.mode` requires to stay valid.
    pub fn mode_as_perm(&self) -> Permissions {
        match self.mode {
            Mode::Edit => Permissions::EDIT,
            Mode::Run => Permissions::CONTROL,
            Mode::View => Permissions::READ,
        }
    }

    pub fn set_permission(&mut self, permission: Permissions) {
        self.permission = permission;
        self.validate_mode();
    }

    /// Drops `self.mode` to the highest permission still held, if the
    /// current mode's permission was just revoked. If no permission is
    /// held at all, `mode` is left as-is (matching the original: its
    /// `highestBitPos` is `-1` for zero permissions, which doesn't match
    /// any of its `switch` cases, so `mode` is never written in that case
    /// -- not a gap, this is deliberately preserved).
    fn validate_mode(&mut self) {
        if self.permission.intersects(self.mode_as_perm()) {
            return;
        }

        if self.permission.contains(Permissions::EDIT) {
            self.mode = Mode::Edit;
        } else if self.permission.contains(Permissions::CONTROL) {
            self.mode = Mode::Run;
        } else if self.permission.contains(Permissions::READ) {
            self.mode = Mode::View;
        }
    }

    pub fn set_playback_cursor(
        &mut self,
        id: Option<Uuid>,
        lock_cursor_to_selection: bool,
        cues: &HashMap<Uuid, Cue>,
    ) {
        self.playback_cursor = id;

        if !lock_cursor_to_selection {
            return;
        }

        match id {
            Some(cue_id) if self.selected != Some(cue_id) => {
                self.selected = Some(cue_id);
                self.expand_to_visible(cue_id, cues);
                // Not using add_selected: that would also call
                // try_update_playback_cursor, re-entering this cursor
                // update pointlessly.
                if !self.selected_rows.contains(&cue_id) {
                    self.selected_rows.clear();
                    self.selected_rows.push(cue_id);
                }
            }
            Some(_) => {}
            None => {
                self.selected_rows.clear();
                self.selected = None;
            }
        }
    }

    fn try_update_playback_cursor(&mut self, id: Option<Uuid>, lock_cursor_to_selection: bool) {
        if lock_cursor_to_selection {
            self.playback_cursor = id;
        }
    }

    pub fn reset_selected(&mut self) {
        self.selected = None;
        self.selected_rows.clear();
    }

    pub fn clear_selected(&mut self, lock_cursor_to_selection: bool) {
        self.reset_selected();
        self.try_update_playback_cursor(None, lock_cursor_to_selection);
    }

    pub fn set_selected(&mut self, id: Uuid, lock_cursor_to_selection: bool) {
        self.selected = Some(id);
        self.selected_rows.clear();
        self.selected_rows.push(id);
        self.try_update_playback_cursor(Some(id), lock_cursor_to_selection);
    }

    pub fn add_selected(&mut self, id: Uuid, lock_cursor_to_selection: bool) {
        self.selected = Some(id);
        if !self.selected_rows.contains(&id) {
            self.selected_rows.push(id);
        }
        self.try_update_playback_cursor(Some(id), lock_cursor_to_selection);
    }

    pub fn remove_from_selected(&mut self, ids: &[Uuid], lock_cursor_to_selection: bool) {
        let mut removed_current = false;

        for id in ids {
            self.selected_rows.retain(|existing| existing != id);
            if !removed_current && Some(*id) == self.selected {
                removed_current = true;
            }
        }

        if removed_current {
            self.selected = self.selected_rows.first().copied();
            self.try_update_playback_cursor(self.selected, lock_cursor_to_selection);
        }
    }

    /// Selects every cue between `anchor` and `target` (inclusive) in
    /// `visible_order` (the flat list's id order, with collapsed-away rows
    /// already filtered out by the caller -- range selection should span
    /// what's visibly between the two rows, not skip past a collapsed
    /// group's hidden children only to land past them). `target` becomes
    /// `selected`; `anchor` is typically the previous `selected`, found by
    /// the caller before this replaces it.
    ///
    /// This has no equivalent in the original `uiState.ts`: the Vue
    /// frontend's shift-click range logic lived in `CueList.vue` itself,
    /// not the store, so there was nothing to port here -- this is new
    /// logic for Phase 4, not a port.
    pub fn select_range(
        &mut self,
        anchor: Uuid,
        target: Uuid,
        visible_order: &[Uuid],
        lock_cursor_to_selection: bool,
    ) {
        let Some(anchor_pos) = visible_order.iter().position(|id| *id == anchor) else {
            // Anchor no longer visible (e.g. its group just got collapsed);
            // fall back to a plain single selection rather than guessing
            // at a range from nowhere.
            self.set_selected(target, lock_cursor_to_selection);
            return;
        };
        let Some(target_pos) = visible_order.iter().position(|id| *id == target) else {
            return;
        };

        let (from, to) = if anchor_pos <= target_pos {
            (anchor_pos, target_pos)
        } else {
            (target_pos, anchor_pos)
        };

        self.selected_rows = visible_order[from..=to].to_vec();
        self.selected = Some(target);
        self.try_update_playback_cursor(Some(target), lock_cursor_to_selection);
    }

    pub fn toggle_expand(&mut self, id: Uuid) {
        if !self.expanded_rows.remove(&id) {
            self.expanded_rows.insert(id);
        }
    }

    /// Expands every ancestor group of `id` so it is visible in the flat
    /// list, without changing whether `id` itself (if it is a group) is
    /// expanded.
    pub fn expand_to_visible(&mut self, id: Uuid, cues: &HashMap<Uuid, Cue>) {
        let mut current = Some(id);

        while let Some(target_id) = current {
            match cues.get(&target_id).and_then(|cue| cue.parent_id) {
                Some(parent_id) => {
                    self.expanded_rows.insert(parent_id);
                    current = Some(parent_id);
                }
                None => current = None,
            }
        }
    }

    pub fn toggle_pre_wait_display_mode(&mut self) {
        self.pre_wait_display_mode = self.pre_wait_display_mode.toggle();
    }

    pub fn toggle_duration_display_mode(&mut self) {
        self.duration_display_mode = self.duration_display_mode.toggle();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf_with_parent(id: Uuid, parent_id: Option<Uuid>) -> Cue {
        use sbsp_backend::model::cue::{
            CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, WaitCueParam,
        };
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

    #[test]
    fn host_starts_with_every_permission_remote_with_none() {
        assert_eq!(UiState::new(true).permission, Permissions::all());
        assert_eq!(UiState::new(false).permission, Permissions::empty());
    }

    #[test]
    fn losing_edit_permission_drops_to_run_then_view() {
        let mut state = UiState::new(true);
        assert_eq!(state.mode, Mode::Edit);

        state.set_permission(Permissions::READ | Permissions::CONTROL);
        assert_eq!(state.mode, Mode::Run);

        state.set_permission(Permissions::READ);
        assert_eq!(state.mode, Mode::View);
    }

    #[test]
    fn losing_all_permission_leaves_mode_unchanged() {
        let mut state = UiState::new(true);
        state.mode = Mode::View;
        state.set_permission(Permissions::empty());
        // No case matches zero permissions in the original either: mode is
        // simply never written, not reset to some default.
        assert_eq!(state.mode, Mode::View);
    }

    #[test]
    fn set_selected_replaces_selection() {
        let mut state = UiState::new(true);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        state.set_selected(a, false);
        state.set_selected(b, false);

        assert_eq!(state.selected, Some(b));
        assert_eq!(state.selected_rows, vec![b]);
    }

    #[test]
    fn add_selected_extends_selection_in_insertion_order() {
        let mut state = UiState::new(true);
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());

        state.add_selected(a, false);
        state.add_selected(b, false);

        assert_eq!(state.selected, Some(b));
        assert_eq!(state.selected_rows, vec![a, b]);
    }

    #[test]
    fn remove_from_selected_falls_back_to_first_remaining() {
        let mut state = UiState::new(true);
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        state.add_selected(a, false);
        state.add_selected(b, false);
        state.add_selected(c, false);

        // Current selection (`selected`) is `c`; removing it should fall
        // back to the first remaining row in insertion order, `a`.
        state.remove_from_selected(&[c], false);

        assert_eq!(state.selected, Some(a));
        assert_eq!(state.selected_rows, vec![a, b]);
    }

    #[test]
    fn remove_from_selected_leaves_selection_alone_if_not_removed() {
        let mut state = UiState::new(true);
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        state.add_selected(a, false);
        state.add_selected(b, false);

        state.remove_from_selected(&[a], false);

        assert_eq!(state.selected, Some(b));
        assert_eq!(state.selected_rows, vec![b]);
    }

    #[test]
    fn set_playback_cursor_locks_selection_when_enabled() {
        let mut state = UiState::new(true);
        let id = Uuid::new_v4();
        let cues = HashMap::new();

        state.set_playback_cursor(Some(id), true, &cues);

        assert_eq!(state.playback_cursor, Some(id));
        assert_eq!(state.selected, Some(id));
        assert_eq!(state.selected_rows, vec![id]);
    }

    #[test]
    fn set_playback_cursor_does_not_touch_selection_when_disabled() {
        let mut state = UiState::new(true);
        let id = Uuid::new_v4();
        let cues = HashMap::new();

        state.set_playback_cursor(Some(id), false, &cues);

        assert_eq!(state.playback_cursor, Some(id));
        assert_eq!(state.selected, None);
    }

    #[test]
    fn expand_to_visible_walks_up_ancestors() {
        let mut state = UiState::new(true);
        let (grandparent, parent, child) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let mut cues = HashMap::new();
        cues.insert(grandparent, leaf_with_parent(grandparent, None));
        cues.insert(parent, leaf_with_parent(parent, Some(grandparent)));
        cues.insert(child, leaf_with_parent(child, Some(parent)));

        state.expand_to_visible(child, &cues);

        assert!(state.expanded_rows.contains(&parent));
        assert!(state.expanded_rows.contains(&grandparent));
        assert!(!state.expanded_rows.contains(&child));
    }

    #[test]
    fn select_range_forward_and_backward_cover_the_same_ids() {
        let ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let (a, b, c, d) = (ids[0], ids[1], ids[2], ids[3]);

        let mut forward = UiState::new(true);
        forward.select_range(b, d, &ids, false);
        assert_eq!(forward.selected_rows, vec![b, c, d]);
        assert_eq!(forward.selected, Some(d));

        let mut backward = UiState::new(true);
        backward.select_range(d, b, &ids, false);
        assert_eq!(backward.selected_rows, vec![b, c, d]);
        assert_eq!(backward.selected, Some(b));

        let _ = a;
    }

    #[test]
    fn select_range_with_missing_anchor_falls_back_to_single_selection() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let hidden_anchor = Uuid::new_v4();

        let mut state = UiState::new(true);
        state.select_range(hidden_anchor, ids[1], &ids, false);

        assert_eq!(state.selected_rows, vec![ids[1]]);
        assert_eq!(state.selected, Some(ids[1]));
    }

    #[test]
    fn select_range_with_missing_target_is_a_no_op() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();

        let mut state = UiState::new(true);
        state.set_selected(ids[0], false);
        state.select_range(ids[0], Uuid::new_v4(), &ids, false);

        assert_eq!(state.selected_rows, vec![ids[0]]);
        assert_eq!(state.selected, Some(ids[0]));
    }

    #[test]
    fn toggle_expand_is_a_simple_flip() {
        let mut state = UiState::new(true);
        let id = Uuid::new_v4();

        state.toggle_expand(id);
        assert!(state.expanded_rows.contains(&id));

        state.toggle_expand(id);
        assert!(!state.expanded_rows.contains(&id));
    }
}
