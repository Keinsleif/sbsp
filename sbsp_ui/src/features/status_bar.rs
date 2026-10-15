// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The bottom bar: the mode picker (view / run / edit, limited to what the
//! permissions allow) and the cue count. Ported from `AppFooter.vue`.
//!
//! The update-available button, the asset-processing indicator and the
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

pub fn view<'a>(ui: &UiState, cue_count: usize, sidebar_open: bool) -> Element<'a, Message> {
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

    let count = container(text(fl!(
        "main-foot-bar-cue-count-suffix",
        n = (cue_count as i64)
    )))
    .center_x(Length::Fill);

    let sidebar_toggle = button(icon::toned(
        Icon::DockRight,
        20.0,
        if sidebar_open { Tone::Primary } else { Tone::Muted },
    ))
    .padding(4)
    .style(button::text)
    .on_press(Message::ToggleSidebar);

    row![picker, count, sidebar_toggle]
        .align_y(Alignment::Center)
        .padding(6)
        .into()
}
