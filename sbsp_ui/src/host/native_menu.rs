// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! macOS-only native global menu bar, built from the shared
//! [`crate::menu::MenuSpec`].
//!
//! On Windows and Linux the same [`MenuSpec`] is instead rendered in-app by
//! [`crate::widgets::menu_bar`] (see the migration plan, section 5.1):
//! `muda` cannot attach accelerators to a winit window on Windows, and
//! cannot attach to a winit window at all on Linux (it requires a GTK
//! window). macOS has neither limitation, because `init_for_nsapp` sets the
//! process-wide menu bar without needing a window handle at all.
//!
//! This module only builds and installs the menu and forwards click events;
//! keyboard handling stays in iced's own keyboard subscription.

use muda::{Menu, MenuId as MudaMenuId, MenuItem, PredefinedMenuItem, Submenu};

use crate::menu::{MenuNode, MenuSpec};

/// Builds the native menu from `spec` and installs it as the application's
/// global menu bar.
///
/// Must be called on the main thread, before iced's event loop starts (muda
/// panics if used from any other thread on macOS). In practice this means
/// calling it at the top of the platform `run()` function, before
/// `iced::application(..).run()`.
pub fn install(spec: &MenuSpec) -> Menu {
    let menu = Menu::new();

    for (label, children) in &spec.menus {
        let submenu = Submenu::new(label, true);
        append(&submenu, children);
        menu.append(&submenu)
            .expect("failed to append top-level menu");
    }

    menu.init_for_nsapp();
    menu
}

fn append(target: &Submenu, nodes: &[MenuNode]) {
    for node in nodes {
        match node {
            // Accelerators are intentionally not set here: keyboard
            // handling stays in iced's keyboard subscription (migration
            // plan, section 5.1). muda only needs the id, label and
            // enabled state; `checked` has no simple muda equivalent for a
            // plain `MenuItem` and is left to the in-app renderer.
            MenuNode::Item {
                id, label, enabled, ..
            } => {
                let item = MenuItem::with_id(MudaMenuId::new(id.0), label, *enabled, None);
                target.append(&item).expect("failed to append menu item");
            }
            MenuNode::Submenu { label, children } => {
                let child = Submenu::new(label, true);
                append(&child, children);
                target.append(&child).expect("failed to append submenu");
            }
            MenuNode::Separator => {
                target
                    .append(&PredefinedMenuItem::separator())
                    .expect("failed to append separator");
            }
        }
    }
}

/// A stream of activated menu item ids (the same strings passed to
/// [`crate::menu::MenuId`]), suitable for `Subscription::batch`-ing
/// alongside the rest of the application's subscriptions.
///
/// `muda::MenuEvent::receiver()` returns a blocking `crossbeam_channel`
/// receiver, so it is drained on a dedicated blocking thread
/// (`tokio::task::spawn_blocking`) rather than awaited directly, which would
/// otherwise permanently block one of iced's async worker threads.
pub fn events() -> iced::Subscription<String> {
    iced::Subscription::run(|| {
        iced::stream::channel(32, async |mut output| {
            use iced::futures::SinkExt;

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            tokio::task::spawn_blocking(move || {
                let receiver = muda::MenuEvent::receiver();
                while let Ok(event) = receiver.recv() {
                    if tx.send(event.id.0.clone()).is_err() {
                        break;
                    }
                }
            });

            while let Some(id) = rx.recv().await {
                if output.send(id).await.is_err() {
                    break;
                }
            }
        })
    })
}
