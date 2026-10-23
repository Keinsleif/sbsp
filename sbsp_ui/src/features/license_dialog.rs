// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The license dialog: which edition this copy is, who it is licensed to,
//! and activation from a license file. Ported from `LicenseDialog.vue`.

use chrono::{DateTime, Local};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

use crate::fl;
use crate::widgets::icon::{self, Icon, Tone};

/// What the license says (the host's `LicenseInformation`, reduced to what
/// is shown).
#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub pro: bool,
    pub owner: String,
    pub issue_time: u64,
    pub id: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Read the license again.
    Reload,
    /// Choose a license file to activate.
    Activate,
    Copy(String),
    CloseNotice,
    Close,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dialog {
    pub info: Option<Info>,
    /// A license was just activated: the instructions about the file are
    /// shown.
    pub activated: bool,
}

impl Dialog {
    pub fn new(info: Option<Info>) -> Self {
        Self {
            info,
            activated: false,
        }
    }
}

/// `-` for no time at all; otherwise the local date and time.
pub fn format_issue_time(seconds: u64) -> String {
    i64::try_from(seconds)
        .ok()
        .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
        .map_or_else(
            || "-".to_owned(),
            |time| time.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string(),
        )
}

fn line<'a>(label: String, value: Element<'a, Message>) -> Element<'a, Message> {
    row![text(format!("{label} :")), value].spacing(6).into()
}

pub fn view<'a>(dialog: &'a Dialog) -> Element<'a, Message> {
    if dialog.activated {
        return column![
            text(fl!("dialog-license-success-title")).size(18),
            text(fl!("dialog-license-success-message")),
            row![
                Space::new().width(Length::Fill),
                button(text(fl!("general-close")))
                    .style(button::secondary)
                    .on_press(Message::CloseNotice),
            ],
        ]
        .spacing(12)
        .width(520)
        .into();
    }

    let pro = dialog.info.as_ref().is_some_and(|info| info.pro);
    let edition = text(if pro { "Pro" } else { "Free" }).style(move |theme: &Theme| text::Style {
        color: pro.then(|| theme.extended_palette().success.base.color),
    });
    let owner = dialog.info.as_ref().map_or("-", |info| info.owner.as_str());
    let issued = dialog
        .info
        .as_ref()
        .map_or_else(|| "-".to_owned(), |info| format_issue_time(info.issue_time));
    let id = dialog.info.as_ref().map_or("", |info| info.id.as_str());

    let copy = button(icon::toned(Icon::ContentCopy, 16.0, Tone::Text))
        .style(button::secondary)
        .on_press_maybe((!id.is_empty()).then(|| Message::Copy(id.to_owned())));

    column![
        text(fl!("dialog-license-title")).size(18),
        line(fl!("dialog-license-edition"), edition.into()),
        line(fl!("dialog-license-owner"), text(owner).into()),
        line(fl!("dialog-license-issued-date"), text(issued).into()),
        column![
            text("Id").size(12),
            row![
                container(text(if id.is_empty() { "-" } else { id })).width(Length::Fill),
                copy
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        ]
        .spacing(2),
        row![
            button(icon::toned(Icon::Refresh, 16.0, Tone::Text))
                .style(button::secondary)
                .on_press(Message::Reload),
            Space::new().width(Length::Fill),
            button(text(fl!("dialog-license-activate-license")))
                .style(button::primary)
                .on_press(Message::Activate),
            button(text(fl!("general-close")))
                .style(button::secondary)
                .on_press(Message::Close),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .width(440)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_issue_time_is_a_local_date_and_time() {
        // A day either way of 1971-01-01 whatever the time zone.
        let shown = format_issue_time(365 * 86_400);
        assert!(shown.starts_with("1970-12-3") || shown.starts_with("1971-01-0"), "{shown}");
        assert_eq!(shown.len(), "1971-01-01 00:00:00".len());
        assert_eq!(format_issue_time(u64::MAX), "-");
    }

    #[test]
    fn a_new_dialog_has_no_notice() {
        let dialog = Dialog::new(None);
        assert!(!dialog.activated);
        assert_eq!(dialog.info, None);
    }
}
