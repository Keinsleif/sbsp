// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! macOS-only native global menu bar, built from the shared
//! [`crate::menu::MenuSpec`].
//!
//! On Windows and Linux the same [`MenuSpec`] is instead rendered in-app by
//! [`crate::widgets::menu_bar`]: `muda` cannot attach accelerators to a winit
//! window on Windows, and cannot attach to a winit window at all on Linux (it
//! requires a GTK window). macOS has neither limitation, because
//! `init_for_nsapp` sets the process-wide menu bar without needing a window
//! handle at all.
//!
//! The menu is installed once, after the first window has opened, and from
//! then on only the state of its items (enabled, checked, label) follows the
//! spec: [`sync`]. It cannot be installed earlier: winit puts its own default
//! menu in place while the application finishes launching, inside `run`,
//! replacing whatever was set before. That default menu is also what gave
//! the application its standard items, so [`install`] makes the application
//! menu itself (About, Services, Hide, Quit).
//!
//! This module only builds and installs the menu and forwards click events;
//! keyboard handling stays in iced's own keyboard subscription.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::LazyLock;

use muda::{CheckMenuItem, Menu, MenuId as MudaMenuId, MenuItem, PredefinedMenuItem, Submenu};
use tokio::sync::broadcast;

use crate::menu::{MenuId, MenuNode, MenuSpec};
/// The items macOS keeps in the application menu. They stay items of the
/// spec, so that they go through the app's own handling (the about dialog,
/// and quitting with its confirmations) rather than the system's.
const APP_MENU_IDS: [&str; 2] = ["help.about", "file.quit"];

enum Entry {
    Plain(MenuItem),
    Check(CheckMenuItem),
}

struct Installed {
    /// Kept for as long as the application runs.
    _menu: Menu,
    entries: HashMap<&'static str, Entry>,
}

thread_local! {
    // muda's items are not `Send`: the menu lives on the main thread, where
    // it is installed and updated.
    static INSTALLED: RefCell<Option<Installed>> = const { RefCell::new(None) };
}

static MENU_EVENT_BUS: LazyLock<broadcast::Sender<String>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(32);
    let bus_tx = tx.clone();

    std::thread::spawn(move || {
        let receiver = muda::MenuEvent::receiver();
        while let Ok(event) = receiver.recv() {
            let _ = bus_tx.send(event.id.0);
        }
    });

    tx
});

/// Builds the native menu from `spec` and installs it as the application's
/// global menu bar.
///
/// Must be called on the main thread, after the window has opened (see the
/// module documentation). The ids of `spec` and the structure of its menus
/// are fixed from here on; [`sync`] follows the state of its items.
pub fn install(spec: &MenuSpec) {
    let already_installed = INSTALLED.with_borrow(|installed| installed.is_some());
    if already_installed {
        return;
    }

    let menu = Menu::new();
    let mut entries = HashMap::new();

    // macOS shows the first menu under the application's name.
    let app = Submenu::new("", true);
    append_found(&app, spec, "help.about", &mut entries);
    append_separator(&app);
    append_predefined(&app, &PredefinedMenuItem::services(None));
    append_separator(&app);
    append_predefined(&app, &PredefinedMenuItem::hide(None));
    append_predefined(&app, &PredefinedMenuItem::hide_others(None));
    append_predefined(&app, &PredefinedMenuItem::show_all(None));
    append_separator(&app);
    append_found(&app, spec, "file.quit", &mut entries);
    menu.append(&app).expect("failed to append the app menu");

    for (label, children) in &spec.without(&APP_MENU_IDS).menus {
        let submenu = Submenu::new(label, true);
        append(&submenu, children, &mut entries);
        menu.append(&submenu)
            .expect("failed to append top-level menu");
    }

    menu.init_for_nsapp();
    INSTALLED.with_borrow_mut(|installed| {
        *installed = Some(Installed {
            _menu: menu,
            entries,
        });
   });
}

/// Brings the items' enabled state, checkmark and label up to date with
/// `spec`. Items that were not there when the menu was installed are
/// ignored. Does nothing before [`install`].
pub fn sync(spec: &MenuSpec) {
    INSTALLED.with_borrow(|installed| {
        let Some(installed) = installed else {
            return;
        };
        for (_, nodes) in &spec.menus {
            sync_nodes(nodes, &installed.entries);
        }
    });
}

/// The menu id behind the string of a click event.
pub fn resolve(raw: &str) -> Option<MenuId> {
    INSTALLED.with_borrow(|installed| {
        installed
            .as_ref()?
            .entries
            .get_key_value(raw)
            .map(|(id, _)| MenuId(id))
    })
}

fn sync_nodes(nodes: &[MenuNode], entries: &HashMap<&'static str, Entry>) {
    for node in nodes {
        match node {
            MenuNode::Item {
                id,
                label,
                enabled,
                checked,
                ..
            } => match entries.get(id.0) {
                Some(Entry::Plain(item)) => {
                    if item.is_enabled() != *enabled {
                        item.set_enabled(*enabled);
                    }
                    if item.text() != *label {
                        item.set_text(label.replace('&', "&&"));
                    }
                }
                Some(Entry::Check(item)) => {
                    if item.is_enabled() != *enabled {
                        item.set_enabled(*enabled);
                    }
                    if item.text() != *label {
                        item.set_text(label);
                    }
                    let checked = checked.unwrap_or(false);
                    if item.is_checked() != checked {
                        item.set_checked(checked);
                    }
                }
                None => {}
            },
            MenuNode::Submenu { children, .. } => sync_nodes(children, entries),
            MenuNode::Separator => {}
        }
    }
}

fn find<'a>(nodes: &'a [MenuNode], wanted: &str) -> Option<&'a MenuNode> {
    nodes.iter().find_map(|node| match node {
        MenuNode::Item { id, .. } if id.0 == wanted => Some(node),
        MenuNode::Submenu { children, .. } => find(children, wanted),
        _ => None,
    })
}

fn append_found(
    target: &Submenu,
    spec: &MenuSpec,
    wanted: &str,
    entries: &mut HashMap<&'static str, Entry>,
) {
    let found = spec.menus.iter().find_map(|(_, nodes)| find(nodes, wanted));
    if let Some(node) = found {
        append(target, std::slice::from_ref(node), entries);
    }
}

fn append_separator(target: &Submenu) {
    append_predefined(target, &PredefinedMenuItem::separator());
}

fn append_predefined(target: &Submenu, item: &PredefinedMenuItem) {
    target
        .append(item)
        .expect("failed to append a standard item");
}

fn append(target: &Submenu, nodes: &[MenuNode], entries: &mut HashMap<&'static str, Entry>) {
    for node in nodes {
        match node {
            // Accelerators are intentionally not set here: keyboard
            // handling stays in iced's keyboard subscription.
            MenuNode::Item {
                id,
                label,
                enabled,
                checked,
                ..
            } => {
                let muda_id = MudaMenuId::new(id.0);
                let escaped_label = label.replace('&', "&&");
                match checked {
                    Some(checked) => {
                        let item = CheckMenuItem::with_id(muda_id, escaped_label, *enabled, *checked, None);
                        target.append(&item).expect("failed to append menu item");
                        entries.insert(id.0, Entry::Check(item));
                    }
                    None => {
                        let item = MenuItem::with_id(muda_id, escaped_label, *enabled, None);
                        target.append(&item).expect("failed to append menu item");
                        entries.insert(id.0, Entry::Plain(item));
                    }
                }
            }
            MenuNode::Submenu { label, children } => {
                let escaped_label = label.replace('&', "&&");
                let child = Submenu::new(escaped_label, true);
                append(&child, children, entries);
                target.append(&child).expect("failed to append submenu");
            }
            MenuNode::Separator => append_separator(target),
        }
    }
}

/// A stream of activated menu item ids (the strings given to
/// [`crate::menu::MenuId`]), suitable for `Subscription::batch`-ing
/// alongside the rest of the application's subscriptions. [`resolve`] turns
/// one back into a [`MenuId`], on the main thread.
///
/// `muda::MenuEvent::receiver()` returns a blocking `crossbeam_channel`
/// receiver. To prevent race conditions when iced drops and restarts this
/// subscription, events are forwarded to a global `broadcast` channel by a
/// single dedicated thread spawned on startup, rather than spawning transient
/// blocking tasks.
pub fn events() -> iced::Subscription<String> {
    iced::Subscription::run(|| {
        iced::stream::channel(32, async |mut output| {
            use iced::futures::SinkExt;

            let mut rx = MENU_EVENT_BUS.subscribe();

            while let Ok(id) = rx.recv().await {
                if output.send(id).await.is_err() {
                    break;
                }
            }
        })
    })
}
