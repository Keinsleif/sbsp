// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The top panel under the menu: a wall clock, the stop/play/pause
//! buttons for the playback-cursor cue, and that cue's title and notes.
//!
//! Layout is the original's: a fixed-height panel in two columns. Left,
//! the clock (stretching to fill) over the transport buttons; right, the
//! cue title and notes (stretching) over the add-cue buttons. A bottom
//! row exists only while its buttons do -- transport buttons outside view
//! mode, add-cue buttons in edit mode -- and when one is missing the box
//! above it grows into the space, so the two columns' boxes need not be
//! the same height.
//!
//! The add-cue buttons are always enabled, as in the original; the ones
//! that need a selected cue simply add nothing without it.

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Theme};

use sbsp_backend::controller::state::PlaybackStatus;
use sbsp_backend::model::cue::Cue;

use crate::domain::new_cue::CueKind;
use crate::port::Control;
use crate::widgets::blink::{Lit, Strength};
use crate::widgets::icon::{Icon, glyph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    Stop,
    Play,
    ReadyPause,
    /// An add-cue button (edit mode).
    AddCue(CueKind),
}

/// Wall-clock time of day, as shown (24h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClockTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// The local time of day now.
pub fn local_clock() -> ClockTime {
    use chrono::Timelike;

    let now = chrono::Local::now();
    ClockTime {
        hour: now.hour(),
        minute: now.minute(),
        second: now.second(),
    }
}

/// What a header button sends for the cue under the playback cursor,
/// given that cue's current playback status (`None` when it is idle).
///
/// Play resumes a paused cue and otherwise executes it. The third button
/// is both "ready" and "pause": it loads an idle cue, pauses a running one
/// and resumes a paused one.
pub fn controls_for(
    message: Message,
    cursor: Option<sbsp_backend::model::cue::Uuid>,
    status: Option<PlaybackStatus>,
) -> Vec<Control> {
    let Some(id) = cursor else {
        return Vec::new();
    };

    use PlaybackStatus as S;
    match message {
        // Not playback: the app adds a cue.
        Message::AddCue(_) => Vec::new(),
        Message::Stop => vec![Control::Stop(id)],
        Message::Play => match status {
            Some(S::Paused | S::PreWaitPaused) => vec![Control::Resume(id)],
            _ => vec![Control::Execute(id)],
        },
        Message::ReadyPause => match status {
            Some(S::PreWaiting | S::Playing) => vec![Control::Pause(id)],
            Some(S::PreWaitPaused | S::Paused) => vec![Control::Resume(id)],
            None => vec![Control::Load(id)],
            Some(S::Loaded | S::Stopping) => Vec::new(),
        },
    }
}

/// How a transport button is lit for the cue under the playback cursor:
/// stop while stopping, play while playing, pause while paused. The cue
/// being pre-waited (play) or loaded (pause) is shown flashing.
pub fn lit_for(message: Message, status: Option<PlaybackStatus>) -> Lit {
    use PlaybackStatus as S;
    match (message, status) {
        (Message::Stop, Some(S::Stopping)) => Lit::Flashing,
        (Message::Play, Some(S::Playing)) => Lit::On,
        (Message::Play, Some(S::PreWaiting)) => Lit::Flashing,
        (Message::ReadyPause, Some(S::Paused)) => Lit::On,
        (Message::ReadyPause, Some(S::Loaded)) => Lit::Flashing,
        _ => Lit::Off,
    }
}

/// "number・name" for the title bar, leaving out an empty number.
pub fn cue_title(cue: &Cue, name: String) -> String {
    [cue.number.as_str(), name.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ・ ")
}

pub struct View<'a> {
    pub clock: ClockTime,
    pub window_focused: bool,
    /// Show the transport buttons (hidden in view mode and by the
    /// `hide_controls` appearance setting).
    pub show_controls: bool,
    /// Show the add-cue button row (edit mode).
    pub show_add_cue: bool,
    pub cursor_cue: Option<&'a Cue>,
    /// Display name of `cursor_cue` (its own, or the generated one).
    pub cursor_cue_name: String,
    pub cursor_status: Option<PlaybackStatus>,
    /// The flashing level of the transport buttons (see `widgets::blink`).
    pub blink: f32,
}

pub fn view<'a>(v: View<'a>) -> Element<'a, Message> {
    let focused = v.window_focused;

    let clock = container(
        row![
            text(format!("{:02}:{:02}.", v.clock.hour, v.clock.minute))
                .size(54)
                .line_height(1.0)
                .font(crate::theme::fonts::MONO),
            text(format!("{:02}", v.clock.second))
                .size(40)
                .line_height(1.0)
                .font(crate::theme::fonts::MONO),
        ]
        .align_y(Alignment::End),
    )
    .padding([0, 12])
    .center_y(Length::Fill)
    .style(move |theme: &Theme| {
        // A red clock is the "this window does not have keyboard focus,
        // your hotkeys are going elsewhere" warning, as in the original.
        let mut style = container::bordered_box(theme);
        if !focused {
            style.background = Some(theme.extended_palette().danger.base.color.into());
        }
        style
    });

    let mut left = column![clock].spacing(12).height(Length::Fill);
    if v.show_controls {
        let has_cursor = v.cursor_cue.is_some();
        let lit = |message| lit_for(message, v.cursor_status).strength(v.blink);

        let transport = row![
            transport_button(
                Icon::Stop,
                lit(Message::Stop),
                Danger,
                has_cursor.then_some(Message::Stop),
            ),
            transport_button(
                Icon::Play,
                lit(Message::Play),
                Success,
                has_cursor.then_some(Message::Play),
            ),
            transport_button(
                Icon::Pause,
                lit(Message::ReadyPause),
                Warning,
                has_cursor.then_some(Message::ReadyPause),
            ),
        ]
        .spacing(2);
        left = left.push(transport);
    }

    let title = v
        .cursor_cue
        .map(|cue| cue_title(cue, v.cursor_cue_name.clone()))
        .unwrap_or_default();
    let notes = v.cursor_cue.map(|cue| cue.notes.clone()).unwrap_or_default();

    let cue_box = column![
        container(text(title))
            .padding([4, 12])
            .width(Length::Fill)
            .style(container::bordered_box),
        container(scrollable(text(notes).width(Length::Fill)))
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(container::bordered_box),
    ]
    .spacing(4)
    .height(Length::Fill);

    let mut right = column![cue_box].spacing(12);
    if v.show_add_cue {
        right = right.push(add_cue_row());
    }

    row![left, right.width(Length::Fill)]
        .spacing(12)
        .padding(12)
        .height(PANEL_HEIGHT)
        .into()
}

/// The original's `h-50`.
const PANEL_HEIGHT: f32 = 200.0;

/// Add-cue buttons. Same size as the transport buttons, so the two bottom
/// rows line up.
fn add_cue_row<'a>() -> Element<'a, Message> {
    let group = |buttons: &[(Icon, CueKind)]| {
        row(buttons.iter().map(|(glyph, kind)| {
            transport_button(*glyph, false, Success, Some(Message::AddCue(*kind)))
        }))
        .spacing(2)
    };

    container(
        row![
            group(&[
                (Icon::VolumeHigh, CueKind::Audio),
                (Icon::TimerSandEmpty, CueKind::Wait),
                (Icon::ChartBellCurveCumulative, CueKind::Fade),
            ]),
            group(&[(Icon::Group, CueKind::Group)]),
            group(&[
                (Icon::PlayCircleOutline, CueKind::Start),
                (Icon::StopCircleOutline, CueKind::Stop),
                (Icon::PauseCircleOutline, CueKind::Pause),
                (Icon::UploadCircleOutline, CueKind::Load),
            ]),
        ]
        .spacing(16),
    )
    .align_right(Length::Fill)
    .into()
}

#[derive(Clone, Copy)]
enum Highlight {
    Danger,
    Success,
    Warning,
}
use Highlight::{Danger, Success, Warning};

fn transport_button<'a>(
    icon: Icon,
    strength: impl Into<Strength>,
    highlight: Highlight,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let strength = strength.into();
    let glyph = glyph(icon, 22.0);

    button(container(glyph).center_x(Length::Fill))
        .width(44)
        .padding([6, 0])
        .style(move |theme: &Theme, status| {
            let base = button::secondary(theme, status);
            if !strength.is_lit() {
                return base;
            }
            let palette = theme.extended_palette();
            let pair = match highlight {
                Highlight::Danger => palette.danger.base,
                Highlight::Success => palette.success.base,
                Highlight::Warning => palette.warning.base,
            };
            button::Style {
                background: Some(pair.color.scale_alpha(strength.0).into()),
                text_color: if strength.shows_accent() { pair.text } else { base.text_color },
                ..base
            }
        })
        .on_press_maybe(on_press)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::model::cue::{
        CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid, WaitCueParam,
    };

    fn cue(number: &str) -> Cue {
        Cue {
            id: Uuid::new_v4(),
            number: number.to_string(),
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
    fn the_transport_buttons_light_for_their_status() {
        use PlaybackStatus as S;

        assert_eq!(lit_for(Message::Stop, Some(S::Stopping)), Lit::Flashing);
        assert_eq!(lit_for(Message::Play, Some(S::Playing)), Lit::On);
        assert_eq!(lit_for(Message::Play, Some(S::PreWaiting)), Lit::Flashing);
        assert_eq!(lit_for(Message::ReadyPause, Some(S::Paused)), Lit::On);
        assert_eq!(lit_for(Message::ReadyPause, Some(S::Loaded)), Lit::Flashing);
        assert_eq!(lit_for(Message::Play, None), Lit::Off);
        assert_eq!(lit_for(Message::Stop, Some(S::Playing)), Lit::Off);
    }

    #[test]
    fn no_cursor_means_no_commands() {
        for message in [Message::Stop, Message::Play, Message::ReadyPause] {
            assert!(controls_for(message, None, None).is_empty());
        }
    }

    #[test]
    fn play_executes_unless_the_cue_is_paused() {
        let id = Uuid::new_v4();
        use PlaybackStatus as S;

        assert_eq!(controls_for(Message::Play, Some(id), None), vec![Control::Execute(id)]);
        assert_eq!(
            controls_for(Message::Play, Some(id), Some(S::Playing)),
            vec![Control::Execute(id)]
        );
        assert_eq!(
            controls_for(Message::Play, Some(id), Some(S::Paused)),
            vec![Control::Resume(id)]
        );
        assert_eq!(
            controls_for(Message::Play, Some(id), Some(S::PreWaitPaused)),
            vec![Control::Resume(id)]
        );
    }

    #[test]
    fn the_third_button_loads_pauses_or_resumes() {
        let id = Uuid::new_v4();
        use PlaybackStatus as S;
        let press = |status| controls_for(Message::ReadyPause, Some(id), status);

        assert_eq!(press(None), vec![Control::Load(id)]);
        assert_eq!(press(Some(S::Playing)), vec![Control::Pause(id)]);
        assert_eq!(press(Some(S::PreWaiting)), vec![Control::Pause(id)]);
        assert_eq!(press(Some(S::Paused)), vec![Control::Resume(id)]);
        assert!(press(Some(S::Loaded)).is_empty());
        assert!(press(Some(S::Stopping)).is_empty());
    }

    #[test]
    fn an_add_button_sends_no_playback_command() {
        let id = Uuid::new_v4();
        assert!(controls_for(Message::AddCue(CueKind::Wait), Some(id), None).is_empty());
    }

    #[test]
    fn stop_stops_the_cursor_cue() {
        let id = Uuid::new_v4();
        assert_eq!(
            controls_for(Message::Stop, Some(id), Some(PlaybackStatus::Playing)),
            vec![Control::Stop(id)]
        );
    }

    #[test]
    fn title_joins_number_and_name_and_skips_an_empty_number() {
        assert_eq!(cue_title(&cue("12"), "Intro".into()), "12・Intro");
        assert_eq!(cue_title(&cue(""), "Intro".into()), "Intro");
    }
}
