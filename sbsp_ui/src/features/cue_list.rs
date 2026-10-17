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

use iced::widget::{
    Space, button, center, column, container, mouse_area, row, scrollable, stack, text,
};
use iced::{Alignment, Border, Color, Element, Length, Theme};

use sbsp_backend::controller::state::PlaybackStatus;
use sbsp_backend::model::cue::{Cue, CueChain, CueColor, CueParam, Uuid, group::GroupMode};
use sbsp_frontend_settings::NameFormatSettings;

use crate::domain::cue::{display_name, get_duration};
use crate::domain::time::format_seconds;
use crate::domain::flat_list::{FlatCueEntry, flatten};
use crate::fl;
use crate::state::assets::AssetResults;
use crate::state::model::ShowModelState;
use crate::state::playback::PlaybackState;
use crate::state::ui::{DisplayMode, UiState};
use crate::widgets::icon::{Icon, Tone, icon, toned};
use crate::widgets::progress;

const INDENT_PER_LEVEL: f32 = 20.0;

/// Rows are a fixed height so the scroll position of any row is just
/// `index * ROW_HEIGHT`: keeping a row in view then needs no layout
/// feedback from the widget tree, only the scrollable's own offset and
/// viewport height (see [`Viewport`]).
const ROW_HEIGHT: f32 = 30.0;

/// The scrollable's id, for scrolling it from outside.
pub const LIST_ID: &str = "cue-list";

/// What the list's scrollable last reported about itself.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Viewport {
    pub offset_y: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Select(Uuid),
    ToggleExpand(Uuid),
    Scrolled(Viewport),
    SetCursor(Uuid),
    TogglePreWaitDisplay,
    ToggleDurationDisplay,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
}

/// Moves the selection one visible row up/down (the arrow-key bindings),
/// or with `extend` adds that row to it. Returns the row that became the
/// selection, for scrolling it into view.
///
/// With nothing selected, Up lands on the first visible row and Down on
/// the last, as in the original. Extending only crosses to a row at the
/// same nesting level (the original refuses to extend a selection into or
/// out of a group), and does nothing at either end of the list.
pub fn move_cursor(
    model: &ShowModelState,
    ui: &mut UiState,
    step: Step,
    extend: bool,
    lock_cursor_to_selection: bool,
) -> Option<Uuid> {
    let rows: Vec<FlatCueEntry> = flatten(
        &model.cue_list.root_ids,
        &model.cue_list.cues,
        &ui.expanded_rows,
    )
    .into_iter()
    .filter(|entry| !entry.is_hidden)
    .collect();

    let current = ui
        .selected
        .and_then(|id| rows.iter().position(|entry| entry.cue_id == id));

    let target = match current {
        Some(index) => {
            let next = match step {
                Step::Up => index.checked_sub(1)?,
                Step::Down => index + 1,
            };
            let entry = rows.get(next)?;
            if extend && entry.level != rows[index].level {
                return None;
            }
            entry
        }
        None => match step {
            Step::Up => rows.first()?,
            Step::Down => rows.last()?,
        },
    };

    let id = target.cue_id;
    if extend {
        ui.add_selected(id, lock_cursor_to_selection);
    } else {
        ui.set_selected(id, lock_cursor_to_selection);
    }
    Some(id)
}

/// Selects every visible row. Like the original, this leaves `selected`
/// (the single "primary" row) alone.
/// The vertical scroll offset that brings `id`'s row fully into view, or
/// `None` if it already is (or the viewport size is not known yet, i.e.
/// nothing has been drawn). Scrolls the minimum distance: the row ends up
/// at whichever edge it was off.
pub fn scroll_offset_for(
    model: &ShowModelState,
    ui: &UiState,
    viewport: Viewport,
    id: Uuid,
) -> Option<f32> {
    if viewport.height <= 0.0 {
        return None;
    }

    let index = visible_order(model, ui).iter().position(|row| *row == id)?;
    let top = index as f32 * ROW_HEIGHT;
    let bottom = top + ROW_HEIGHT;

    if top < viewport.offset_y {
        Some(top)
    } else if bottom > viewport.offset_y + viewport.height {
        Some(bottom - viewport.height)
    } else {
        None
    }
}

pub fn select_all(model: &ShowModelState, ui: &mut UiState) {
    ui.selected_rows = visible_order(model, ui);
}

const W_CURSOR: f32 = 28.0;
const W_STATUS: f32 = 28.0;
const W_TYPE: f32 = 28.0;
const W_NUMBER: f32 = 54.0;
const W_TIME: f32 = 112.0;
const W_REPEAT: f32 = 28.0;
const W_CHAIN: f32 = 36.0;

/// The column header plus the rows. Column widths are shared constants
/// (cells below, header cells here), so the two line up without a table
/// widget.
pub fn view<'a>(
    model: &'a ShowModelState,
    ui: &'a UiState,
    playback: &'a PlaybackState,
    assets: &'a AssetResults,
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
            Some(row_view(entry, cue, model, ui, playback, assets, name_format))
        })
        .collect();

    let body: Element<'a, Message> = if rows.is_empty() {
        container(text(fl!("cue-list-empty")))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else {
        scrollable(column(rows).width(Length::Fill))
            .id(LIST_ID)
            .on_scroll(|viewport| {
                Message::Scrolled(Viewport {
                    offset_y: viewport.absolute_offset().y,
                    height: viewport.bounds().height,
                })
            })
            .height(Length::Fill)
            .into()
    };

    column![header(ui), body].into()
}

fn cell<'a>(content: impl Into<Element<'a, Message>>, width: f32) -> Element<'a, Message> {
    container(content).center_x(width).into()
}

fn header<'a>(ui: &UiState) -> Element<'a, Message> {
    let display_toggle = |mode: DisplayMode, message: Message| {
        button(icon(
            match mode {
                DisplayMode::Elapsed => Icon::AlphaEBoxOutline,
                DisplayMode::Remain => Icon::AlphaRBoxOutline,
            },
            20.0,
        ))
        .padding(0)
        .style(button::text)
        .on_press(message)
    };

    let time_header = |label: String, mode: DisplayMode, message: Message| {
        cell(
            row![text(label), display_toggle(mode, message)]
                .spacing(4)
                .align_y(Alignment::Center),
            W_TIME,
        )
    };

    row![
        Space::new().width(W_CURSOR + W_STATUS + W_TYPE),
        cell(text("#"), W_NUMBER),
        container(text(fl!("main-name"))).width(Length::Fill).padding([0, 8]),
        time_header(
            fl!("main-pre-wait"),
            ui.pre_wait_display_mode,
            Message::TogglePreWaitDisplay,
        ),
        time_header(
            fl!("main-duration"),
            ui.duration_display_mode,
            Message::ToggleDurationDisplay,
        ),
        cell(icon(Icon::Repeat, 18.0), W_REPEAT),
        cell(icon(Icon::ChevronDoubleDown, 18.0), W_CHAIN),
    ]
    .align_y(Alignment::Center)
    .padding([4, 8])
    .into()
}

fn row_view<'a>(
    entry: &FlatCueEntry,
    cue: &'a Cue,
    model: &'a ShowModelState,
    ui: &UiState,
    playback: &PlaybackState,
    assets: &AssetResults,
    name_format: &NameFormatSettings,
) -> Element<'a, Message> {
    let id = entry.cue_id;
    let is_selected = ui.selected_rows.contains(&id);
    let is_cursor = ui.playback_cursor == Some(id);
    let active = playback.active_cues.get(&id);

    let cursor_mark: Element<'a, Message> = if is_cursor {
        icon(Icon::ArrowRightBold, 18.0).into()
    } else {
        Space::new().into()
    };
    let cursor = button(container(cursor_mark).center_x(Length::Fill))
        .width(W_CURSOR)
        .padding(0)
        .style(button::text)
        .on_press(Message::SetCursor(id));

    let status: Element<'a, Message> = match active.and_then(|a| status_icon(a.status)) {
        Some((glyph, tone)) => toned(glyph, 18.0, tone).into(),
        None => Space::new().into(),
    };

    let indent = Space::new().width(entry.level as f32 * INDENT_PER_LEVEL);
    let disclosure: Element<'a, Message> = match entry.group {
        Some(group) => button(icon(
            if group.is_expanded {
                Icon::MenuDown
            } else {
                Icon::MenuRight
            },
            18.0,
        ))
            .padding(0)
            .width(20)
            .style(button::text)
            .on_press(Message::ToggleExpand(id))
            .into(),
        None => Space::new().width(20).into(),
    };
    let name = text(display_name(cue, &model.cue_list.cues, name_format)).width(Length::Fill);

    // While a cue is in its pre-wait the pre-wait cell is the live one and
    // the duration cell stays static; from playback on it flips, as in the
    // original.
    let in_pre_wait = active.is_some_and(|a| {
        matches!(a.status, PlaybackStatus::PreWaiting | PlaybackStatus::PreWaitPaused)
    });
    let in_playback = active.is_some_and(|a| {
        matches!(
            a.status,
            PlaybackStatus::Playing | PlaybackStatus::Paused | PlaybackStatus::Stopping
        )
    });
    let live = active.filter(|a| a.duration > 0.0);

    let pre_wait = match live.filter(|_| in_pre_wait) {
        Some(a) => live_time(a.position, a.duration, ui.pre_wait_display_mode),
        None => (
            format_seconds((cue.pre_wait != 0.0).then_some(cue.pre_wait)),
            0.0,
        ),
    };
    let duration = match live.filter(|_| !in_pre_wait) {
        Some(a) => live_time(a.position, a.duration, ui.duration_display_mode),
        None => (
            format_seconds(get_duration(Some(cue), |_| asset_duration(cue, assets))),
            0.0,
        ),
    };

    let repeats = match &cue.params {
        CueParam::Audio(audio) => audio.repeat,
        CueParam::Group { base, .. } => {
            matches!(base.mode, GroupMode::Playlist { repeat: true })
        }
        _ => false,
    };

    let repeat_mark: Element<'a, Message> = if repeats {
        icon(Icon::Repeat, 18.0).into()
    } else {
        Space::new().into()
    };
    let chain_mark: Element<'a, Message> = match chain_icon(&entry.chain) {
        Some(glyph) => icon(glyph, 18.0).into(),
        None => Space::new().into(),
    };

    let content = row![
        cursor,
        cell(status, W_STATUS),
        cell(icon(type_icon(&cue.params), 18.0), W_TYPE),
        cell(text(cue.number.clone()), W_NUMBER),
        row![indent, disclosure, name]
            .spacing(4)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        time_cell(pre_wait.0, pre_wait.1, in_pre_wait),
        time_cell(duration.0, duration.1, in_playback),
        cell(repeat_mark, W_REPEAT),
        cell(chain_mark, W_CHAIN),
    ]
    .align_y(Alignment::Center);

    let color = cue.color;
    mouse_area(
        container(content)
            .padding([0, 8])
            .center_y(ROW_HEIGHT)
            .width(Length::Fill)
            .style(move |theme: &Theme| row_style(theme, is_selected, is_cursor, color)),
    )
    .on_press(Message::Select(id))
    .into()
}

/// Label and progress (0..=1) for a running cue's time cell: the elapsed
/// or the remaining time, per the column's display mode.
fn live_time(position: f64, duration: f64, mode: DisplayMode) -> (String, f32) {
    let shown = match mode {
        DisplayMode::Elapsed => position,
        DisplayMode::Remain => duration - position,
    };
    (
        format_seconds(Some(shown)),
        (position / duration).clamp(0.0, 1.0) as f32,
    )
}

/// An audio cue's duration comes from its analyzed asset, if that has
/// arrived yet.
pub(crate) fn asset_duration(cue: &Cue, assets: &AssetResults) -> Option<f64> {
    match &cue.params {
        CueParam::Audio(audio) => assets
            .get_metadata(&audio.target)
            .and_then(|metadata| metadata.duration),
        _ => None,
    }
}

/// A time field with its progress fill behind the text. `active` outlines
/// it (the phase currently running).
fn time_cell<'a>(label: String, progress: f32, active: bool) -> Element<'a, Message> {
    let bar = progress::bar(progress, |theme: &Theme| {
        theme.extended_palette().primary.base.color.scale_alpha(0.5)
    });

    let field = container(stack![bar, center(text(label))])
        .width(Length::Fill)
        .height(ROW_HEIGHT * 0.75)
        .style(move |theme: &Theme| container::Style {
            border: if active {
                Border {
                    color: theme.extended_palette().primary.base.color,
                    width: 1.0,
                    radius: 2.0.into(),
                }
            } else {
                Border::default()
            },
            ..Default::default()
        });

    container(field)
        .padding([0, 4])
        .width(W_TIME)
        .center_y(ROW_HEIGHT)
        .into()
}

fn row_style(
    theme: &Theme,
    is_selected: bool,
    is_cursor: bool,
    color: CueColor,
) -> container::Style {
    let palette = theme.extended_palette();

    // Selection wins over the cue's own color tint, as in the original.
    let background = if is_selected {
        Some(palette.primary.weak.color.into())
    } else if color != CueColor::None {
        Some(cue_color(color).scale_alpha(0.2).into())
    } else {
        None
    };

    container::Style {
        background,
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

fn type_icon(param: &CueParam) -> Icon {
    match param {
        CueParam::Audio(_) => Icon::VolumeHigh,
        CueParam::Wait(_) => Icon::TimerSandEmpty,
        CueParam::Fade(_) => Icon::ChartBellCurveCumulative,
        CueParam::Start(_) => Icon::PlayCircleOutline,
        CueParam::Stop(_) => Icon::StopCircleOutline,
        CueParam::Pause(_) => Icon::PauseCircleOutline,
        CueParam::Load(_) => Icon::UploadCircleOutline,
        CueParam::Group { .. } => Icon::Group,
    }
}

fn chain_icon(chain: &CueChain) -> Option<Icon> {
    match chain {
        CueChain::DoNotChain => None,
        CueChain::AfterStart { .. } => Some(Icon::ArrowExpandDown),
        CueChain::AfterComplete { .. } => Some(Icon::ArrowCollapseDown),
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

/// The marker for a playback status, as in the original's status column;
/// pre-waiting reads as playing and its paused state as paused, since the
/// pre-wait cell itself is what tells the two apart.
fn status_icon(status: PlaybackStatus) -> Option<(Icon, Tone)> {
    Some(match status {
        PlaybackStatus::Loaded => (Icon::Upload, Tone::Warning),
        PlaybackStatus::PreWaiting | PlaybackStatus::Playing => (Icon::Play, Tone::Success),
        PlaybackStatus::PreWaitPaused | PlaybackStatus::Paused => (Icon::Pause, Tone::Warning),
        PlaybackStatus::Stopping => (Icon::TimerSand, Tone::Warning),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::ShowModel;
    use sbsp_backend::model::cue::{
        CueChain, CueCursorAdvanceTriggerOverride, CueParam, WaitCueParam,
        group::{GroupCueParamBase, GroupMode},
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
        let (a, g, c, d) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
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
    fn arrow_down_with_nothing_selected_lands_on_the_last_row() {
        let (model, _a, g, _c, _d) = sample();
        let mut ui = UiState::new(true);

        assert_eq!(
            move_cursor(&model, &mut ui, Step::Down, false, false),
            Some(g)
        );
        assert_eq!(ui.selected, Some(g));
    }

    #[test]
    fn arrow_up_with_nothing_selected_lands_on_the_first_row() {
        let (model, a, _g, _c, _d) = sample();
        let mut ui = UiState::new(true);

        assert_eq!(
            move_cursor(&model, &mut ui, Step::Up, false, false),
            Some(a)
        );
    }

    #[test]
    fn moving_steps_over_hidden_children_and_into_expanded_ones() {
        let (model, a, g, c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.set_selected(a, false);

        // Collapsed: a -> g, then the end of the list.
        assert_eq!(
            move_cursor(&model, &mut ui, Step::Down, false, false),
            Some(g)
        );
        assert_eq!(move_cursor(&model, &mut ui, Step::Down, false, false), None);
        assert_eq!(ui.selected, Some(g));

        // Expanded: g -> c.
        ui.toggle_expand(g);
        assert_eq!(
            move_cursor(&model, &mut ui, Step::Down, false, false),
            Some(c)
        );
    }

    #[test]
    fn moving_up_stops_at_the_first_row() {
        let (model, a, _g, _c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.set_selected(a, false);

        assert_eq!(move_cursor(&model, &mut ui, Step::Up, false, false), None);
        assert_eq!(ui.selected, Some(a));
    }

    #[test]
    fn extending_adds_rows_at_the_same_level_only() {
        let (model, a, g, c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        ui.set_selected(a, false);

        assert_eq!(
            move_cursor(&model, &mut ui, Step::Down, true, false),
            Some(g)
        );
        assert_eq!(ui.selected_rows, vec![a, g]);

        // g (level 0) -> c (level 1): refused, selection unchanged.
        assert_eq!(move_cursor(&model, &mut ui, Step::Down, true, false), None);
        assert_eq!(ui.selected_rows, vec![a, g]);
        let _ = c;
    }

    #[test]
    fn select_all_covers_visible_rows_only() {
        let (model, a, g, c, d) = sample();
        let mut ui = UiState::new(true);

        select_all(&model, &mut ui);
        assert_eq!(ui.selected_rows, vec![a, g]);

        ui.toggle_expand(g);
        select_all(&model, &mut ui);
        assert_eq!(ui.selected_rows, vec![a, g, c, d]);
    }

    #[test]
    fn scroll_offset_is_none_while_the_row_is_in_view_or_size_unknown() {
        let (model, a, g, _c, _d) = sample();
        let ui = UiState::new(true);
        let view = Viewport {
            offset_y: 0.0,
            height: 100.0,
        };

        assert_eq!(scroll_offset_for(&model, &ui, view, a), None);
        assert_eq!(scroll_offset_for(&model, &ui, view, g), None);
        assert_eq!(scroll_offset_for(&model, &ui, Viewport::default(), g), None);
    }

    #[test]
    fn scroll_offset_aligns_a_row_below_the_view_to_the_bottom_edge() {
        let (model, _a, g, _c, d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        // Rows: a(0) g(1) c(2) d(3); a viewport two rows tall at the top.
        let view = Viewport {
            offset_y: 0.0,
            height: 2.0 * ROW_HEIGHT,
        };

        assert_eq!(
            scroll_offset_for(&model, &ui, view, d),
            Some(4.0 * ROW_HEIGHT - 2.0 * ROW_HEIGHT)
        );
    }

    #[test]
    fn scroll_offset_aligns_a_row_above_the_view_to_the_top_edge() {
        let (model, a, g, _c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        let view = Viewport {
            offset_y: 2.0 * ROW_HEIGHT,
            height: 2.0 * ROW_HEIGHT,
        };

        assert_eq!(scroll_offset_for(&model, &ui, view, a), Some(0.0));
    }

    #[test]
    fn live_time_shows_elapsed_or_remaining_with_progress() {
        let (label, progress) = live_time(15.0, 60.0, DisplayMode::Elapsed);
        assert_eq!(label, "00:15.00");
        assert_eq!(progress, 0.25);

        let (label, progress) = live_time(15.0, 60.0, DisplayMode::Remain);
        assert_eq!(label, "00:45.00");
        assert_eq!(progress, 0.25);
    }

    #[test]
    fn live_time_progress_stays_within_the_bar() {
        assert_eq!(live_time(90.0, 60.0, DisplayMode::Elapsed).1, 1.0);
        assert_eq!(live_time(-1.0, 60.0, DisplayMode::Elapsed).1, 0.0);
    }

    #[test]
    fn each_distinct_status_has_a_distinct_marker() {
        use std::collections::HashSet;

        let markers: HashSet<(Icon, Tone)> = [
            PlaybackStatus::Loaded,
            PlaybackStatus::Playing,
            PlaybackStatus::Paused,
            PlaybackStatus::Stopping,
        ]
        .into_iter()
        .filter_map(status_icon)
        .collect();

        assert_eq!(markers.len(), 4);
    }

    #[test]
    fn pre_wait_statuses_share_their_playback_markers() {
        assert_eq!(
            status_icon(PlaybackStatus::PreWaiting),
            status_icon(PlaybackStatus::Playing)
        );
        assert_eq!(
            status_icon(PlaybackStatus::PreWaitPaused),
            status_icon(PlaybackStatus::Paused)
        );
    }

    #[test]
    fn no_color_is_fully_transparent_and_others_are_opaque() {
        assert_eq!(cue_color(CueColor::None).a, 0.0);
        assert_eq!(cue_color(CueColor::Red).a, 1.0);
    }
}
