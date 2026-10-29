// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! New / open / save / export and the questions that come before them.
//! Ported from `window-menu.ts` and `MainView.vue`.
//!
//! This module holds the decisions (what to ask, in what order) and the
//! confirmation dialogs. Doing the work (file dialogs, the backend calls)
//! is the host's job, see `host::files`.

use iced::widget::{button, column, row, text};
use iced::Element;

use crate::fl;

/// A file menu entry or its hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    New,
    Open,
    Save,
    SaveAs,
    Export,
}

/// What the user asked for that replaces or ends the current show, and so
/// may have to wait for a question or a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum After {
    New,
    Open,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirm {
    /// "Active cues will be stopped. Continue?"
    StopActive(After),
    /// "Save the changes?"
    SaveChanges(After),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// OK / Save.
    Yes,
    /// Don't save.
    No,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    AskStop,
    AskSave,
    Go,
}

/// The first thing to do about `after`. Replacing the show or closing the
/// app stops whatever is active, which is asked about first.
pub fn first_step(has_active_cues: bool, modified: bool) -> Step {
    if has_active_cues {
        Step::AskStop
    } else {
        after_stop_confirmed(modified)
    }
}

/// What follows a "yes, stop them".
pub fn after_stop_confirmed(modified: bool) -> Step {
    if modified { Step::AskSave } else { Step::Go }
}

pub fn view<'a>(confirm: Confirm) -> Element<'a, Answer> {
    let buttons = match confirm {
        Confirm::StopActive(_) => row![
            button(text(fl!("general-cancel")))
                .style(button::secondary)
                .on_press(Answer::Cancel),
            button(text(fl!("general-confirm"))).on_press(Answer::Yes),
        ],
        Confirm::SaveChanges(_) => row![
            button(text(fl!("dialog-save-confirm-cancel")))
                .style(button::secondary)
                .on_press(Answer::Cancel),
            button(text(fl!("dialog-save-confirm-dont-save")))
                .style(button::danger)
                .on_press(Answer::No),
            button(text(fl!("dialog-save-confirm-save"))).on_press(Answer::Yes),
        ],
    }
    .spacing(8);

    let message = match confirm {
        Confirm::StopActive(After::Quit) => fl!("dialog-quit-while-active-content"),
        Confirm::StopActive(_) => fl!("menu-file-warning-before-load-and-reset"),
        Confirm::SaveChanges(_) => fl!("dialog-save-confirm-content"),
    };

    column![
        text(fl!("dialog-message-confirmation")).size(18),
        text(message).width(320),
        buttons,
    ]
    .spacing(12)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_cues_are_asked_about_first() {
        assert_eq!(first_step(true, true), Step::AskStop);
        assert_eq!(first_step(true, false), Step::AskStop);
    }

    #[test]
    fn then_about_unsaved_changes() {
        assert_eq!(first_step(false, true), Step::AskSave);
        assert_eq!(first_step(false, false), Step::Go);
        assert_eq!(after_stop_confirmed(true), Step::AskSave);
        assert_eq!(after_stop_confirmed(false), Step::Go);
    }
}
