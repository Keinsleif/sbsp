// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The input parts the editors are built from. Ported from
//! `components/input/{NumberInput,TimeInput,VolumeInput,VolumeFader,
//! PanningFader}.vue`.
//!
//! None of these hold a value. A field shows what it is given and reports
//! what the user does as [`field::Message`]; the owner of the value runs
//! the message through [`field::Drafts`] and turns a finished edit into a
//! value with the matching `domain::field::commit_*`. The faders report
//! their own movement (`FaderMessage`): `Moved` while dragging (the owner
//! updates its local value) and `Released` at the end (the owner sends the
//! change to the backend once, as the original did on pointer-up).

use std::time::{Duration, Instant};

use iced::widget::{Space, button, canvas, column, row, slider, text};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse};

use crate::domain::fader::{decibels_to_fader, fader_to_decibels};
use crate::domain::field::{NumberSpec, TimeSpec, format_number, format_time, format_volume};
use crate::widgets::field::{self, Drafts, FieldId, text_field};
use crate::widgets::icon::{Icon, Tone, toned};

/// A small caption above a field (the original's floating label).
pub fn labeled<'a, Message: 'a>(
    label: Option<&str>,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    match label {
        Some(label) => column![
            text(label.to_string()).size(12).style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.base.text.scale_alpha(0.7)),
            }),
            content.into()
        ]
        .spacing(2)
        .into(),
        None => content.into(),
    }
}

// ---------------------------------------------------------------------
// Number

pub struct NumberInput<'a> {
    pub id: FieldId,
    pub label: Option<&'a str>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub value: Option<f64>,
    pub spec: NumberSpec,
    pub show_buttons: bool,
    pub enabled: bool,
}

pub fn number_input<'a>(input: NumberInput<'a>, drafts: &Drafts) -> Element<'a, field::Message> {
    let NumberInput {
        id,
        label,
        prefix,
        suffix,
        value,
        spec,
        show_buttons,
        enabled,
    } = input;

    let shown = format_number(value, spec.precision);
    let mut line = row![].spacing(4).align_y(Alignment::Center);

    if let Some(prefix) = prefix {
        line = line.push(text(prefix));
    }
    line = line.push(text_field(id, &shown, drafts, enabled).width(Length::Fill));
    if let Some(suffix) = suffix {
        line = line.push(text(suffix));
    }
    if show_buttons {
        line = line
            .push(step_button(Icon::ChevronDown, id, -1, enabled))
            .push(step_button(Icon::ChevronUp, id, 1, enabled));
    }

    labeled(label, line)
}

fn step_button<'a>(
    glyph: Icon,
    id: FieldId,
    direction: i8,
    enabled: bool,
) -> Element<'a, field::Message> {
    button(toned(glyph, 16.0, Tone::Text))
        .padding(4)
        .style(button::text)
        .on_press_maybe(enabled.then_some(field::Message::Step(id, direction)))
        .into()
}

// ---------------------------------------------------------------------
// Time

pub fn time_input<'a>(
    id: FieldId,
    label: Option<&str>,
    value: Option<f64>,
    spec: &TimeSpec,
    drafts: &Drafts,
    enabled: bool,
) -> Element<'a, field::Message> {
    let shown = format_time(value, spec);
    let field = text_field(id, &shown, drafts, enabled)
        .align_x(iced::alignment::Horizontal::Center)
        .width(Length::Fill);

    labeled(label, field)
}

// ---------------------------------------------------------------------
// Volume

pub fn volume_input<'a>(
    id: FieldId,
    volume: f64,
    drafts: &Drafts,
    enabled: bool,
) -> Element<'a, field::Message> {
    text_field(id, &format_volume(volume), drafts, enabled)
        .width(100)
        .into()
}

// ---------------------------------------------------------------------
// Faders

#[derive(Debug, Clone, PartialEq)]
pub enum FaderMessage {
    /// The number field next to the slider.
    Field(field::Message),
    /// The value while dragging: dB for the volume fader, -1..1 for the
    /// panning fader.
    Moved(f64),
    /// The drag ended; send the value.
    Released,
    /// A disabled fader's slider still has to produce something.
    Nothing,
}

/// How many tick labels a volume fader shows (the original thins them out
/// on narrow layouts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThumbAmount {
    #[default]
    Full,
    Decreased,
    BaseOnly,
}

/// (fader position, label). The labels name the dB the position stands
/// for: the fader is not linear below -10 dB (`domain::fader`).
fn volume_ticks(amount: ThumbAmount) -> &'static [(f64, &'static str)] {
    match amount {
        ThumbAmount::Full => &[
            (10.0, "10"),
            (5.0, "5"),
            (0.0, "0"),
            (-5.0, "-5"),
            (-10.0, "-10"),
            (-15.0, "-20"),
            (-20.0, "-30"),
            (-25.0, "-40"),
            (-30.0, "-60"),
        ],
        ThumbAmount::Decreased => &[(10.0, "10"), (0.0, "0"), (-10.0, "-10"), (-30.0, "-60")],
        ThumbAmount::BaseOnly => &[(0.0, "0")],
    }
}

const FADER_MIN: f64 = -30.0;
const FADER_MAX: f64 = 10.0;
const PANNING_RANGE: f64 = 64.0;

pub struct VolumeFader<'a> {
    pub id: FieldId,
    pub label: &'a str,
    pub volume: f64,
    pub thumb: ThumbAmount,
    pub enabled: bool,
}

pub fn volume_fader<'a>(fader: VolumeFader<'a>, drafts: &Drafts) -> Element<'a, FaderMessage> {
    let VolumeFader {
        id,
        label,
        volume,
        thumb,
        enabled,
    } = fader;

    let header = row![
        text(label.to_string()),
        Space::new().width(Length::Fill),
        volume_input(id, volume, drafts, enabled).map(FaderMessage::Field),
    ]
    .align_y(Alignment::Center);

    let position = decibels_to_fader(volume).clamp(FADER_MIN, FADER_MAX) as f32;
    let track = if enabled {
        slider(FADER_MIN as f32..=FADER_MAX as f32, position, |position| {
            FaderMessage::Moved(fader_to_decibels(f64::from(position)))
        })
        .step(0.05_f32)
        .on_release(FaderMessage::Released)
    } else {
        slider(FADER_MIN as f32..=FADER_MAX as f32, position, |_| FaderMessage::Nothing)
    };

    column![
        header,
        ticks(volume_ticks(thumb), FADER_MIN, FADER_MAX),
        track
    ]
    .spacing(2)
    .into()
}

pub struct PanningFader<'a> {
    pub id: FieldId,
    pub label: &'a str,
    pub panning: f64,
    pub enabled: bool,
}

pub fn panning_fader<'a>(fader: PanningFader<'a>, drafts: &Drafts) -> Element<'a, FaderMessage> {
    let PanningFader {
        id,
        label,
        panning,
        enabled,
    } = fader;

    let prefix = if panning < 0.0 {
        "L "
    } else if panning > 0.0 {
        "R "
    } else {
        "C "
    };
    let input = number_input(
        NumberInput {
            id,
            label: None,
            prefix: Some(prefix.to_string()),
            suffix: None,
            value: Some(panning),
            spec: NumberSpec {
                min: Some(-1.0),
                max: Some(1.0),
                step: 1.0 / 8.0,
                precision: 3,
                accept_null: false,
            },
            show_buttons: false,
            enabled,
        },
        drafts,
    )
    .map(FaderMessage::Field);

    let header = row![
        text(label.to_string()),
        Space::new().width(Length::Fill),
        row![input].width(140),
    ]
    .align_y(Alignment::Center);

    let position = (panning * PANNING_RANGE).clamp(-PANNING_RANGE, PANNING_RANGE) as f32;
    let range = -(PANNING_RANGE as f32)..=(PANNING_RANGE as f32);
    let track = if enabled {
        slider(range, position, |position| {
            FaderMessage::Moved(f64::from(position) / PANNING_RANGE)
        })
        .step(1.0_f32)
        .on_release(FaderMessage::Released)
    } else {
        slider(range, position, |_| FaderMessage::Nothing)
    };

    column![
        header,
        ticks(
            &[(-PANNING_RANGE, "L"), (0.0, "C"), (PANNING_RANGE, "R")],
            -PANNING_RANGE,
            PANNING_RANGE
        ),
        track
    ]
    .spacing(2)
    .into()
}

/// Two quick releases are a double click, which resets a fader to its
/// default. (A `mouse_area` around the slider cannot see it: the slider
/// captures the press.) One per fader, kept by whoever owns its value.
#[derive(Debug, Default, Clone, Copy)]
pub struct DoubleRelease {
    last: Option<Instant>,
}

impl DoubleRelease {
    pub const WINDOW: Duration = Duration::from_millis(400);

    /// Records a release; true if it completes a double click.
    pub fn released(&mut self, now: Instant) -> bool {
        let double = self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) <= Self::WINDOW);
        // A third quick release starts over rather than counting again.
        self.last = if double { None } else { Some(now) };
        double
    }
}

// ---------------------------------------------------------------------
// Tick labels

/// Room the slider's handle takes at each end of the track, so labels
/// line up with where the handle actually sits.
const HANDLE_INSET: f32 = 8.0;
const TICKS_HEIGHT: f32 = 14.0;

fn ticks<'a>(ticks: &[(f64, &'static str)], min: f64, max: f64) -> Element<'a, FaderMessage> {
    canvas::Canvas::new(Ticks {
        ticks: ticks.to_vec(),
        min,
        max,
    })
    .width(Length::Fill)
    .height(TICKS_HEIGHT)
    .into()
}

struct Ticks {
    ticks: Vec<(f64, &'static str)>,
    min: f64,
    max: f64,
}

impl<Message> canvas::Program<Message> for Ticks {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let color: Color = theme.extended_palette().background.base.text.scale_alpha(0.7);
        let span = (bounds.width - HANDLE_INSET * 2.0).max(0.0);

        for &(value, label) in &self.ticks {
            let fraction = ((value - self.min) / (self.max - self.min)) as f32;
            frame.fill_text(canvas::Text {
                content: label.to_string(),
                position: Point::new(HANDLE_INSET + span * fraction, 0.0),
                color,
                size: 10.0.into(),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Top,
                ..canvas::Text::default()
            });
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_second_quick_release_is_a_double_click() {
        let mut state = DoubleRelease::default();
        let start = Instant::now();

        assert!(!state.released(start));
        assert!(state.released(start + Duration::from_millis(200)));
    }

    #[test]
    fn slow_releases_are_not_a_double_click() {
        let mut state = DoubleRelease::default();
        let start = Instant::now();

        assert!(!state.released(start));
        assert!(!state.released(start + Duration::from_millis(900)));
        // ...but the slow one starts a new pair.
        assert!(state.released(start + Duration::from_millis(1000)));
    }

    #[test]
    fn a_third_quick_release_starts_over() {
        let mut state = DoubleRelease::default();
        let start = Instant::now();

        assert!(!state.released(start));
        assert!(state.released(start + Duration::from_millis(100)));
        assert!(!state.released(start + Duration::from_millis(200)));
    }

    #[test]
    fn every_tick_sits_inside_its_fader() {
        for amount in [ThumbAmount::Full, ThumbAmount::Decreased, ThumbAmount::BaseOnly] {
            for (position, _) in volume_ticks(amount) {
                assert!((FADER_MIN..=FADER_MAX).contains(position), "{position}");
            }
        }
    }

    #[test]
    fn the_tick_labels_name_the_decibels_of_their_position() {
        for (position, label) in volume_ticks(ThumbAmount::Full) {
            let decibels = fader_to_decibels(*position);
            let expected: f64 = label.parse().unwrap();
            assert!((decibels - expected).abs() < 1e-9, "{position}: {decibels} vs {label}");
        }
    }
}
