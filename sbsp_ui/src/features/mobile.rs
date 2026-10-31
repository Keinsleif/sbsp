// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The phone layout of the web build: a big clock, one of three screens
//! (the cue list, the controls for the cue under the playback cursor, the
//! running cues) and a bar to choose between them. Ported from
//! `MainViewMobile.vue`, `ControlsPanel.vue` and `SeekBar.vue`.
//!
//! The list and the running cues are the desktop's own views; this module
//! has the frame around them, the controls screen, and the cursor moves of
//! its skip buttons.

use std::collections::{HashMap, HashSet};

use iced::widget::{Space, button, column, container, row, slider, text};
use iced::{Alignment, Element, Length, Theme};

use sbsp_backend::controller::state::PlaybackStatus;
use sbsp_backend::model::cue::{Cue, CueParam, Uuid};

use crate::domain::flat_list::flatten;
use crate::domain::time::format_seconds;
use crate::features::header::{self, ClockTime};
use crate::fl;
use crate::widgets::blink::Strength;
use crate::widgets::divider;
use crate::widgets::icon::{Icon, colored};

/// The layout is used below this width (Tailwind's `sm`, which the original
/// switched on too).
pub const MAX_WIDTH: f32 = 640.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    List,
    Controls,
    Active,
}

/// The controls screen is only for those who may control; for anyone else
/// a remembered choice of it falls back to the list.
pub fn effective_tab(tab: Tab, can_control: bool) -> Tab {
    if tab == Tab::Controls && !can_control {
        Tab::List
    } else {
        tab
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    Previous,
    Next,
    Parent,
    Child,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Tab(Tab),
    /// Stop, play and ready/pause: the same as the desktop's header.
    Transport(header::Message),
    Skip(Skip),
    Rewind,
    FastForward,
    ToggleRepeat,
    /// The seek bar is being dragged to this position.
    Seek(f64),
    SeekReleased,
}

/// Where the playback cursor goes for a skip button. `Previous` and `Next`
/// move through the list (children included, collapsed or not) to the
/// nearest cue on the same level, as the original does; with no cursor
/// they go to the first and the last cue.
pub fn skip(
    direction: Skip,
    cursor: Option<Uuid>,
    root_ids: &[Uuid],
    cues: &HashMap<Uuid, Cue>,
) -> Option<Uuid> {
    let everything: HashSet<Uuid> = cues.keys().copied().collect();
    let flat = flatten(root_ids, cues, &everything);

    let Some(cursor) = cursor else {
        return match direction {
            Skip::Previous => flat.first(),
            Skip::Next => flat.last(),
            Skip::Parent | Skip::Child => None,
        }
        .map(|entry| entry.cue_id);
    };
    let index = flat.iter().position(|entry| entry.cue_id == cursor)?;
    let level = flat[index].level;

    match direction {
        Skip::Previous => flat[..index]
            .iter()
            .rev()
            .find(|entry| entry.level == level)
            .map(|entry| entry.cue_id),
        Skip::Next => flat[index + 1..]
            .iter()
            .find(|entry| entry.level == level)
            .map(|entry| entry.cue_id),
        Skip::Parent => flat[index].parent,
        Skip::Child => match &cues.get(&cursor)?.params {
            CueParam::Group { children, .. } => children.first().copied(),
            _ => None,
        },
    }
}

/// The clock, the screen, and the bar of screens. `body` is the chosen
/// screen, already made by the caller (the list and the running cues are
/// the desktop's views, with their own messages).
pub fn frame<'a, M: Clone + 'a>(
    clock: ClockTime,
    tab: Tab,
    can_control: bool,
    body: Element<'a, M>,
    on_tab: impl Fn(Tab) -> M + 'a,
) -> Element<'a, M> {
    let clock = container(
        row![
            text(format!("{:02}:{:02}.", clock.hour, clock.minute))
                .size(56)
                .line_height(1.0)
                .font(crate::theme::fonts::MONO),
            text(format!("{:02}", clock.second))
                .size(36)
                .line_height(1.0)
                .font(crate::theme::fonts::MONO),
        ]
        .align_y(Alignment::End),
    )
    .center_x(Length::Fill)
    .padding([16, 12]);

    let tab_button = |glyph: Icon, label: String, target: Tab| {
        let selected = tab == target;
        button(
            row![
                colored(glyph, 18.0, move |theme: &Theme| {
                    let palette = theme.extended_palette();
                    if selected {
                        palette.primary.base.text
                    } else {
                        palette.secondary.base.text
                    }
                }),
                text(label),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([12, 0])
        .style(if selected { button::primary } else { button::secondary })
        .on_press(on_tab(target))
    };

    let mut tabs = row![tab_button(Icon::FormatListBulleted, fl!("main-mobile-list"), Tab::List)];
    if can_control {
        tabs = tabs.push(tab_button(Icon::Remote, fl!("main-mobile-controls"), Tab::Controls));
    }
    tabs = tabs.push(tab_button(Icon::Monitor, fl!("main-mobile-active"), Tab::Active));

    column![
        clock,
        divider::horizontal(),
        container(body).width(Length::Fill).height(Length::Fill),
        tabs.spacing(1),
    ]
    .into()
}

/// What the controls screen shows.
pub struct Controls {
    /// "number・name" of the cue under the cursor (empty without one).
    pub title: String,
    pub duration: Option<f64>,
    pub status: Option<PlaybackStatus>,
    /// The cue under the cursor, while it is running.
    pub playing: Option<Playing>,
    /// The seek bar's value while it is being dragged.
    pub dragging: Option<f64>,
    /// The flashing level of the transport buttons (see `widgets::blink`).
    pub blink: f32,
}

pub struct Playing {
    pub position: f64,
    pub duration: f64,
    pub repeating: bool,
}

pub fn controls<'a>(c: Controls) -> Element<'a, Message> {
    let status = c.status;

    let skips = row![
        key(Icon::ArrowUpLeft, false, Tone::Plain, 2, Some(Message::Skip(Skip::Parent))),
        key(Icon::SkipPrevious, false, Tone::Plain, 1, Some(Message::Skip(Skip::Previous))),
        key(Icon::SkipNext, false, Tone::Plain, 1, Some(Message::Skip(Skip::Next))),
        key(Icon::ArrowDownRight, false, Tone::Plain, 2, Some(Message::Skip(Skip::Child))),
    ]
    .spacing(2);

    let running = c.playing.is_some();
    let (position, duration, repeating) = c
        .playing
        .as_ref()
        .map_or((0.0, 1.0, false), |p| (p.position, p.duration.max(0.001), p.repeating));
    let shown = c.dragging.unwrap_or(position).clamp(0.0, duration);

    let bar = if running {
        slider(0.0..=duration as f32, shown as f32, |value| Message::Seek(f64::from(value)))
            .step(0.01_f32)
            .on_release(Message::SeekReleased)
    } else {
        slider(0.0..=1.0_f32, 0.0, |_| Message::Seek(0.0))
    };
    let readout = |seconds: Option<f64>| text(format_seconds(seconds)).font(crate::theme::fonts::MONO);
    let times = row![
        readout(running.then_some(shown)),
        Space::new().width(Length::Fill),
        readout(running.then_some(duration - shown)),
    ];

    let seeks = row![
        key(Icon::Rewind, false, Tone::Plain, 1, running.then_some(Message::Rewind)),
        key(Icon::Repeat, repeating, Tone::Warning, 1, running.then_some(Message::ToggleRepeat)),
        key(Icon::FastForward, false, Tone::Plain, 1, running.then_some(Message::FastForward)),
    ]
    .spacing(2);

    let has_cue = !c.title.is_empty() || c.duration.is_some() || c.status.is_some();
    let transport = |message: header::Message| has_cue.then_some(Message::Transport(message));
    let buttons = row![
        key(
            Icon::Stop,
            header::lit_for(header::Message::Stop, status).strength(c.blink),
            Tone::Danger,
            1,
            transport(header::Message::Stop),
        ),
        key(
            Icon::Play,
            header::lit_for(header::Message::Play, status).strength(c.blink),
            Tone::Success,
            1,
            transport(header::Message::Play),
        ),
        key(
            Icon::Pause,
            header::lit_for(header::Message::ReadyPause, status).strength(c.blink),
            Tone::Warning,
            1,
            transport(header::Message::ReadyPause),
        ),
    ]
    .spacing(2);

    column![
        text(c.title).size(20),
        row![
            Space::new().width(Length::Fill),
            text(format_seconds(c.duration)).font(crate::theme::fonts::MONO),
        ],
        skips,
        Space::new().height(Length::Fill),
        bar,
        times,
        seeks,
        buttons,
    ]
    .spacing(12)
    .padding(12)
    .into()
}

#[derive(Clone, Copy)]
enum Tone {
    Plain,
    Success,
    Warning,
    Danger,
}

/// A big button of a row, `portion` parts wide. Lit ones take the color of
/// what they mean.
fn key<'a>(
    glyph: Icon,
    lit: impl Into<Strength>,
    tone: Tone,
    portion: u16,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let lit = lit.into();
    let pair = move |theme: &Theme| {
        let palette = theme.extended_palette();
        match tone {
            Tone::Plain => palette.secondary.base,
            Tone::Success => palette.success.base,
            Tone::Warning => palette.warning.base,
            Tone::Danger => palette.danger.base,
        }
    };

    button(
        container(colored(glyph, 26.0, move |theme: &Theme| {
            if lit.shows_accent() {
                pair(theme).text
            } else {
                theme.extended_palette().background.base.text
            }
        }))
        .center_x(Length::Fill),
    )
    .width(Length::FillPortion(portion))
    .padding([14, 0])
    .style(move |theme: &Theme, status| {
        let base = button::secondary(theme, status);
        if !lit.is_lit() {
            return base;
        }
        let pair = pair(theme);
        button::Style {
            background: Some(pair.color.scale_alpha(lit.0).into()),
            text_color: if lit.shows_accent() { pair.text } else { base.text_color },
            ..base
        }
    })
    .on_press_maybe(on_press)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::TemplateSettings;

    fn wait(id: Uuid) -> Cue {
        let mut cue = TemplateSettings::default().wait;
        cue.id = id;
        cue
    }

    /// a, group(x, y), c
    fn show() -> (Vec<Uuid>, HashMap<Uuid, Cue>, [Uuid; 5]) {
        let [a, g, x, y, c] = [(); 5].map(|()| Uuid::new_v4());
        let mut cues = HashMap::new();
        for id in [a, x, y, c] {
            cues.insert(id, wait(id));
        }
        let mut group = TemplateSettings::default().group;
        group.id = g;
        if let CueParam::Group { children, .. } = &mut group.params {
            *children = vec![x, y];
        }
        cues.insert(g, group);
        (vec![a, g, c], cues, [a, g, x, y, c])
    }

    #[test]
    fn next_and_previous_stay_on_the_level() {
        let (roots, cues, [a, g, x, y, c]) = show();
        let go = |direction, from| skip(direction, Some(from), &roots, &cues);

        assert_eq!(go(Skip::Next, a), Some(g));
        // The group's children are on another level: they are passed over.
        assert_eq!(go(Skip::Next, g), Some(c));
        assert_eq!(go(Skip::Previous, c), Some(g));
        assert_eq!(go(Skip::Next, x), Some(y));
        assert_eq!(go(Skip::Previous, y), Some(x));
        assert_eq!(go(Skip::Previous, x), None);
        assert_eq!(go(Skip::Next, c), None);
    }

    #[test]
    fn parent_and_child_move_between_levels() {
        let (roots, cues, [a, g, x, y, _]) = show();
        let go = |direction, from| skip(direction, Some(from), &roots, &cues);

        assert_eq!(go(Skip::Parent, y), Some(g));
        assert_eq!(go(Skip::Parent, a), None);
        assert_eq!(go(Skip::Child, g), Some(x));
        assert_eq!(go(Skip::Child, a), None);
    }

    #[test]
    fn without_a_cursor_the_ends_are_taken() {
        let (roots, cues, [a, _, _, _, c]) = show();
        assert_eq!(skip(Skip::Previous, None, &roots, &cues), Some(a));
        assert_eq!(skip(Skip::Next, None, &roots, &cues), Some(c));
        assert_eq!(skip(Skip::Parent, None, &roots, &cues), None);
    }

    #[test]
    fn the_controls_screen_needs_the_permission() {
        assert_eq!(effective_tab(Tab::Controls, false), Tab::List);
        assert_eq!(effective_tab(Tab::Controls, true), Tab::Controls);
        assert_eq!(effective_tab(Tab::Active, false), Tab::Active);
    }
}
