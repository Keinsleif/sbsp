// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The update dialog: checks for a newer version, downloads it with its
//! progress, and installs it. Ported from `UpdateDialog.vue`.

use iced::widget::{Space, button, column, progress_bar, row, text};
use iced::{Alignment, Element, Length};

use crate::fl;

#[derive(Debug, Clone)]
pub enum Message {
    Check,
    Install,
    Close,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    Checking,
    UpToDate,
    /// A newer version, with its number.
    Available(String),
    Downloading(u8),
    /// Downloaded; the app is about to restart.
    Restarting,
    Failed(String),
    /// Not installed by an installer, so it cannot update itself.
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dialog {
    pub current: String,
    pub stage: Stage,
}

impl Dialog {
    pub fn new(stage: Stage) -> Self {
        Self {
            current: env!("CARGO_PKG_VERSION").to_owned(),
            stage,
        }
    }

    /// Closing is refused while the update is being installed.
    pub fn busy(&self) -> bool {
        matches!(self.stage, Stage::Downloading(_) | Stage::Restarting)
    }
}

fn line<'a>(label: String, value: String) -> Element<'a, Message> {
    row![text(format!("{label} :")), text(value)].spacing(6).into()
}

pub fn view<'a>(dialog: &'a Dialog) -> Element<'a, Message> {
    let mut body = column![
        text(fl!("dialog-update-title")).size(18),
        line(fl!("dialog-update-current"), dialog.current.clone()),
    ]
    .spacing(12)
    .width(440);

    let mut buttons = row![Space::new().width(Length::Fill)]
        .spacing(8)
        .align_y(Alignment::Center);

    match &dialog.stage {
        Stage::Checking => body = body.push(text(fl!("dialog-update-checking"))),
        Stage::UpToDate => body = body.push(text(fl!("dialog-update-up-to-date"))),
        Stage::Available(version) => {
            body = body.push(line(fl!("dialog-update-latest"), version.clone()));
            buttons = buttons.push(
                button(text(fl!("dialog-update-install")))
                    .style(button::primary)
                    .on_press(Message::Install),
            );
        }
        Stage::Downloading(percent) => {
            body = body
                .push(text(fl!("dialog-update-downloading")))
                .push(progress_bar(0.0..=100.0, f32::from(*percent)));
        }
        Stage::Restarting => body = body.push(text(fl!("dialog-update-restarting"))),
        Stage::Failed(error) => {
            body = body
                .push(text(fl!("dialog-update-failed")))
                .push(text(error.clone()).size(12));
            buttons = buttons.push(
                button(text(fl!("dialog-update-retry")))
                    .style(button::primary)
                    .on_press(Message::Check),
            );
        }
        Stage::Unavailable => body = body.push(text(fl!("dialog-update-unavailable"))),
    }

    buttons = buttons.push(
        button(text(fl!("general-close")))
            .style(button::secondary)
            .on_press_maybe((!dialog.busy()).then_some(Message::Close)),
    );
    body.push(buttons).into()
}
