// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The right-hand sidebar: the running cues (title, elapsed/remaining,
//! progress) and the master level meter, on two tabs. Ported from
//! `SideBar.vue`, `ActiveCueItem.vue` and `LevelMeter.vue`.

use std::collections::HashMap;

use iced::mouse;
use iced::widget::canvas::{self, Canvas};
use iced::widget::{Space, button, column, container, row, scrollable, stack, text};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use sbsp_backend::controller::state::{ActiveCue, PlaybackStatus, StateParam};
use sbsp_backend::model::cue::Uuid;
use crate::settings::NameFormatSettings;

use crate::domain::cue::build_cue_name;
use crate::domain::flat_list::flatten;
use crate::domain::time::format_seconds;
use crate::features::header::cue_title;
use crate::fl;
use crate::state::level_meter::{Channel, FLOOR_DB, LevelMeterState};
use crate::state::model::ShowModelState;
use crate::state::playback::PlaybackState;
use crate::state::ui::{SideBarTab, UiState};
use crate::widgets::icon::{Icon, Tone, icon, toned};
use crate::widgets::{divider, progress};

/// The original's `w-65`.
pub const WIDTH: f32 = 260.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    SelectTab(SideBarTab),
}

pub fn view<'a>(
    model: &'a ShowModelState,
    ui: &UiState,
    playback: &'a PlaybackState,
    meter: &'a LevelMeterState,
    name_format: &NameFormatSettings,
) -> Element<'a, Message> {
    let tab_button = |label: String, tab: SideBarTab| {
        button(text(label).center())
            .width(Length::Fill)
            .style(if ui.side_bar_tab == tab {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::SelectTab(tab))
    };

    let tabs = row![
        tab_button(fl!("main-side-bar-active-cues"), SideBarTab::ActiveCues),
        tab_button(fl!("main-side-bar-meter-title"), SideBarTab::Meter),
    ]
    .spacing(1)
    .padding(1);

    let body: Element<'a, Message> = match ui.side_bar_tab {
        SideBarTab::ActiveCues => active_cues(model, ui, playback, name_format),
        SideBarTab::Meter => level_meter(meter),
    };

    column![tabs, divider::horizontal(), body]
        .width(WIDTH)
        .height(Length::Fill)
        .into()
}

/// Running cues in list order. The playback state keeps them in a hash
/// map, whose order changes from one run to the next; sorting by position
/// in the cue list keeps each item where the eye left it.
pub fn ordered_active_cues<'a>(
    model: &ShowModelState,
    ui: &UiState,
    playback: &'a PlaybackState,
) -> Vec<&'a ActiveCue> {
    let order: HashMap<Uuid, usize> = flatten(
        &model.cue_list.root_ids,
        &model.cue_list.cues,
        &ui.expanded_rows,
    )
    .iter()
    .enumerate()
    .map(|(index, entry)| (entry.cue_id, index))
    .collect();

    let mut active: Vec<&ActiveCue> = playback.active_cues.values().collect();
    // Cues missing from the list (removed while running) sort last, by id,
    // so they stay put too.
    active.sort_by_key(|cue| (order.get(&cue.cue_id).copied().unwrap_or(usize::MAX), cue.cue_id));
    active
}

pub fn active_cues<'a>(
    model: &'a ShowModelState,
    ui: &UiState,
    playback: &'a PlaybackState,
    name_format: &NameFormatSettings,
) -> Element<'a, Message> {
    let items: Vec<Element<'a, Message>> = ordered_active_cues(model, ui, playback)
        .into_iter()
        .map(|active| {
            let title = model
                .cue_list
                .cues
                .get(&active.cue_id)
                .map(|cue| {
                    let name = match &cue.name {
                        Some(name) => name.clone(),
                        None => build_cue_name(Some(cue), &model.cue_list.cues, name_format),
                    };
                    cue_title(cue, name)
                })
                .unwrap_or_default();
            active_item(title, active)
        })
        .collect();

    scrollable(column(items).width(Length::Fill))
        .height(Length::Fill)
        .into()
}

/// The two time readouts of a running cue: (elapsed, remaining). While
/// pre-waiting the first counts down the pre-wait ("-00:03.20") and the
/// second reads zero; once playing they are the position and what's left.
/// An unknown duration leaves its readout blank, as in the original.
pub fn readouts(status: PlaybackStatus, position: f64, duration: f64) -> (String, String) {
    let remaining = || format!("-{}", format_seconds(Some(duration - position)));

    match status {
        PlaybackStatus::PreWaiting | PlaybackStatus::PreWaitPaused => (
            if duration > 0.0 { remaining() } else { String::new() },
            format_seconds(Some(0.0)),
        ),
        _ => (
            format_seconds(Some(position)),
            if duration > 0.0 { remaining() } else { String::new() },
        ),
    }
}

fn bar_color(status: PlaybackStatus) -> impl Fn(&Theme) -> Color {
    move |theme: &Theme| match status {
        PlaybackStatus::PreWaiting => Color::from_rgb8(0xEA, 0xB3, 0x08),
        PlaybackStatus::Paused | PlaybackStatus::PreWaitPaused | PlaybackStatus::Stopping => {
            Color::from_rgb8(0xF9, 0x73, 0x16)
        }
        _ => theme.extended_palette().primary.base.color,
    }
}

fn active_item<'a>(title: String, active: &ActiveCue) -> Element<'a, Message> {
    let repeating = matches!(active.params, StateParam::Audio(audio) if audio.repeating);
    let mark: Element<'a, Message> = if active.status == PlaybackStatus::Stopping {
        toned(Icon::TimerSand, 16.0, Tone::Warning).into()
    } else if repeating {
        icon(Icon::Repeat, 16.0).into()
    } else {
        Space::new().width(16).height(16).into()
    };

    let (elapsed, remain) = readouts(active.status, active.position, active.duration);
    let ratio = if active.duration > 0.0 {
        (active.position / active.duration) as f32
    } else {
        0.0
    };

    let track = container(stack![
        progress::bar(ratio, bar_color(active.status)),
    ])
    .width(Length::Fill)
    .height(16)
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            border: iced::Border {
                color: palette.background.strong.color,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    });

    column![
        row![container(text(title)).width(Length::Fill), mark]
            .align_y(Alignment::Center)
            .padding([8, 12]),
        row![
            text(elapsed).font(crate::theme::fonts::MONO),
            Space::new().width(Length::Fill),
            text(remain).font(crate::theme::fonts::MONO),
        ]
        .padding([0, 12]),
        track,
    ]
    .spacing(6)
    .into()
}

fn level_meter<'a>(meter: &'a LevelMeterState) -> Element<'a, Message> {
    column![
        container(text("Master")).center_x(Length::Fill),
        Canvas::new(MeterProgram { meter })
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

/// Where the meter's gridlines/labels sit: dB value and how far down the
/// bar it is (linear in dB over the 60 dB range).
const SCALE: [(&str, f32); 8] = [
    ("0", 0.0),
    ("6", 0.1),
    ("12", 0.2),
    ("18", 0.3),
    ("24", 0.4),
    ("30", 0.5),
    ("42", 0.7),
    ("60", 1.0),
];

struct MeterProgram<'a> {
    meter: &'a LevelMeterState,
}

impl<Message> canvas::Program<Message> for MeterProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        const BAR_W: f32 = 12.0;
        const LABEL_W: f32 = 28.0;
        const GAP: f32 = 4.0;
        const TOP: f32 = 22.0;
        const BOTTOM: f32 = 8.0;

        let palette = theme.extended_palette();
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let total_w = BAR_W * 2.0 + LABEL_W + GAP * 2.0;
        let x0 = ((bounds.width - total_w) / 2.0).max(0.0);
        let bar_h = (bounds.height - TOP - BOTTOM).max(1.0);
        let left_x = x0;
        let label_x = x0 + BAR_W + GAP;
        let right_x = label_x + LABEL_W + GAP;

        let mut draw_bar = |x: f32, channel: &Channel| {
            // The three zones of the original's gradient, bottom up: green
            // to 70%, amber to 95%, red above.
            for (from, to, color) in [
                (0.0, 0.70, Color::from_rgb8(52, 211, 103)),
                (0.70, 0.95, Color::from_rgb8(251, 191, 36)),
                (0.95, 1.0, Color::from_rgb8(255, 0, 0)),
            ] {
                frame.fill_rectangle(
                    Point::new(x, TOP + bar_h * (1.0 - to)),
                    Size::new(BAR_W, bar_h * (to - from)),
                    color,
                );
            }

            // Cover the part above the current level.
            frame.fill_rectangle(
                Point::new(x, TOP),
                Size::new(BAR_W, bar_h * channel.empty_fraction()),
                palette.background.base.color,
            );

            frame.stroke_rectangle(
                Point::new(x, TOP),
                Size::new(BAR_W, bar_h),
                canvas::Stroke::default()
                    .with_color(palette.background.strong.color)
                    .with_width(1.0),
            );

            // The clip lamp above the bar.
            let lamp_origin = Point::new(x, TOP - BAR_W - 4.0);
            if channel.is_clipping() {
                frame.fill_rectangle(lamp_origin, Size::new(BAR_W, BAR_W), Color::from_rgb8(239, 68, 68));
            }
            frame.stroke_rectangle(
                lamp_origin,
                Size::new(BAR_W, BAR_W),
                canvas::Stroke::default()
                    .with_color(palette.background.strong.color)
                    .with_width(1.0),
            );
        };
        draw_bar(left_x, &self.meter.left);
        draw_bar(right_x, &self.meter.right);

        for (label, fraction) in SCALE {
            frame.fill_text(canvas::Text {
                content: label.to_string(),
                // Centered in the label column between the two bars.
                position: Point::new(label_x + LABEL_W / 2.0, TOP + bar_h * fraction),
                color: palette.background.base.text,
                size: 10.0.into(),
                // `.into()` so this compiles whether the field is typed as
                // `Horizontal` or as `text::Alignment`.
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                font: crate::theme::fonts::UI,
                ..canvas::Text::default()
            });
        }

        // The scale is relative to FLOOR_DB, kept in one place.
        debug_assert_eq!(FLOOR_DB, -60.0);

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::{
        Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, WaitCueParam,
    };

    fn cue(id: Uuid) -> Cue {
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

    fn active(id: Uuid) -> ActiveCue {
        ActiveCue {
            cue_id: id,
            position: 0.0,
            duration: 10.0,
            status: PlaybackStatus::Playing,
            params: StateParam::None,
        }
    }

    #[test]
    fn playing_readouts_are_position_and_remaining() {
        assert_eq!(
            readouts(PlaybackStatus::Playing, 3.5, 10.0),
            ("00:03.50".to_string(), "-00:06.50".to_string())
        );
    }

    #[test]
    fn pre_wait_readouts_count_down_and_the_second_reads_zero() {
        assert_eq!(
            readouts(PlaybackStatus::PreWaiting, 1.0, 4.0),
            ("-00:03.00".to_string(), "00:00.00".to_string())
        );
    }

    #[test]
    fn an_unknown_duration_leaves_the_remaining_readout_blank() {
        assert_eq!(
            readouts(PlaybackStatus::Playing, 2.0, 0.0),
            ("00:02.00".to_string(), String::new())
        );
    }

    #[test]
    fn active_cues_are_listed_in_cue_list_order() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let mut model = ShowModelState::default();
        for id in [a, b, c] {
            model.cue_list.cues.insert(id, cue(id));
        }
        model.cue_list.root_ids = vec![a, b, c];

        let mut playback = PlaybackState::new();
        // Inserted out of list order on purpose.
        for id in [c, a, b] {
            playback.active_cues.insert(id, active(id));
        }

        let ui = UiState::new(true);
        let order: Vec<Uuid> = ordered_active_cues(&model, &ui, &playback)
            .iter()
            .map(|cue| cue.cue_id)
            .collect();

        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn a_running_cue_missing_from_the_list_sorts_last() {
        let (a, ghost) = (Uuid::new_v4(), Uuid::new_v4());
        let mut model = ShowModelState::default();
        model.cue_list.cues.insert(a, cue(a));
        model.cue_list.root_ids = vec![a];

        let mut playback = PlaybackState::new();
        playback.active_cues.insert(ghost, active(ghost));
        playback.active_cues.insert(a, active(a));

        let ui = UiState::new(true);
        let order: Vec<Uuid> = ordered_active_cues(&model, &ui, &playback)
            .iter()
            .map(|cue| cue.cue_id)
            .collect();

        assert_eq!(order, vec![a, ghost]);
    }
}
