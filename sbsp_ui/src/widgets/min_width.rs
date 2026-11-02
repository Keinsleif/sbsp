// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A width floor for rows that would otherwise shrink with the window.
//!
//! A column that is `Fill` wide gets squeezed without limit, and inside a
//! horizontal scrollable a `Fill` is worth nothing (the width there is
//! unbounded). So the width has to be handed down: [`min_width`] asks for
//! the room there is, builds the content at `max(room, min)` and, when the
//! room is smaller than `min`, puts it in a horizontal scrollable.

use iced::widget::{Responsive, responsive, scrollable};
use iced::{Element, Length};

/// `content(width)` builds the content at exactly `width`.
///
/// The widget fills the room it is given. Inside a vertical scrollable,
/// call `.height(Length::Shrink)` on the result: the height is not known
/// there.
pub fn min_width<'a, Message: 'a>(
    min: f32,
    content: impl Fn(f32) -> Element<'a, Message> + 'a,
) -> Responsive<'a, Message> {
    responsive(move |room| {
        if room.width >= min {
            content(room.width)
        } else {
            scrollable(content(min))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::default(),
                ))
                .width(Length::Fill)
                .height(Length::Shrink)
                .into()
        }
    })
}
