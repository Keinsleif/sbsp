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
use iced::{Alignment, Border, Color, Element, Length, Padding, Point, Theme, mouse};
use iced_aw::ContextMenu as AwContextMenu;
use std::collections::HashMap;

use sbsp_backend::controller::state::PlaybackStatus;
use sbsp_backend::manager::InsertPosition;
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
use crate::theme::fonts;
use crate::widgets::divider;
use crate::widgets::icon::{Icon, Tone, icon, toned};
use crate::widgets::progress;
use crate::widgets::tree_guide::{self, Kind as GuideKind};

/// Rows are a fixed height so the scroll position of any row is just
/// `index * ROW_HEIGHT`: keeping a row in view then needs no layout
/// feedback from the widget tree, only the scrollable's own offset and
/// viewport height (see [`Viewport`]).
const ROW_HEIGHT: f32 = 30.0;
const HEADER_HEIGHT: f32 = 32.0;

/// A drag scrolls the list when the pointer is this close to its top or
/// bottom edge, by up to `AUTO_SCROLL_MAX` pixels per tick.
const AUTO_SCROLL_EDGE: f32 = 48.0;
const AUTO_SCROLL_MAX: f32 = 16.0;

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
    /// The pointer entered / left a row (edit mode): the context menu
    /// acts on the row under it, and the app cannot see the pointer.
    Hover(Uuid),
    HoverEnd(Uuid),
    /// A right click, whatever it hit: the context menu opens by itself,
    /// the app only selects the row under the pointer.
    RightClicked,
    Context(ContextAction),
    /// Pressed on a row's handle (edit mode): the selected cues, this one
    /// too, are being dragged.
    DragStart(Uuid),
    /// The pointer, within the list, while dragging.
    DragMoved(Point),
    /// The pointer left the list: dropping now drops nowhere.
    DragLeft,
    /// The button was released: drop where the indicator is.
    DragDropped,
    /// While dragging: time to scroll if the pointer is at an edge.
    DragTick,
}

/// What the context menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAction {
    Copy,
    Cut,
    Paste,
    Duplicate,
    Delete,
}

/// What the list adds in edit mode: the drag handles, the context menu and
/// the drop indicator (the other modes have none of them).
#[derive(Debug, Clone, Copy)]
pub struct EditMode<'a> {
    pub can_paste: bool,
    pub drag: Option<&'a Drag>,
}

/// A drag of cues in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct Drag {
    /// What is being moved.
    pub ids: Vec<Uuid>,
    /// The pointer within the list, if it is over it.
    pub pointer: Option<Point>,
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

    let flat = flatten(
        &model.cue_list.root_ids,
        &model.cue_list.cues,
        &ui.expanded_rows,
    );
    let index = render_rows(&flat)
        .iter()
        .position(|row| matches!(row, RenderRow::Entry(i) if flat[*i].cue_id == id))?;
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

/// A row as drawn: a cue, or the "end" row closing an open group (and the
/// list), which is where a drag can drop at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderRow {
    /// An index into the flat list.
    Entry(usize),
    End { parent: Option<Uuid>, level: usize },
}

/// The rows to draw, top to bottom (`CueList.vue`'s `buildRenderRows`).
pub fn render_rows(flat: &[FlatCueEntry]) -> Vec<RenderRow> {
    fn close_groups(rows: &mut Vec<RenderRow>, open: &mut Vec<(Uuid, usize)>, level: usize) {
        while let Some(&(id, group_level)) = open.last() {
            if group_level < level {
                break;
            }
            open.pop();
            rows.push(RenderRow::End {
                parent: Some(id),
                level: group_level + 1,
            });
        }
    }

    let mut rows = Vec::new();
    // The expanded groups the rows so far are inside, with their levels.
    let mut open: Vec<(Uuid, usize)> = Vec::new();

    for (index, entry) in flat.iter().enumerate() {
        if entry.is_hidden {
            continue;
        }
        close_groups(&mut rows, &mut open, entry.level);
        rows.push(RenderRow::Entry(index));
        if entry.group.is_some_and(|group| group.is_expanded) {
            open.push((entry.cue_id, entry.level));
        }
    }
    close_groups(&mut rows, &mut open, 0);
    rows.push(RenderRow::End {
        parent: None,
        level: 0,
    });
    rows
}

/// Whether `id` is one of `ids` or lies inside one of them (a group cannot
/// be dropped into itself).
fn is_within(cues: &HashMap<Uuid, Cue>, id: Uuid, ids: &[Uuid]) -> bool {
    let mut current = Some(id);
    // The depth bound only guards against a broken parent chain.
    for _ in 0..=cues.len() {
        let Some(id) = current else {
            return false;
        };
        if ids.contains(&id) {
            return true;
        }
        current = cues.get(&id).and_then(|cue| cue.parent_id);
    }
    false
}

/// The row a drop would land before (or inside, for an end row) and the
/// position to send. `None` over nothing, or over the dragged cues.
fn drop_target(
    flat: &[FlatCueEntry],
    rows: &[RenderRow],
    cues: &HashMap<Uuid, Cue>,
    viewport: Viewport,
    drag: &Drag,
) -> Option<(usize, InsertPosition)> {
    let pointer = drag.pointer?;
    let y = pointer.y + viewport.offset_y;
    if y < 0.0 {
        return None;
    }
    let index = (y / ROW_HEIGHT) as usize;

    let (anchor, position) = match *rows.get(index)? {
        RenderRow::Entry(i) => {
            let id = flat.get(i)?.cue_id;
            (Some(id), InsertPosition::Before { target: id })
        }
        RenderRow::End { parent, .. } => (
            parent,
            InsertPosition::Inside {
                target: parent,
                index: None,
            },
        ),
    };
    if anchor.is_some_and(|id| is_within(cues, id, &drag.ids)) {
        return None;
    }
    Some((index, position))
}

/// Where dropping `drag` now would put the cues.
pub fn drop_position(
    model: &ShowModelState,
    ui: &UiState,
    viewport: Viewport,
    drag: &Drag,
) -> Option<InsertPosition> {
    let flat = flatten(&model.cue_list.root_ids, &model.cue_list.cues, &ui.expanded_rows);
    let rows = render_rows(&flat);
    drop_target(&flat, &rows, &model.cue_list.cues, viewport, drag).map(|(_, position)| position)
}

/// How far to scroll per tick for a pointer at `y` in a list `height`
/// tall: nothing in the middle, faster the closer to an edge.
fn auto_scroll_delta(y: f32, height: f32) -> f32 {
    if height <= 2.0 * AUTO_SCROLL_EDGE {
        return 0.0;
    }
    if y < AUTO_SCROLL_EDGE {
        -AUTO_SCROLL_MAX * ((AUTO_SCROLL_EDGE - y) / AUTO_SCROLL_EDGE).min(1.0)
    } else if y > height - AUTO_SCROLL_EDGE {
        AUTO_SCROLL_MAX * ((y - (height - AUTO_SCROLL_EDGE)) / AUTO_SCROLL_EDGE).min(1.0)
    } else {
        0.0
    }
}

/// The offset to scroll to while dragging with the pointer near an edge,
/// or `None` if it should stay where it is.
pub fn drag_scroll_offset(
    model: &ShowModelState,
    ui: &UiState,
    viewport: Viewport,
    drag: &Drag,
) -> Option<f32> {
    let pointer = drag.pointer?;
    let delta = auto_scroll_delta(pointer.y, viewport.height);
    if delta == 0.0 {
        return None;
    }
    let flat = flatten(&model.cue_list.root_ids, &model.cue_list.cues, &ui.expanded_rows);
    let content = render_rows(&flat).len() as f32 * ROW_HEIGHT;
    let max = (content - viewport.height).max(0.0);
    let offset = (viewport.offset_y + delta).clamp(0.0, max);
    ((offset - viewport.offset_y).abs() > f32::EPSILON).then_some(offset)
}

const W_HANDLE: f32 = 20.0;
const W_CURSOR: f32 = 28.0;
const W_STATUS: f32 = 28.0;
const W_TYPE: f32 = 28.0;
const W_NUMBER: f32 = 54.0;
const W_TIME: f32 = 112.0;
const W_REPEAT: f32 = 28.0;
const W_CHAIN: f32 = 36.0;

/// What every row is drawn from.
#[derive(Clone, Copy)]
struct Env<'a, 'b> {
    model: &'a ShowModelState,
    ui: &'b UiState,
    playback: &'b PlaybackState,
    assets: &'b AssetResults,
    name_format: &'b NameFormatSettings,
    edit: Option<EditMode<'b>>,
}

/// The column header plus the rows. Column widths are shared constants
/// (cells below, header cells here), so the two line up without a table
/// widget. `edit` is `Some` in edit mode.
pub fn view<'a>(
    model: &'a ShowModelState,
    ui: &'a UiState,
    playback: &PlaybackState,
    assets: &AssetResults,
    name_format: &NameFormatSettings,
    viewport: Viewport,
    edit: Option<EditMode<'_>>,
) -> Element<'a, Message> {
    let flat = flatten(
        &model.cue_list.root_ids,
        &model.cue_list.cues,
        &ui.expanded_rows,
    );
    let layout = render_rows(&flat);

    let drag = edit.and_then(|edit| edit.drag);
    let target = drag
        .and_then(|drag| drop_target(&flat, &layout, &model.cue_list.cues, viewport, drag))
        .map(|(index, _)| index);

    let env = Env {
        model,
        ui,
        playback,
        assets,
        name_format,
        edit,
    };
    let rows: Vec<Element<'a, Message>> = layout
        .iter()
        .enumerate()
        .filter_map(|(index, row)| match *row {
            RenderRow::Entry(i) => {
                let entry = &flat[i];
                let cue = model.cue_list.cues.get(&entry.cue_id)?;
                Some(row_view(entry, cue, &env, target == Some(index)))
            }
            RenderRow::End { level, .. } => Some(end_row_view(level, target == Some(index))),
        })
        .collect();

    // Only the closing row of the root: no cues.
    let body: Element<'a, Message> = if layout.len() <= 1 {
        container(text(fl!("cue-list-empty")))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else {
        let list = scrollable(column(rows).width(Length::Fill))
            .id(LIST_ID)
            .on_scroll(|viewport| {
                Message::Scrolled(Viewport {
                    offset_y: viewport.absolute_offset().y,
                    height: viewport.bounds().height,
                })
            })
            .height(Length::Fill);

        // The pointer is only followed while dragging: a message per mouse
        // move would rebuild the whole list for nothing.
        let mut area = mouse_area(list);
        if drag.is_some() {
            area = area
                .on_move(Message::DragMoved)
                .on_exit(Message::DragLeft)
                .interaction(mouse::Interaction::Grabbing);
        }

        let ghost: Element<'a, Message> = match drag.and_then(|drag| Some((drag.ids.len(), drag.pointer?))) {
            Some((count, pointer)) => container(
                container(text(fl!("main-cue-list-moving-cue", n = (count as i64))).size(12))
                    .padding([2, 10])
                    .style(|theme: &Theme| {
                        let palette = theme.extended_palette();
                        container::Style {
                            background: Some(palette.primary.base.color.into()),
                            text_color: Some(palette.primary.base.text),
                            border: Border {
                                radius: 4.0.into(),
                                ..Border::default()
                            },
                            ..Default::default()
                        }
                    }),
            )
            .padding(Padding {
                top: pointer.y + 8.0,
                left: pointer.x + 16.0,
                right: 0.0,
                bottom: 0.0,
            })
            .into(),
            None => Space::new().into(),
        };
        stack![area, ghost].into()
    };

    column![header(ui), divider::horizontal(), body].into()
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
            W_TIME - 1.0,
        )
    };

    // Each line takes its pixel from the column before it, so the header's
    // columns stay over the rows' (the name column fills the rest).
    row![
        Space::new().width(W_HANDLE + W_CURSOR + W_STATUS + W_TYPE - 1.0),
        divider::vertical(),
        cell(text("#"), W_NUMBER - 1.0),
        divider::vertical(),
        container(text(fl!("main-name"))).width(Length::Fill).padding([0, 8]),
        divider::vertical(),
        time_header(
            fl!("main-pre-wait"),
            ui.pre_wait_display_mode,
            Message::TogglePreWaitDisplay,
        ),
        divider::vertical(),
        time_header(
            fl!("main-duration"),
            ui.duration_display_mode,
            Message::ToggleDurationDisplay,
        ),
        divider::vertical(),
        cell(icon(Icon::Repeat, 18.0), W_REPEAT - 1.0),
        divider::vertical(),
        cell(icon(Icon::ChevronDoubleDown, 18.0), W_CHAIN),
    ]
    .align_y(Alignment::Center)
    .height(HEADER_HEIGHT)
    .into()
}

fn row_view<'a>(
    entry: &FlatCueEntry,
    cue: &'a Cue,
    env: &Env<'a, '_>,
    is_drop: bool,
) -> Element<'a, Message> {
    let Env {
        model,
        ui,
        playback,
        assets,
        name_format,
        edit,
    } = *env;
    let id = entry.cue_id;
    let is_selected = ui.selected_rows.contains(&id);
    let is_cursor = ui.playback_cursor == Some(id);
    let active = playback.active_cues.get(&id);

    let cursor_mark: Element<'a, Message> = if is_cursor {
        icon(Icon::ArrowRightBold, 18.0).into()
    } else {
        Space::new().into()
    };
    // The whole cell is the button, so a row without the cursor can be
    // clicked too: an empty mark has no height of its own, and a button
    // that fits it would have none either.
    let cursor = button(
        container(cursor_mark)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
        .width(W_CURSOR)
        .height(ROW_HEIGHT - 1.0)
        .padding(0)
        .style(button::text)
        .on_press(Message::SetCursor(id));

    let status: Element<'a, Message> = match active.and_then(|a| status_icon(a.status)) {
        Some((glyph, tone)) => toned(glyph, 18.0, tone).into(),
        None => Space::new().into(),
    };

    let indent = tree_guide::view(entry.level, GuideKind::Item, ROW_HEIGHT - 1.0);
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

    // Edit mode: a handle to drag the row by.
    let handle: Element<'a, Message> = if edit.is_some() {
        mouse_area(container(icon(Icon::DragVertical, 18.0)).center_x(W_HANDLE))
            .on_press(Message::DragStart(id))
            .interaction(mouse::Interaction::Grab)
            .into()
    } else {
        Space::new().width(W_HANDLE).into()
    };

    let content = row![
        handle,
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
    // The line under the row is part of its height, so a row stays
    // `ROW_HEIGHT` tall (the scroll position is computed from that).
    let area = mouse_area(column![
        container(content)
            .padding([0, 8])
            .center_y(ROW_HEIGHT - 1.0)
            .width(Length::Fill)
            .style(move |theme: &Theme| row_style(theme, is_selected, is_cursor, color)),
        divider::horizontal(),
    ])
    .on_press(Message::Select(id));

    let row: Element<'a, Message> = match edit {
        Some(edit) => {
            let can_paste = edit.can_paste;
            AwContextMenu::new(
                area.on_enter(Message::Hover(id))
                    .on_exit(Message::HoverEnd(id)),
                move || context_menu_view(can_paste),
            )
            .into()
        }
        None => area.into(),
    };
    drop_marker(row, is_drop)
}

/// The closing row of an open group or of the list: where a drop lands
/// after the last row.
fn end_row_view<'a>(level: usize, is_drop: bool) -> Element<'a, Message> {
    let label = if level > 0 {
        fl!("cue-list-end")
    } else {
        fl!("cue-list-end-of-list")
    };
    // Lined up with the name column of a row at the same level: after its
    // padding and the columns before it come the guide, then the space of
    // the disclosure triangle (and the gaps around it).
    let lead = 8.0 + W_HANDLE + W_CURSOR + W_STATUS + W_TYPE + W_NUMBER;
    let triangle = 20.0 + 8.0;

    let content = row![
        Space::new().width(lead),
        tree_guide::view(level, GuideKind::End, ROW_HEIGHT - 1.0),
        Space::new().width(triangle),
        text(label).style(text::secondary),
    ]
    .align_y(Alignment::Center);
    let row = column![
        container(content)
            .center_y(ROW_HEIGHT - 1.0)
            .width(Length::Fill),
        divider::horizontal(),
    ];
    drop_marker(row.into(), is_drop)
}

/// The line over a row's top edge where a drop would land. Always a
/// layer, so a row's widgets keep their state when the line comes and goes.
fn drop_marker<'a>(row: Element<'a, Message>, is_drop: bool) -> Element<'a, Message> {
    let line: Element<'a, Message> = if is_drop {
        container(Space::new())
            .width(Length::Fill)
            .height(2)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().primary.base.color.into()),
                ..Default::default()
            })
            .into()
    } else {
        Space::new().into()
    };
    stack![row, line].into()
}

/// The right-click menu. Items act on the selection, which the app has
/// already moved to the clicked row if that was not part of it.
fn context_menu_view<'a>(can_paste: bool) -> Element<'a, Message> {
    let entry = |glyph: Icon, label: String, action: ContextAction, enabled: bool| {
        button(
            row![icon(glyph, 18.0), text(label)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([4, 10])
        .style(button::text)
        .on_press_maybe(enabled.then_some(Message::Context(action)))
    };

    container(
        column![
            entry(
                Icon::ContentCopy,
                fl!("main-cue-list-context-menu-copy"),
                ContextAction::Copy,
                true,
            ),
            entry(
                Icon::ContentCut,
                fl!("main-cue-list-context-menu-cut"),
                ContextAction::Cut,
                true,
            ),
            entry(
                Icon::ContentPaste,
                fl!("main-cue-list-context-menu-paste"),
                ContextAction::Paste,
                can_paste,
            ),
            entry(
                Icon::ContentDuplicate,
                fl!("main-cue-list-context-menu-duplicate"),
                ContextAction::Duplicate,
                true,
            ),
            divider::horizontal(),
            entry(
                Icon::TrashCan,
                fl!("main-cue-list-context-menu-delete"),
                ContextAction::Delete,
                true,
            ),
        ]
        .spacing(2),
    )
    .width(Length::Fixed(180.0))
    .padding(4)
    .style(container::rounded_box)
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

    // The stack is sized explicitly: an empty bar (no progress) is
    // zero-sized, and a stack takes its size from its first layer, which
    // would leave the label no room.
    let field = container(
        stack![bar, center(text(label).font(fonts::MONO))]
            .width(Length::Fill)
            .height(Length::Fill),
    )
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
        .center_y(ROW_HEIGHT - 1.0)
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

pub(crate) fn type_icon(param: &CueParam) -> Icon {
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

pub(crate) fn chain_icon(chain: &CueChain) -> Option<Icon> {
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

    fn flat_of(model: &ShowModel, ui: &UiState) -> Vec<FlatCueEntry> {
        flatten(&model.cue_list.root_ids, &model.cue_list.cues, &ui.expanded_rows)
    }

    #[test]
    fn a_collapsed_group_has_no_end_row_but_the_list_does() {
        let (model, _a, _g, _c, _d) = sample();
        let ui = UiState::new(true);
        let flat = flat_of(&model, &ui);

        assert_eq!(
            render_rows(&flat),
            vec![
                RenderRow::Entry(0),
                RenderRow::Entry(1),
                RenderRow::End { parent: None, level: 0 },
            ]
        );
    }

    #[test]
    fn an_open_group_ends_with_its_own_end_row() {
        let (model, _a, g, _c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        let flat = flat_of(&model, &ui);

        assert_eq!(
            render_rows(&flat),
            vec![
                RenderRow::Entry(0),
                RenderRow::Entry(1),
                RenderRow::Entry(2),
                RenderRow::Entry(3),
                RenderRow::End { parent: Some(g), level: 1 },
                RenderRow::End { parent: None, level: 0 },
            ]
        );
    }

    #[test]
    fn an_end_row_closes_a_group_before_the_next_row_at_its_level() {
        let (mut model, a, g, _c, _d) = sample();
        let b = Uuid::new_v4();
        model.cue_list.cues.insert(b, leaf(b, None));
        model.cue_list.root_ids = vec![g, b, a];
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        let flat = flat_of(&model, &ui);

        // g, c, d, end of g, b, a, end of list
        let rows = render_rows(&flat);
        assert_eq!(rows.len(), 7);
        assert_eq!(rows[3], RenderRow::End { parent: Some(g), level: 1 });
        assert_eq!(rows[4], RenderRow::Entry(3));
    }

    #[test]
    fn an_open_empty_group_still_has_an_end_row_to_drop_into() {
        let (mut model, a, g, c, d) = sample();
        model.cue_list.cues.insert(g, group(g, vec![]));
        model.cue_list.cues.remove(&c);
        model.cue_list.cues.remove(&d);
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        let flat = flat_of(&model, &ui);

        assert_eq!(
            render_rows(&flat),
            vec![
                RenderRow::Entry(0),
                RenderRow::Entry(1),
                RenderRow::End { parent: Some(g), level: 1 },
                RenderRow::End { parent: None, level: 0 },
            ]
        );
        let _ = a;
    }

    fn pointer_at(row: f32, offset: f32) -> Drag {
        Drag {
            ids: Vec::new(),
            pointer: Some(Point::new(50.0, row * ROW_HEIGHT + ROW_HEIGHT / 2.0 - offset)),
        }
    }

    fn target(
        model: &ShowModel,
        ui: &UiState,
        viewport: Viewport,
        drag: &Drag,
    ) -> Option<InsertPosition> {
        drop_position(model, ui, viewport, drag)
    }

    #[test]
    fn a_drop_lands_before_the_row_or_inside_at_an_end_row() {
        let (model, a, g, c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        let view = Viewport { offset_y: 0.0, height: 300.0 };
        // Rows: a, g, c, d, end of g, end of list.

        assert!(matches!(
            target(&model, &ui, view, &pointer_at(0.0, 0.0)),
            Some(InsertPosition::Before { target }) if target == a
        ));
        assert!(matches!(
            target(&model, &ui, view, &pointer_at(2.0, 0.0)),
            Some(InsertPosition::Before { target }) if target == c
        ));
        assert!(matches!(
            target(&model, &ui, view, &pointer_at(4.0, 0.0)),
            Some(InsertPosition::Inside { target: Some(t), index: None }) if t == g
        ));
        assert!(matches!(
            target(&model, &ui, view, &pointer_at(5.0, 0.0)),
            Some(InsertPosition::Inside { target: None, index: None })
        ));
        // Below the last row, or no pointer: nowhere.
        assert!(target(&model, &ui, view, &pointer_at(6.0, 0.0)).is_none());
        assert!(target(&model, &ui, view, &Drag { ids: vec![], pointer: None }).is_none());
    }

    #[test]
    fn the_scroll_offset_moves_the_rows_under_the_pointer() {
        let (model, _a, g, c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        let view = Viewport { offset_y: 60.0, height: 60.0 };

        // Two rows scrolled away: the pointer's first row is row 2.
        assert!(matches!(
            target(&model, &ui, view, &pointer_at(2.0, 60.0)),
            Some(InsertPosition::Before { target }) if target == c
        ));
    }

    #[test]
    fn cues_cannot_be_dropped_onto_themselves_or_into_themselves() {
        let (model, a, g, c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        let view = Viewport { offset_y: 0.0, height: 300.0 };
        let dragging = |ids: Vec<Uuid>, row: f32| Drag {
            ids,
            pointer: pointer_at(row, 0.0).pointer,
        };

        // On the dragged cue itself.
        assert!(target(&model, &ui, view, &dragging(vec![a], 0.0)).is_none());
        // A group over its own child, and over its own end row.
        assert!(target(&model, &ui, view, &dragging(vec![g], 2.0)).is_none());
        assert!(target(&model, &ui, view, &dragging(vec![g], 4.0)).is_none());
        // But over something else it can land, and the end of the list too.
        assert!(target(&model, &ui, view, &dragging(vec![g], 0.0)).is_some());
        assert!(target(&model, &ui, view, &dragging(vec![g], 5.0)).is_some());
        // A child can leave its group for the end of the list.
        assert!(target(&model, &ui, view, &dragging(vec![c], 5.0)).is_some());
    }

    #[test]
    fn dragging_scrolls_faster_the_closer_to_the_edge() {
        let height = 300.0;
        assert_eq!(auto_scroll_delta(150.0, height), 0.0);
        assert_eq!(auto_scroll_delta(AUTO_SCROLL_EDGE, height), 0.0);
        assert_eq!(auto_scroll_delta(0.0, height), -AUTO_SCROLL_MAX);
        assert_eq!(auto_scroll_delta(height, height), AUTO_SCROLL_MAX);
        let slow = auto_scroll_delta(AUTO_SCROLL_EDGE - 12.0, height);
        let fast = auto_scroll_delta(AUTO_SCROLL_EDGE - 36.0, height);
        assert!(slow < 0.0 && fast < slow);
        // A list too short to have a middle does not scroll by itself.
        assert_eq!(auto_scroll_delta(10.0, 80.0), 0.0);
    }

    #[test]
    fn drag_scrolling_stays_within_the_list() {
        let (model, _a, g, _c, _d) = sample();
        let mut ui = UiState::new(true);
        ui.toggle_expand(g);
        // 6 rows: 180px of content in a 100px viewport: at most 80 to scroll.
        let at = |offset_y: f32, y: f32| {
            drag_scroll_offset(
                &model,
                &ui,
                Viewport { offset_y, height: 100.0 },
                &Drag { ids: vec![], pointer: Some(Point::new(10.0, y)) },
            )
        };

        assert_eq!(at(0.0, 50.0), None, "middle of the list");
        assert_eq!(at(0.0, 0.0), None, "already at the top");
        assert_eq!(at(10.0, 0.0), Some(0.0), "stops at the top");
        assert_eq!(at(0.0, 100.0), Some(AUTO_SCROLL_MAX));
        assert_eq!(at(80.0, 100.0), None, "already at the bottom");
        assert_eq!(at(75.0, 100.0), Some(80.0), "stops at the bottom");
    }
}
