// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The bottom bar: the mode picker (view / run / edit, limited to what the
//! permissions allow) and the cue count. Ported from `AppFooter.vue`.
//!
//! The asset-processing indicator and the
//! server/settings/dock buttons that bar also held come with the features
//! they belong to (Phase 6/7).

use std::fmt;

use iced::widget::{button, container, pick_list, row, text};
use iced::{Alignment, Element, Length};

use crate::fl;
use crate::widgets::icon::{self, Icon, Tone};
use crate::state::ui::{Mode, UiState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    ModeSelected(Mode),
    ToggleSidebar,
    ToggleBottomTab,
    OpenSettings,
    OpenServerPanel,
    OpenUpdate,
}

/// A mode with its translated label, which is what `pick_list` shows.
/// Equality is on the mode alone, so the selection survives a language
/// switch (the label changes, the mode does not).
#[derive(Clone)]
struct ModeOption {
    mode: Mode,
    label: String,
}

impl PartialEq for ModeOption {
    fn eq(&self, other: &Self) -> bool {
        self.mode == other.mode
    }
}

impl fmt::Display for ModeOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn mode_label(mode: Mode) -> String {
    match mode {
        Mode::View => fl!("main-foot-bar-modes-view"),
        Mode::Run => fl!("main-foot-bar-modes-run"),
        Mode::Edit => fl!("main-foot-bar-modes-edit"),
    }
}

/// `has_server`: this build can run the API server (the host).
pub fn view<'a>(
    ui: &UiState,
    cue_count: usize,
    has_server: bool,
    update_available: bool,
) -> Element<'a, Message> {
    let options: Vec<ModeOption> = ui
        .available_modes()
        .into_iter()
        .map(|mode| ModeOption {
            mode,
            label: mode_label(mode),
        })
        .collect();
    let selected = options.iter().find(|option| option.mode == ui.mode).cloned();

    let picker = pick_list(options, selected, |option| Message::ModeSelected(option.mode))
        .width(120);

    // Beside the mode picker, as in the original; it stays while an update
    // is waiting and opens the update dialog.
    let update = update_available.then(|| {
        button(
            row![icon::icon(Icon::Refresh, 16.0), text(fl!("dialog-update-updates-available"))]
                .spacing(4)
                .align_y(Alignment::Center),
        )
        .padding([3, 8])
        .style(button::secondary)
        .on_press(Message::OpenUpdate)
    });

    let count = container(text(fl!(
        "main-foot-bar-cue-count-suffix",
        n = (cue_count as i64)
    )))
    .center_x(Length::Fill);

    let toggle = |glyph: Icon, open: bool, message: Message| {
        button(icon::toned(
            glyph,
            20.0,
            if open { Tone::Primary } else { Tone::Muted },
        ))
        .padding(4)
        .style(button::text)
        .on_press(message)
    };
    let btn = |glyph: Icon, message: Message| {
        button(icon::icon(glyph, 20.0))
        .padding(4)
        .style(button::text)
        .on_press(message)
    };
    let bottom_toggle = toggle(Icon::DockBottom, ui.is_bottom_tab_open, Message::ToggleBottomTab);
    let sidebar_toggle = toggle(Icon::DockRight, ui.is_right_sidebar_open, Message::ToggleSidebar);

    let settings = btn(Icon::Cog, Message::OpenSettings);

    let mut bar = row![picker].align_y(Alignment::Center);
    if let Some(update) = update {
        bar = bar.push(update);
    }
    let mut bar = bar.push(count).push(bottom_toggle).push(sidebar_toggle);
    if has_server {
        bar = bar.push(btn(Icon::Server, Message::OpenServerPanel));
    }
    bar.push(settings).padding(6).into()
}
