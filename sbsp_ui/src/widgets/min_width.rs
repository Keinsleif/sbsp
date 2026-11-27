// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A width floor for rows that would otherwise shrink with the window.
//!
//! A column that is `Fill` wide gets squeezed without limit, and inside a
//! horizontal scrollable a `Fill` is worth nothing (the width there is
//! unbounded). So the width has to be handed down: [`min_width`] asks for
//! the room there is, builds the content at `max(room, min)` and, when the
//! room is smaller than `min`, puts it in a horizontal scrollable (in which
//! the pointer positions of events are given in the content's own space,
//! see [`scroll_space`]). When the content fits, the scrollable is still
//! there but is kept from taking touch gestures (see [`touch_gate`]), so a
//! swipe goes on to the scrollable around it.

use crate::widgets::scroll_space::scroll_space;
use crate::widgets::touch_gate::touch_gate;
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
        let is_scrollable = room.width < min;
        let scroll_bar = if is_scrollable {
            scrollable::Scrollbar::default()
        } else {
            scrollable::Scrollbar::hidden()
        };
        let content_width = if is_scrollable { min } else { room.width };
        let scrolling: Element<'_, Message> = scrollable(scroll_space(content(content_width)))
            .direction(scrollable::Direction::Horizontal(scroll_bar))
            .width(Length::Fill)
            .height(Length::Shrink)
            .into();
        // The scrollable stays when the content fits, so that what is inside
        // keeps its state across the change, but it does not take touches.
        touch_gate(scrolling, !is_scrollable).into()
    })
}
