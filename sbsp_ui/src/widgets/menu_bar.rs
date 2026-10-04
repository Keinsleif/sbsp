// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Renders a [`MenuSpec`] as an in-app menu bar using `iced_aw`.
//!
//! Used on every target except the macOS host/remote build, which uses a
//! native global menu instead (see `crate::host::native_menu`).

use iced::widget::{button, container, row, text};
use iced::{Alignment, Border, Element, Length, Theme};
use iced_aw::menu::{Item, Menu, MenuBar};
use iced_aw::style::Status;
use iced_aw::style::menu_bar::Style;

use crate::menu::{MenuId, MenuNode, MenuSpec};

const SUBMENU_WIDTH: f32 = 220.0;

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
        // iced_aw defaults both of these to false, which reads as broken
        // against a native menu bar: clicking the already-open root again
        // (or an item inside a submenu) does nothing, since nothing ever
        // schedules a close. True matches ordinary menu-bar behavior:
        // picking an item closes the whole bar, and so does re-clicking
        // the open root.
        .close_on_item_click(true)
        .close_on_background_click(true)
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

    Menu::new(items).width(SUBMENU_WIDTH).spacing(1.0)
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
        text(format!("{prefix}{label}")).width(Length::Fill),
        text(shortcut.unwrap_or_default()).size(12),
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
        btn.into()
    }
}

fn entry_button_label<'a, Message: Clone + 'a>(label: String) -> Element<'a, Message> {
    let content = row![text(label).width(Length::Fill), text("›").size(12),]
        .spacing(12)
        .align_y(Alignment::Center);

    button(content)
        .width(Length::Fill)
        .padding([3, 10])
        .style(button::text)
        .into()
}

fn separator<'a, Message: Clone + 'a>() -> Element<'a, Message> {
    container(text(""))
        .height(1)
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            ..Default::default()
        })
        .into()
}
