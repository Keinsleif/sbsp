// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A modal dialog overlay, replacing PrimeVue's `Dialog` (see the migration
//! plan, section 6).
//!
//! Unlike [`crate::widgets::toast`], this widget does not own any state of
//! its own: "which dialog is open, if any" is application state (see the
//! earlier discussion on global state -- this is exactly the `uiState`
//! case, i.e. `Option<Dialog>` living in the app's own `State`). This
//! module only provides the overlay chrome: a dimmed, click-to-dismiss
//! backdrop and a centered card.
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

use iced::widget::{center, container, mouse_area, opaque, stack, text};
use iced::{Color, Element, Length, Theme};

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

    stack![base, opaque(backdrop), card].into()
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

    stack![base, opaque(backdrop), card].into()
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
