// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Renders a [`MenuSpec`] as an in-app menu bar using `iced_aw`.
//!
//! Used on every target except the macOS host/remote build, which uses a
//! native global menu instead (see `crate::host::native_menu`).

use iced::widget::{button, container, row, text};
use iced::{Alignment, Element, Length, Theme};
use iced_aw::menu::{Item, Menu, MenuBar};
use iced_aw::style::menu_bar;

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

    MenuBar::new(roots).style(menu_bar::primary).into()
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

    Menu::new(items).width(SUBMENU_WIDTH).spacing(4.0)
}

fn top_level_button<'a, Message: Clone + 'a>(label: String) -> Element<'a, Message> {
    button(text(label))
        .padding([4, 10])
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
        .padding([4, 10])
        .style(button::text);

    if enabled {
        btn.on_press_with(on_press).into()
    } else {
        btn.into()
    }
}

fn entry_button_label<'a, Message: Clone + 'a>(label: String) -> Element<'a, Message> {
    let content = row![
        text(label).width(Length::Fill),
        text("›").size(12),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    button(content)
        .width(Length::Fill)
        .padding([4, 10])
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
