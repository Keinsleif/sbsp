// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The show model as the UI holds it.
//!
//! `sbsp_backend::model::ShowModel` already has the shape the UI needs
//! (`cue_list.cues: HashMap<Uuid, Cue>` and `cue_list.root_ids:
//! Vec<Uuid>`), so there is no state struct of its own: [`ShowModelState`]
//! is that type. A cue is found with `model.cue_list.cues.get(id)`; the
//! list in display order and the cue after a given one come from
//! [`crate::domain::flat_list::flatten`] and
//! [`crate::domain::flat_list::next_cue_id`], called on `model.cue_list`;
//! a new model from the backend replaces the old one as a whole.

use sbsp_backend::model::ShowModel;
use sbsp_backend::model::cue::Cue;

use crate::domain::flat_list::flatten;
use crate::state::ui::UiState;

pub type ShowModelState = ShowModel;

/// The `Cue`s currently selected, in flat-list (depth-first, visual) order.
///
/// This flattens the whole list, so it is not cheap; avoid calling it every
/// frame. Walking `selected_rows` directly would skip that cost and also
/// skip stale ids, but it would give selection order rather than tree
/// order.
pub fn selected_cues<'a>(model: &'a ShowModel, ui: &UiState) -> Vec<&'a Cue> {
    let flat = flatten(
        &model.cue_list.root_ids,
        &model.cue_list.cues,
        &ui.expanded_rows,
    );

    flat.into_iter()
        .filter(|entry| ui.selected_rows.contains(&entry.cue_id))
        .filter_map(|entry| model.cue_list.cues.get(&entry.cue_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::{
        CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid, WaitCueParam,
    };

    fn leaf(id: Uuid) -> Cue {
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
            parent_id: None,
            params: CueParam::Wait(WaitCueParam { duration: 1.0 }),
        }
    }

    #[test]
    fn selected_cues_filters_and_orders_by_flat_list() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let mut model = ShowModel::default();
        model.cue_list.cues.insert(a, leaf(a));
        model.cue_list.cues.insert(b, leaf(b));
        model.cue_list.cues.insert(c, leaf(c));
        model.cue_list.root_ids = vec![a, b, c];

        let mut ui = UiState::new(true);
        // Select in an order different from tree order.
        ui.add_selected(c, false);
        ui.add_selected(a, false);

        let selected = selected_cues(&model, &ui);
        let ids: Vec<Uuid> = selected.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![a, c]);
    }

    #[test]
    fn selected_cues_drops_stale_ids() {
        let a = Uuid::new_v4();
        let mut model = ShowModel::default();
        model.cue_list.cues.insert(a, leaf(a));
        model.cue_list.root_ids = vec![a];

        let mut ui = UiState::new(true);
        ui.add_selected(a, false);
        ui.add_selected(Uuid::new_v4(), false); // not in the model at all

        assert_eq!(selected_cues(&model, &ui).len(), 1);
    }
}
