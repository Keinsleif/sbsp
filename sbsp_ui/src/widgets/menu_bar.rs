// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Renders a [`MenuSpec`] as an in-app menu bar using `iced_aw`.
//!
//! Used on every target except the macOS host/remote build, which uses a
//! native global menu instead (see `crate::host::native_menu`).

use iced::advanced::text::{self as advanced_text, Paragraph as _};
use iced::widget::{button, container, mouse_area, row, text};
use iced::{Alignment, Border, Element, Length, Theme, mouse};
use iced_aw::menu::{Item, Menu, MenuBar};
use iced_aw::style::Status;
use iced_aw::style::menu_bar::Style;

use crate::menu::{MenuId, MenuNode, MenuSpec};

/// The narrowest a dropdown gets, however short its entries are.
const MIN_MENU_WIDTH: f32 = 160.0;
/// Space around an entry's text: the button's padding on both sides, the
/// gap between the label and the shortcut, and some room for the estimate
/// being rough.
const ENTRY_EXTRA: f32 = 2.0 * 10.0 + 12.0 + 16.0;
const SHORTCUT_SIZE: f32 = 12.0;

/// Builds the menu bar widget. `on_select` maps a clicked item's [`MenuId`]
/// to the application's `Message` type.
///
/// Every label and shortcut string is cloned out of `spec` up front, so the
/// returned widget does not borrow from it.
pub fn menu_bar<'a, Message, F>(spec: &MenuSpec, on_select: F) -> Element<'a, Message>
where
    Message: Clone + 'a,
    F: Fn(MenuId) -> Message + Clone + 'a,
{
    let roots = spec
        .menus
        .iter()
        .map(|(label, children)| {
            let submenu = build_menu(children, on_select.clone());
            Item::with_menu(top_level_button(label.clone()), submenu)
        })
        .collect::<Vec<_>>();

    MenuBar::new(roots)
        .style(native_style)
        .close_on_item_click_global(true)
        .close_on_background_click_global(true)
        .into()
}

/// Closer to a native menu bar's flat look than `iced_aw`'s own
/// `menu_bar::primary` (which defaults to an 8px corner radius and a drop
/// shadow on the dropdown -- noticeably more rounded/padded than any
/// desktop OS's actual menu).
fn native_style(theme: &Theme, _status: Status) -> Style {
    let palette = theme.extended_palette();

    Style {
        bar_background: palette.background.base.color.into(),
        bar_border: Border::default(),
        bar_shadow: iced::Shadow::default(),
        menu_background: palette.background.base.color.into(),
        menu_border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 2.0.into(),
        },
        menu_shadow: iced::Shadow {
            color: iced::Color::BLACK.scale_alpha(0.25),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        },
        path: palette.primary.weak.color.into(),
        path_border: Border {
            radius: 2.0.into(),
            ..Default::default()
        },
    }
}

/// How wide the dropdown for `nodes` should be: the widest entry, so that
/// no label wraps onto a second line. Text is not measured (the menu is
/// built before any layout), so each character is given a width by kind,
/// a little generous.
fn menu_width(nodes: &[MenuNode]) -> f32 {
    let text_size = crate::theme::fonts::TEXT_SIZE;
    let widest = nodes
        .iter()
        .map(|node| match node {
            MenuNode::Item {
                label,
                shortcut,
                checked,
                ..
            } => {
                let mark = if checked.is_some() { 2.0 } else { 0.0 };
                text_width(label, text_size)
                    + mark * text_size
                    + shortcut
                        .as_deref()
                        .map_or(0.0, |s| text_width(s, SHORTCUT_SIZE))
            }
            MenuNode::Submenu { label, .. } => text_width(label, text_size) + SHORTCUT_SIZE,
            MenuNode::Separator => 0.0,
        })
        .fold(0.0_f32, f32::max);

    (widest + ENTRY_EXTRA).max(MIN_MENU_WIDTH)
}

/// How wide `text` is at `size` in the UI font. Measured with the text
/// engine (so it follows the font actually in use), remembered per string
/// since the menu is rebuilt with every view. Falls back to a rough
/// estimate when nothing can be measured (no fonts, as in headless tests).
fn text_width(text: &str, size: f32) -> f32 {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static MEASURED: RefCell<HashMap<(String, u32), f32>> = RefCell::new(HashMap::new());
    }

    let key = (text.to_owned(), size.to_bits());
    if let Some(width) = MEASURED.with(|cache| cache.borrow().get(&key).copied()) {
        return width;
    }

    let paragraph = <iced::Renderer as advanced_text::Renderer>::Paragraph::with_text(
        advanced_text::Text {
            content: text,
            bounds: iced::Size::INFINITE,
            size: iced::Pixels(size),
            line_height: advanced_text::LineHeight::default(),
            font: crate::theme::fonts::UI,
            align_x: advanced_text::Alignment::Default,
            align_y: iced::alignment::Vertical::Top,
            shaping: advanced_text::Shaping::Advanced,
            wrapping: advanced_text::Wrapping::None,
        },
    );
    let measured = paragraph.min_bounds().width;
    let width = if measured > 0.0 {
        measured
    } else {
        estimated_width(text, size)
    };
    MEASURED.with(|cache| cache.borrow_mut().insert(key, width));
    width
}

/// A rough width of `text` at `size`: full-width (CJK) characters take a
/// whole em, anything else about half of one.
fn estimated_width(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| if is_wide(c) { 1.0 } else { 0.55 })
        .sum::<f32>()
        * size
}

fn is_wide(c: char) -> bool {
    matches!(c,
        '\u{1100}'..='\u{115F}'
        | '\u{2E80}'..='\u{A4CF}'
        | '\u{AC00}'..='\u{D7A3}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{FE30}'..='\u{FE6F}'
        | '\u{FF00}'..='\u{FF60}'
        | '\u{FFE0}'..='\u{FFE6}')
}

fn build_menu<'a, Message, F>(
    nodes: &[MenuNode],
    on_select: F,
) -> Menu<'a, Message, Theme, iced::Renderer>
where
    Message: Clone + 'a,
    F: Fn(MenuId) -> Message + Clone + 'a,
{
    let items = nodes
        .iter()
        .map(|node| match node {
            MenuNode::Item {
                id,
                label,
                shortcut,
                enabled,
                checked,
            } => {
                let id = *id;
                let on_select = on_select.clone();
                Item::new(entry_button(
                    label.clone(),
                    shortcut.clone(),
                    *checked,
                    *enabled,
                    move || on_select(id),
                ))
            }
            MenuNode::Submenu { label, children } => {
                let submenu = build_menu(children, on_select.clone());
                Item::with_menu(entry_button_label(label.clone()), submenu)
            }
            MenuNode::Separator => Item::new(separator()),
        })
        .collect::<Vec<_>>();

    Menu::new(items)
        .width(menu_width(nodes))
        .spacing(0.0)
}

fn top_level_button<'a, Message: Clone + 'a>(label: String) -> Element<'a, Message> {
    button(text(label))
        .padding([3, 8])
        .style(button::text)
        .into()
}

fn entry_button<'a, Message: Clone + 'a>(
    label: String,
    shortcut: Option<String>,
    checked: Option<bool>,
    enabled: bool,
    on_press: impl Fn() -> Message + 'a,
) -> Element<'a, Message> {
    let prefix = match checked {
        Some(true) => "✓ ",
        _ => "",
    };

    let content = row![
        text(format!("{prefix}{label}"))
            .width(Length::Fill)
            .wrapping(text::Wrapping::None),
        text(shortcut.unwrap_or_default())
            .size(SHORTCUT_SIZE)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let btn = button(content)
        .width(Length::Fill)
        .padding([3, 10])
        .style(button::text);

    if enabled {
        btn.on_press_with(on_press).into()
    } else {
        inert(btn.into())
    }
}

fn entry_button_label<'a, Message: Clone + 'a>(label: String) -> Element<'a, Message> {
    let content = row![
        text(label).width(Length::Fill).wrapping(text::Wrapping::None),
        text("›").size(SHORTCUT_SIZE),
    ]
        .spacing(12)
        .align_y(Alignment::Center);

    inert(
        button(content)
            .width(Length::Fill)
            .padding([3, 10])
            .style(button::text)
            .into(),
    )
}

/// The dropdown is an overlay, and iced hides the cursor from the widgets
/// below it only where the overlay reports a mouse interaction. A button
/// without `on_press` reports none, so the buttons and fields under a
/// disabled item, a submenu entry or a separator would still react to the
/// pointer.
fn inert<'a, Message: Clone + 'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    mouse_area(content)
        .interaction(mouse::Interaction::Idle)
        .into()
}

fn separator<'a, Message: Clone + 'a>() -> Element<'a, Message> {
    inert(
        container(text(""))
            .height(1)
            .width(Length::Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                ..Default::default()
            })
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_text_is_wider_than_the_same_number_of_latin_letters() {
        assert!(estimated_width("キュー番号", 14.0) > estimated_width("Cues!", 14.0));
    }

    #[test]
    fn a_menu_is_as_wide_as_its_longest_entry() {
        let short = [MenuNode::item("a", "Copy")];
        let long = [
            MenuNode::item("a", "Copy"),
            MenuNode::item("b", "Renumber selected cues").with_shortcut("Ctrl+R"),
        ];
        assert_eq!(menu_width(&short), MIN_MENU_WIDTH);
        assert!(menu_width(&long) > menu_width(&short));
        // Wide enough for the label and the shortcut next to each other.
        assert!(
            menu_width(&long)
                > text_width("Renumber selected cues", crate::theme::fonts::TEXT_SIZE)
                    + text_width("Ctrl+R", SHORTCUT_SIZE)
        );
    }

    #[test]
    fn a_checkmark_and_a_submenu_arrow_take_room_too() {
        let plain = [MenuNode::item("a", "Some fairly long entry label here")];
        let checked = [MenuNode::item("a", "Some fairly long entry label here").checked(true)];
        assert!(menu_width(&checked) > menu_width(&plain));
    }
}
