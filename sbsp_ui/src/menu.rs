// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A platform-independent description of the application menu.
//!
//! [`MenuSpec`] is built once from plain data and rendered by two different
//! backends:
//!
//! - [`crate::widgets::menu_bar`]: an in-app menu bar drawn by iced itself.
//!   Used on Windows, Linux and Web.
//! - `crate::host::native_menu` (macOS only, `host` feature): a native
//!   global menu bar built with `muda`.
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
        /// the actual key handling stays in iced's keyboard subscription
        /// (see the migration plan, section 5.1).
        shortcut: Option<String>,
        enabled: bool,
        /// `Some(_)` renders a checkmark/indicator; `None` renders a plain item.
        checked: Option<bool>,
    },
    /// A nested menu.
    Submenu { label: String, children: Vec<MenuNode> },
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
}
