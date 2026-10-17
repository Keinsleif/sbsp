// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! One-pixel lines between the sections of the window.

use iced::widget::{Space, container};
use iced::{Element, Length, Theme};

fn line(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.strong.color.into()),
        ..Default::default()
    }
}

/// A line across the full width.
pub fn horizontal<'a, Message: 'a>() -> Element<'a, Message> {
    container(Space::new())
        .width(Length::Fill)
        .height(1)
        .style(line)
        .into()
}

/// A line over the full height of its row.
pub fn vertical<'a, Message: 'a>() -> Element<'a, Message> {
    container(Space::new())
        .width(1)
        .height(Length::Fill)
        .style(line)
        .into()
}
