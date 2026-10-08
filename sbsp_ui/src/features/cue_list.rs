// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The cue list (Phase 4): a read-only tree view with selection and
//! group expand/collapse. Replaces the Vue frontend's `CueList.vue` /
//! `CueListRow.vue` for display and click-selection only -- keyboard
//! movement, hotkeys, drag-and-drop reordering, the context menu and
//! inline editing are separate, later pieces of Phase 4/6.
//!
//! This module owns no state of its own: selection and expansion live in
//! [`UiState`] (they have to, other screens read them too), the cues in
//! [`ShowModelState`], and playback status in [`PlaybackState`]. It only
//! turns those into an `Element` and reports what the user clicked as a
//! [`Message`]; the app decides what to do with it (in particular, which
//! of replace/extend/toggle/range a click means depends on the keyboard
//! modifiers, which this widget cannot see).

use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length, Theme};

use sbsp_backend::controller::state::PlaybackStatus;
use sbsp_backend::model::cue::{Cue, CueColor, Uuid};
use sbsp_frontend_settings::NameFormatSettings;

use crate::domain::cue::build_cue_name;
use crate::domain::flat_list::{FlatCueEntry, flatten};
use crate::fl;
use crate::state::model::ShowModelState;
use crate::state::playback::PlaybackState;
use crate::state::ui::UiState;

const INDENT_PER_LEVEL: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    Select(Uuid),
    ToggleExpand(Uuid),
}

/// The ids of the rows actually drawn, top to bottom: the flat list with
/// collapsed-away rows removed. This is the order a shift-click range
/// selection should span (see [`UiState::select_range`]).
pub fn visible_order(model: &ShowModelState, ui: &UiState) -> Vec<Uuid> {
    flatten(
        &model.cue_list.root_ids,
        &model.cue_list.cues,
        &ui.expanded_rows,
    )
    .into_iter()
    .filter(|entry| !entry.is_hidden)
    .map(|entry| entry.cue_id)
    .collect()
}

pub fn view<'a>(
    model: &'a ShowModelState,
    ui: &'a UiState,
    playback: &'a PlaybackState,
    name_format: &NameFormatSettings,
) -> Element<'a, Message> {
    let flat = flatten(
        &model.cue_list.root_ids,
        &model.cue_list.cues,
        &ui.expanded_rows,
    );

    let rows: Vec<Element<'a, Message>> = flat
        .iter()
        .filter(|entry| !entry.is_hidden)
        .filter_map(|entry| {
            let cue = model.cue_list.cues.get(&entry.cue_id)?;
            Some(row_view(entry, cue, model, ui, playback, name_format))
        })
        .collect();

    if rows.is_empty() {
        return container(text(fl!("cue-list-empty")))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    }

    scrollable(column(rows).width(Length::Fill))
        .height(Length::Fill)
        .into()
}

fn row_view<'a>(
    entry: &FlatCueEntry,
    cue: &'a Cue,
    model: &'a ShowModelState,
    ui: &UiState,
    playback: &PlaybackState,
    name_format: &NameFormatSettings,
) -> Element<'a, Message> {
    let id = entry.cue_id;
    let is_selected = ui.selected_rows.contains(&id);
    let is_cursor = ui.playback_cursor == Some(id);

    let number = text(if cue.number.is_empty() {
        "-".to_string()
    } else {
        cue.number.clone()
    })
    .width(48);

    let indent = Space::new().width(entry.level as f32 * INDENT_PER_LEVEL);

    let disclosure: Element<'a, Message> = match entry.group {
        Some(group) => button(text(if group.is_expanded { "▾" } else { "▸" }))
            .padding(0)
            .width(16)
            .style(button::text)
            .on_press(Message::ToggleExpand(id))
            .into(),
        None => Space::new().width(16).into(),
    };

    let color = cue_color(cue.color);
    let color_dot = container(Space::new())
        .width(10)
        .height(10)
        .style(move |_theme: &Theme| container::Style {
            background: Some(color.into()),
            border: Border {
                radius: 5.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let status = text(
        playback
            .active_cues
            .get(&id)
            .map(|active| status_glyph(active.status))
            .unwrap_or(""),
    )
    .width(20);

    let name = text(build_cue_name(
        Some(cue),
        &model.cue_list.cues,
        name_format,
    ))
    .width(Length::Fill);

    let content = row![number, status, indent, disclosure, color_dot, name]
        .spacing(8)
        .align_y(Alignment::Center);

    mouse_area(
        container(content)
            .padding([4, 8])
            .width(Length::Fill)
            .style(move |theme: &Theme| row_style(theme, is_selected, is_cursor)),
    )
    .on_press(Message::Select(id))
    .into()
}

fn row_style(theme: &Theme, is_selected: bool, is_cursor: bool) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: is_selected.then(|| palette.primary.weak.color.into()),
        text_color: is_selected.then_some(palette.primary.weak.text),
        // The playback cursor (the cue that fires next) is a separate
        // concept from the selection (what the user is editing), so it
        // gets its own marker rather than reusing the selection fill.
        border: if is_cursor {
            Border {
                color: palette.primary.strong.color,
                width: 2.0,
                radius: 0.0.into(),
            }
        } else {
            Border::default()
        },
        ..Default::default()
    }
}

/// A first-draft palette (Radix-ish hues), not a reproduction of the
/// PrimeVue-era colors -- same status as `theme::palette`: replace once
/// there is a visual pass on real screens.
fn cue_color(color: CueColor) -> Color {
    match color {
        CueColor::None => Color::TRANSPARENT,
        CueColor::Red => Color::from_rgb8(0xE5, 0x48, 0x4D),
        CueColor::Purple => Color::from_rgb8(0x8E, 0x4E, 0xC6),
        CueColor::Blue => Color::from_rgb8(0x3E, 0x63, 0xDD),
        CueColor::Cyan => Color::from_rgb8(0x05, 0xA2, 0xC2),
        CueColor::Green => Color::from_rgb8(0x30, 0xA4, 0x6C),
        CueColor::Yellow => Color::from_rgb8(0xF5, 0xD9, 0x0A),
        CueColor::Orange => Color::from_rgb8(0xF7, 0x6B, 0x15),
        CueColor::Grey => Color::from_rgb8(0x8B, 0x8D, 0x98),
    }
}

/// One glyph per playback status, kept to characters common to ordinary
/// system fonts (the same family as the disclosure triangles above, which
/// rendered fine in the Phase 0 prototype) rather than emoji-range
/// symbols that are more likely to show as tofu.
fn status_glyph(status: PlaybackStatus) -> &'static str {
    match status {
        PlaybackStatus::Loaded => "●",
        PlaybackStatus::PreWaiting | PlaybackStatus::PreWaitPaused => "…",
        PlaybackStatus::Playing => "▶",
        PlaybackStatus::Paused => "‖",
        PlaybackStatus::Stopping => "■",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::{
        CueChain, CueCursorAdvanceTriggerOverride, CueParam, WaitCueParam,
        group::{GroupCueParamBase, GroupMode},
    };
    use sbsp_backend::model::ShowModel;

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

    fn group(id: Uuid, children: Vec<Uuid>) -> Cue {
        Cue {
            params: CueParam::Group {
                base: GroupCueParamBase {
                    mode: GroupMode::Concurrency,
                },
                children,
            },
            ..leaf(id, None)
        }
    }

    /// root: a, group g (children c, d)
    fn sample() -> (ShowModel, Uuid, Uuid, Uuid, Uuid) {
        let (a, g, c, d) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let mut model = ShowModel::default();
        model.cue_list.cues.insert(a, leaf(a, None));
        model.cue_list.cues.insert(g, group(g, vec![c, d]));
        model.cue_list.cues.insert(c, leaf(c, Some(g)));
        model.cue_list.cues.insert(d, leaf(d, Some(g)));
        model.cue_list.root_ids = vec![a, g];
        (model, a, g, c, d)
    }

    #[test]
    fn visible_order_hides_children_of_a_collapsed_group() {
        let (model, a, g, _c, _d) = sample();
        let ui = UiState::new(true);

        assert_eq!(visible_order(&model, &ui), vec![a, g]);
    }

    #[test]
    fn visible_order_includes_children_once_expanded() {
        let (model, a, g, c, d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);

        assert_eq!(visible_order(&model, &ui), vec![a, g, c, d]);
    }

    #[test]
    fn every_status_has_a_distinct_glyph() {
        use std::collections::HashSet;

        let glyphs: HashSet<&str> = [
            PlaybackStatus::Loaded,
            PlaybackStatus::PreWaiting,
            PlaybackStatus::Playing,
            PlaybackStatus::Paused,
            PlaybackStatus::Stopping,
        ]
        .into_iter()
        .map(status_glyph)
        .collect();

        assert_eq!(glyphs.len(), 5);
    }

    #[test]
    fn no_color_is_fully_transparent_and_others_are_opaque() {
        assert_eq!(cue_color(CueColor::None).a, 0.0);
        assert_eq!(cue_color(CueColor::Red).a, 1.0);
    }
}
