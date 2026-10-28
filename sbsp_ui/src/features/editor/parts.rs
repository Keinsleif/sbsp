// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Pieces the editor's tabs share: drop-down lists, check boxes, the list
//! of cues to pick from, and the fade parameter box.

use std::collections::HashMap;
use std::fmt;

use iced::widget::{button, column, container, pick_list, row, text};
use iced::{Alignment, Element, Length, Theme};

use sbsp_backend::model::cue::{Cue, Uuid, audio::FadeParam};
use crate::settings::NameFormatSettings;

use crate::domain::cue::display_name;
use crate::domain::cue_edit::{FADE_POWER, FadeSlot};
use crate::domain::easing::{Curve, CurveKind};
use crate::domain::field::TimeSpec;
use crate::domain::flat_list::FlatCueEntry;
use crate::fl;
use crate::widgets::curve::curve_view;
use crate::widgets::field::{self, Drafts};
use crate::widgets::icon::{Icon, icon};
use crate::widgets::inputs::{NumberInput, labeled, number_input, time_input};

use super::{Context, Message, fade_fields};

/// An option with its translated label. Equality is on the value alone,
/// so the selection survives a language switch.
#[derive(Clone)]
pub(super) struct Choice<T> {
    pub value: T,
    pub label: String,
}

impl<T> Choice<T> {
    pub fn new(value: T, label: String) -> Self {
        Self { value, label }
    }
}

impl<T: PartialEq> PartialEq for Choice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// A drop-down list; without `enabled` just the current choice, greyed
/// out (`pick_list` has no disabled state). `selected` that is not among
/// the options (or `None`) shows nothing.
pub(super) fn picker<'a, T>(
    options: Vec<Choice<T>>,
    selected: Option<&T>,
    enabled: bool,
    on_select: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: Clone + PartialEq + 'a,
{
    let current = selected.and_then(|selected| {
        options
            .iter()
            .find(|option| option.value == *selected)
            .cloned()
    });

    if !enabled {
        let label = current.map(|choice| choice.label).unwrap_or_default();
        return container(text(label))
            .padding([5, 10])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let mut style = container::bordered_box(theme);
                style.text_color = Some(
                    theme.extended_palette().background.base.text.scale_alpha(0.5),
                );
                style
            })
            .into();
    }

    pick_list(options, current, move |choice: Choice<T>| on_select(choice.value))
        .width(Length::Fill)
        .into()
}

/// A labeled check box (a button, as iced's own check box is not used
/// here). Without `on_press` it is greyed out.
pub(super) fn check<'a>(
    label: String,
    checked: bool,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let glyph = if checked {
        Icon::CheckboxMarked
    } else {
        Icon::CheckboxBlankOutline
    };
    button(row![icon(glyph, 20.0), text(label)].spacing(8).align_y(Alignment::Center))
        .style(button::text)
        .on_press_maybe(on_press)
        .into()
}

pub(super) fn choice_name(
    cue: &Cue,
    cues: &HashMap<Uuid, Cue>,
    name_format: &NameFormatSettings,
) -> String {
    crate::features::header::cue_title(cue, display_name(cue, cues, name_format))
}

/// The cues a drop-down can offer, in list order, that pass `accept`.
pub(super) fn cue_choices(
    ctx: &Context<'_>,
    flat: &[FlatCueEntry],
    accept: impl Fn(&Cue) -> bool,
) -> Vec<Choice<Uuid>> {
    let cues = &ctx.model.cue_list.cues;
    flat.iter()
        .filter_map(|entry| cues.get(&entry.cue_id))
        .filter(|cue| accept(cue))
        .map(|cue| Choice::new(cue.id, choice_name(cue, cues, ctx.name_format)))
        .collect()
}

pub(super) struct FadeInput<'a> {
    pub slot: FadeSlot,
    pub label: String,
    pub param: Option<FadeParam>,
    /// Whether the fade can be switched off (audio cues' fades).
    pub toggle: bool,
    /// Draw the fade-in curve and the fade-out curve (a fade cue goes
    /// either way); otherwise only the one that fits the slot.
    pub both_curves: bool,
    pub enabled: bool,
    pub drafts: &'a Drafts,
}

/// The original's `FadeParamInput`: an on/off box, the duration, the curve
/// and its intensity, and a picture of the curve.
pub(super) fn fade_param_input<'a>(input: FadeInput<'a>) -> Element<'a, Message> {
    let FadeInput {
        slot,
        label,
        param,
        toggle,
        both_curves,
        enabled,
        drafts,
    } = input;
    let (duration_id, power_id) = fade_fields(slot);

    let on = param.is_some();
    let fields_enabled = enabled && on;
    let curve = param.map(|param| Curve::from(param.easing));

    let mut line = row![].spacing(8).align_y(Alignment::End);
    if toggle {
        line = line.push(check(
            String::new(),
            on,
            enabled.then_some(Message::FadeEnabled(slot, !on)),
        ));
    }

    line = line.push(
        container(
            time_input(
                duration_id,
                Some(&fl!("main-duration")),
                param.map(|param| param.duration),
                &TimeSpec::default(),
                drafts,
                fields_enabled,
            )
            .map(Message::Field),
        )
        .width(Length::Fixed(100.0)),
    );

    let kinds = [
        CurveKind::Linear,
        CurveKind::InPow,
        CurveKind::OutPow,
        CurveKind::InOutPow,
    ]
    .into_iter()
    .map(|kind| Choice::new(kind, kind_label(kind)))
    .collect();
    line = line.push(
        container(labeled(
            Some(&fl!("main-bottom-editor-input-curve")),
            picker(
                kinds,
                curve.as_ref().map(|curve| &curve.kind),
                fields_enabled,
                move |kind| Message::FadeKind(slot, kind),
            ),
        ))
        .width(Length::Fixed(120.0)),
    );

    let linear = curve.is_none_or(|curve| curve.kind == CurveKind::Linear);
    let power: Element<'a, field::Message> = number_input(
        NumberInput {
            id: power_id,
            label: None,
            prefix: None,
            suffix: None,
            value: curve.and_then(|curve| curve.power),
            spec: FADE_POWER,
            show_buttons: true,
            enabled: fields_enabled && !linear,
        },
        drafts,
    );
    line = line.push(
        container(labeled(
            Some(&fl!("main-bottom-editor-input-intensity")),
            power.map(Message::Field),
        ))
        .width(Length::Fixed(130.0)),
    );

    // The pictures show the fade even while the box is greyed out by an
    // active cue, but not while the fade is off.
    let falling = slot == FadeSlot::AudioOut;
    if both_curves {
        line = line
            .push(curve_view::<Message>(curve, false))
            .push(curve_view::<Message>(curve, true));
    } else {
        line = line.push(curve_view::<Message>(curve, falling));
    }

    container(column![text(label).size(12), line].spacing(4))
        .padding(8)
        .style(container::bordered_box)
        .into()
}

fn kind_label(kind: CurveKind) -> String {
    match kind {
        CurveKind::Linear => fl!("main-bottom-editor-input-linear"),
        CurveKind::InPow => fl!("main-bottom-editor-input-ease-in"),
        CurveKind::OutPow => fl!("main-bottom-editor-input-ease-out"),
        CurveKind::InOutPow => fl!("main-bottom-editor-input-ease-in-out"),
    }
}
