// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Gives what is inside a scrollable the pointer positions of its layout.
//!
//! A scrollable shifts the cursor it hands to its content by the scroll
//! offset, so that it can be compared with the content's layout, but the
//! positions inside events (the mouse's `CursorMoved`, a finger's position)
//! reach the content as they are in the window. A widget that tests such a
//! position against its bounds would be off by the scroll offset.
//!
//! Put this directly inside the scrollable: it works out the offset from
//! the viewport the scrollable passes down, and shifts every pointer
//! position in the events it forwards by it. When the offset changes under
//! a pointer that does not move, the content is told where the pointer now
//! is, as a `CursorMoved` would.
//!
//! While a pointer is held down the offset the content is given stays the
//! one at the press, so that the distance the pointer travels from the
//! press does not depend on the content scrolling meanwhile (it can: a
//! drag that narrows the content can bring the offset back to zero). The
//! content is caught up when the pointer is released.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, touch};

#[derive(Debug, Default)]
struct State {
    /// The scroll offset the content was last caught up with.
    translation: Vector,
    /// The offset in force since a pointer was pressed, until it is released.
    held: Option<Vector>,
    /// Where the mouse was last seen, in the window.
    last_mouse: Option<Point>,
}

pub struct ScrollSpace<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

pub fn scroll_space<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> ScrollSpace<'a, Message, Theme, Renderer> {
    ScrollSpace {
        content: content.into(),
    }
}

/// The event with its pointer position moved by `by`.
fn shifted(event: &Event, by: Vector) -> Event {
    match event {
        Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Event::Mouse(mouse::Event::CursorMoved {
                position: *position + by,
            })
        }
        Event::Touch(touch::Event::FingerPressed { id, position }) => {
            Event::Touch(touch::Event::FingerPressed {
                id: *id,
                position: *position + by,
            })
        }
        Event::Touch(touch::Event::FingerMoved { id, position }) => {
            Event::Touch(touch::Event::FingerMoved {
                id: *id,
                position: *position + by,
            })
        }
        Event::Touch(touch::Event::FingerLifted { id, position }) => {
            Event::Touch(touch::Event::FingerLifted {
                id: *id,
                position: *position + by,
            })
        }
        Event::Touch(touch::Event::FingerLost { id, position }) => {
            Event::Touch(touch::Event::FingerLost {
                id: *id,
                position: *position + by,
            })
        }
        other => other.clone(),
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ScrollSpace<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // The scrollable's viewport is its own bounds moved by the offset,
        // and this widget starts at its bounds' origin.
        let bounds = layout.bounds();
        let translation = Vector::new(viewport.x - bounds.x, viewport.y - bounds.y);

        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                state.last_mouse = Some(*position);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                state.held.get_or_insert(translation);
            }
            _ => {}
        }
        let given = state.held.unwrap_or(translation);

        let shifted_event = shifted(event, given);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            &shifted_event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
        ) {
            state.held = None;
        }

        // Catch the content up with an offset that changed under the pointer.
        if state.held.is_none() {
            let changed = translation != state.translation;
            state.translation = translation;
            if changed
                && !matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. }))
                && let Some(position) = state.last_mouse
            {
                let event = Event::Mouse(mouse::Event::CursorMoved {
                    position: position + translation,
                });
                self.content.as_widget_mut().update(
                    &mut tree.children[0],
                    &event,
                    layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<ScrollSpace<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(widget: ScrollSpace<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_positions_are_moved_and_other_events_are_not() {
        let by = Vector::new(100.0, 0.0);
        let at = Point::new(52.0, 8.0);
        assert!(matches!(
            shifted(&Event::Mouse(mouse::Event::CursorMoved { position: at }), by),
            Event::Mouse(mouse::Event::CursorMoved { position }) if position == Point::new(152.0, 8.0)
        ));
        let id = touch::Finger(1);
        assert!(matches!(
            shifted(&Event::Touch(touch::Event::FingerPressed { id, position: at }), by),
            Event::Touch(touch::Event::FingerPressed { id: shifted_id, position })
                if shifted_id == id && position == Point::new(152.0, 8.0)
        ));
        assert!(matches!(
            shifted(&Event::Touch(touch::Event::FingerLost { id, position: at }), by),
            Event::Touch(touch::Event::FingerLost { position, .. }) if position == Point::new(152.0, 8.0)
        ));
        assert!(matches!(
            shifted(
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                by
            ),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ));
    }
}
