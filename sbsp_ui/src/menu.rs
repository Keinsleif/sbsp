// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A platform-independent description of the application menu.
//!
//! [`MenuSpec`] is built once from plain data and rendered by two different
//! backends:
//!
//! - [`crate::widgets::menu_bar`]: an in-app menu bar drawn by iced itself.
//!   Used on Windows, Linux and Web.
//! - `crate::platform::native_menu` (macOS only):a native global menu bar
//!   built with `muda`.
//!
//! Both backends only need to read this tree; neither needs to know about
//! the other.

/// A stable identifier for a menu action.
///
/// Using a `&'static str` (rather than an enum per menu) keeps `MenuSpec`
/// independent from the application's `Message` type. The owner of the menu
/// maps ids back to `Message` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuId(pub &'static str);

/// One entry of a menu.
#[derive(Debug, Clone)]
pub enum MenuNode {
    /// A clickable action.
    Item {
        id: MenuId,
        label: String,
        /// Human-readable shortcut hint (e.g. `"Ctrl+S"`). Display only;
        /// the actual key handling stays in iced's keyboard subscription.
        shortcut: Option<String>,
        enabled: bool,
        /// `Some(_)` renders a checkmark/indicator; `None` renders a plain item.
        checked: Option<bool>,
    },
    /// A nested menu.
    Submenu {
        label: String,
        children: Vec<MenuNode>,
    },
    /// A visual divider.
    Separator,
}

impl MenuNode {
    pub fn item(id: &'static str, label: impl Into<String>) -> Self {
        Self::Item {
            id: MenuId(id),
            label: label.into(),
            shortcut: None,
            enabled: true,
            checked: None,
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        if let Self::Item { shortcut: s, .. } = &mut self {
            *s = Some(shortcut.into());
        }
        self
    }

    /// [`MenuNode::with_shortcut`] for something that may not be bound.
    pub fn with_shortcut_opt(self, shortcut: Option<String>) -> Self {
        match shortcut {
            Some(shortcut) => self.with_shortcut(shortcut),
            None => self,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        if let Self::Item { enabled: e, .. } = &mut self {
            *e = enabled;
        }
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        if let Self::Item { checked: c, .. } = &mut self {
            *c = Some(checked);
        }
        self
    }

    pub fn submenu(label: impl Into<String>, children: Vec<MenuNode>) -> Self {
        Self::Submenu {
            label: label.into(),
            children,
        }
    }

    pub fn separator() -> Self {
        Self::Separator
    }
}

/// A full menu bar: an ordered list of top-level menus, each with its own
/// entries.
#[derive(Debug, Clone, Default)]
pub struct MenuSpec {
    pub menus: Vec<(String, Vec<MenuNode>)>,
}

impl MenuSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn menu(mut self, label: impl Into<String>, items: Vec<MenuNode>) -> Self {
        self.menus.push((label.into(), items));
        self
    }

    /// The same menus without the items with these ids, for a platform that
    /// shows them elsewhere. A separator left at the start or end of a
    /// menu, or next to another, is dropped, and so is a menu left empty.
    pub fn without(&self, ids: &[&str]) -> MenuSpec {
        MenuSpec {
            menus: self
                .menus
                .iter()
                .map(|(label, nodes)| (label.clone(), strip(nodes, ids)))
                .filter(|(_, nodes)| !nodes.is_empty())
                .collect(),
        }
    }


    /// [`MenuSpec::menu`], but only adds the menu when `condition` holds
    /// (e.g. a host-only menu in a build shared with remote/web).
    pub fn menu_if(self, condition: bool, label: impl Into<String>, items: Vec<MenuNode>) -> Self {
        if condition {
            self.menu(label, items)
        } else {
            self
        }
    }
}

fn strip(nodes: &[MenuNode], ids: &[&str]) -> Vec<MenuNode> {
    let mut kept: Vec<MenuNode> = Vec::new();
    for node in nodes {
        match node {
            MenuNode::Item { id, .. } if ids.contains(&id.0) => {}
            MenuNode::Submenu { label, children } => {
                let children = strip(children, ids);
                if !children.is_empty() {
                    kept.push(MenuNode::Submenu {
                        label: label.clone(),
                        children,
                    });
                }
            }
            MenuNode::Separator => {
                if !matches!(kept.last(), None | Some(MenuNode::Separator)) {
                    kept.push(MenuNode::Separator);
                }
            }
            other => kept.push(other.clone()),
        }
    }
    while matches!(kept.last(), Some(MenuNode::Separator)) {
        kept.pop();
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(nodes: &[MenuNode]) -> Vec<&str> {
        nodes
            .iter()
            .map(|node| match node {
                MenuNode::Item { id, .. } => id.0,
                MenuNode::Submenu { .. } => "<submenu>",
                MenuNode::Separator => "-",
            })
            .collect()
    }

    #[test]
    fn removed_items_take_their_dangling_separators_with_them() {
        let spec = MenuSpec::new()
            .menu(
                "File",
                vec![
                    MenuNode::item("file.open", "Open"),
                    MenuNode::separator(),
                    MenuNode::item("file.quit", "Quit"),
                ],
            )
            .menu(
                "Help",
                vec![
                    MenuNode::item("help.credits", "Credits"),
                    MenuNode::separator(),
                    MenuNode::item("help.about", "About"),
                ],
            );

        let trimmed = spec.without(&["file.quit", "help.about"]);

        assert_eq!(ids(&trimmed.menus[0].1), ["file.open"]);
        assert_eq!(ids(&trimmed.menus[1].1), ["help.credits"]);
    }

    #[test]
    fn a_separator_left_at_the_start_or_doubled_is_dropped() {
        let spec = MenuSpec::new().menu(
            "Edit",
            vec![
                MenuNode::item("a", "A"),
                MenuNode::separator(),
                MenuNode::item("gone", "Gone"),
                MenuNode::separator(),
                MenuNode::item("b", "B"),
            ],
        );

        let trimmed = spec.without(&["gone"]);

        assert_eq!(ids(&trimmed.menus[0].1), ["a", "-", "b"]);
    }

    #[test]
    fn an_emptied_menu_or_submenu_is_dropped() {
        let spec = MenuSpec::new()
            .menu("Only", vec![MenuNode::item("gone", "Gone")])
            .menu(
                "Tools",
                vec![
                    MenuNode::item("keep", "Keep"),
                    MenuNode::submenu("Inner", vec![MenuNode::item("gone", "Gone")]),
                ],
            );

        let trimmed = spec.without(&["gone"]);

        assert_eq!(trimmed.menus.len(), 1);
        assert_eq!(trimmed.menus[0].0, "Tools");
        assert_eq!(ids(&trimmed.menus[0].1), ["keep"]);
    }

    #[test]
    fn menu_if_only_adds_the_menu_when_the_condition_holds() {
        let spec = MenuSpec::new()
            .menu_if(false, "Skipped", Vec::new())
            .menu_if(true, "Added", Vec::new());

        assert_eq!(spec.menus.len(), 1);
        assert_eq!(spec.menus[0].0, "Added");
    }
}
