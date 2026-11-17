// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A modal dialog overlay.
//!
//! Unlike [`crate::widgets::toast`], this widget does not own any state of
//! its own: "which dialog is open, if any" is application state, held by
//! the app. This module only provides the overlay chrome: a dimmed,
//! click-to-dismiss backdrop and a centered card.
//!
//! ```ignore
//! // in `view`, once a dialog is open:
//! modal::over(main_content, dialog_card, Message::CloseDialog)
//! ```
//!
//! The backdrop and the card are separate `stack` layers rather than one
//! widget wrapping the other: `mouse_area::on_press` fires for any click
//! within its own bounds, so nesting the card inside the backdrop's
//! `mouse_area` would also dismiss on clicks *inside* the card. As
//! independent layers, a click only reaches the backdrop where the card
//! (which has no click handler of its own placed on top of it) does not
//! itself claim it.
//!
//! `opaque()` must wrap only the card's own container, not the `center()`
//! around it: `opaque` captures a click when the cursor is over the
//! *layout bounds of whatever it wraps*, and `center()` reports the full
//! available (fill) area as its bounds regardless of the child's actual
//! size. Wrapping `center(...)` in `opaque()` therefore captures clicks
//! anywhere on screen -- including outside the visible card -- and the
//! backdrop underneath never sees them. `center(opaque(card))` keeps the
//! captured area limited to the card's real, tight bounds.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::{center, container, mouse_area, opaque, stack, text};
use iced::{Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};

/// Wraps `base` with `content` shown as a centered, dimmed modal on top.
/// Clicking the dimmed backdrop sends `on_dismiss`.
pub fn over<'a, Message: Clone + 'a>(
    base: Element<'a, Message>,
    content: Element<'a, Message>,
    on_dismiss: Message,
) -> Element<'a, Message> {
    let backdrop = mouse_area(
        container(text(""))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(backdrop_style),
    )
    .on_press(on_dismiss);

    let card = center(opaque(container(content).style(card_style).padding(20)));

    stack![inert(base), opaque(backdrop), card].into()
}

/// A dialog that must be resolved through its own buttons and should not be
/// dismissed by clicking outside (e.g. "unsaved changes" prompts). Same as
/// [`over`], but the backdrop does not react to clicks.
pub fn over_blocking<'a, Message: Clone + 'a>(
    base: Element<'a, Message>,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    let backdrop = container(text(""))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(backdrop_style);

    let card = center(opaque(container(content).style(card_style).padding(20)));

    stack![inert(base), opaque(backdrop), card].into()
}

/// What lies under a modal: drawn and laid out as usual, but it does not
/// get keyboard or input-method events.
///
/// `opaque` keeps the pointer out, but a `stack` hands every event an upper
/// layer did not capture on to the layers below, so a text field of the base
/// that still has the focus would take what is typed into the dialog. The
/// events are not captured here (that would hide them from the app's own
/// key handling, which closes a dialog on Escape): the base just does not
/// see them. Other events (redraws, resizes) pass, so the base keeps
/// animating.
fn inert<'a, Message: 'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    Element::new(Inert { content })
}

struct Inert<'a, Message> {
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, Theme, Renderer> for Inert<'_, Message> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
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
        self.content.as_widget_mut().layout(tree, renderer, limits)
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
            .operate(tree, layout, renderer, operation);
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
        if matches!(event, Event::Keyboard(_) | Event::InputMethod(_)) {
            return;
        }
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
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
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

fn card_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.base.color.into()),
        text_color: Some(palette.background.base.text),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.4),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..Default::default()
    }
}

fn backdrop_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::BLACK.scale_alpha(0.5).into()),
        ..Default::default()
    }
}
