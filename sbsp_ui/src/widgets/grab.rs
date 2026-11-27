// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A handle that is dragged sideways with a mouse or a finger.
//!
//! The widget follows the drag itself, from the press to the release, and
//! reports how far the pointer has gone since it went down. The press
//! position comes from the events (the mouse's last known position, or the
//! finger's own), not from the cursor at the time the batch of events is
//! processed: the cursor is the latest one for every event of a batch, so a
//! press that is followed by a move out of the handle before the batch is
//! handled is still a press on the handle, and the first movement of a drag
//! is not lost. One pointer owns a drag: another finger cannot move or end it.
//!
//! Positions of events are compared with the widget's bounds as they are,
//! so inside a scrollable the content must be wrapped in
//! [`scroll_space`](super::scroll_space), which puts them in the space of
//! the layout.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, touch, window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pointer {
    Mouse,
    Finger(touch::Finger),
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    pointer: Pointer,
    start_x: f32,
}

#[derive(Debug, Default)]
struct State {
    /// Where the mouse was last seen, in the window.
    last_mouse: Option<Point>,
    drag: Option<Drag>,
}

pub struct Grab<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    on_start: Message,
    on_drag: Box<dyn Fn(f32) -> Message + 'a>,
    on_end: Message,
}

/// `on_start` is sent when a drag begins, `on_drag` with the distance the
/// pointer has moved sideways since then, `on_end` when it is over.
pub fn grab<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    on_start: Message,
    on_drag: impl Fn(f32) -> Message + 'a,
    on_end: Message,
) -> Grab<'a, Message, Theme, Renderer> {
    Grab {
        content: content.into(),
        on_start,
        on_drag: Box::new(on_drag),
        on_end,
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Grab<'_, Message, Theme, Renderer>
where
    Message: Clone,
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                state.last_mouse = Some(*position);
                if let Some(drag) = state.drag.filter(|drag| drag.pointer == Pointer::Mouse) {
                    shell.publish((self.on_drag)(position.x - drag.start_x));
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if state.drag.is_none() =>
            {
                // Where the press was, if the mouse has been seen to move;
                // otherwise where the cursor is.
                let pressed_at = state.last_mouse.or_else(|| cursor.position());
                if let Some(position) = pressed_at
                    && viewport.contains(position)
                    && layout.bounds().contains(position)
                {
                    state.drag = Some(Drag {
                        pointer: Pointer::Mouse,
                        start_x: position.x,
                    });
                    shell.publish(self.on_start.clone());
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state
                    .drag
                    .take_if(|drag| drag.pointer == Pointer::Mouse)
                    .is_some()
                {
                    shell.publish(self.on_end.clone());
                }
            }
            Event::Touch(touch::Event::FingerPressed { id, position })
                if state.drag.is_none()
                    && viewport.contains(*position)
                    && layout.bounds().contains(*position) =>
            {
                state.drag = Some(Drag {
                    pointer: Pointer::Finger(*id),
                    start_x: position.x,
                });
                shell.publish(self.on_start.clone());
            }
            Event::Touch(touch::Event::FingerMoved { id, position }) => {
                if let Some(drag) = state
                    .drag
                    .filter(|drag| drag.pointer == Pointer::Finger(*id))
                {
                    shell.publish((self.on_drag)(position.x - drag.start_x));
                }
            }
            Event::Touch(
                touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
            ) if state
                .drag
                .take_if(|drag| drag.pointer == Pointer::Finger(*id))
                .is_some() =>
            {
                shell.publish(self.on_end.clone());
            }
            // A release that happens outside the window after the focus is lost
            // is never delivered, so the drag ends with the focus.
            Event::Window(window::Event::Unfocused) if state.drag.take().is_some() => {
                shell.publish(self.on_end.clone());
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.drag.is_some() || cursor.is_over(layout.bounds()) {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::None
        }
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

impl<'a, Message, Theme, Renderer> From<Grab<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(grab: Grab<'a, Message, Theme, Renderer>) -> Self {
        Element::new(grab)
    }
}
