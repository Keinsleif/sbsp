// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A horizontal progress fill: a colored block covering the first
//! `ratio` of the width, nothing behind it (put it in a `stack!` over
//! whatever the track should look like).

use iced::widget::{Space, container, row};
use iced::{Color, Element, Length, Theme};

/// Thousandths, used as `FillPortion` weights for the filled and the empty
/// part.
const RESOLUTION: u16 = 1000;

/// `ratio` is clamped to 0..=1. The bar fills its parent's width and
/// height.
pub fn bar<'a, Message: 'a>(
    ratio: f32,
    color: impl Fn(&Theme) -> Color + 'a,
) -> Element<'a, Message> {
    let filled = (ratio.clamp(0.0, 1.0) * RESOLUTION as f32).round() as u16;

    if filled == 0 {
        return Space::new().into();
    }

    let fill = container(Space::new())
        .width(Length::FillPortion(filled))
        .height(Length::Fill)
        .style(move |theme: &Theme| container::Style {
            background: Some(color(theme).into()),
            ..Default::default()
        });
    let rest = Space::new().width(Length::FillPortion(RESOLUTION - filled.min(RESOLUTION - 1)));

    row![fill, rest].height(Length::Fill).into()
}
