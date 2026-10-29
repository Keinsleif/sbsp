// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The "renumber cues" dialog. Ported from `RenumberDialog.vue`.
//!
//! The selected cues get the numbers `start`, `start + increment`, ...,
//! each wrapped in the prefix and suffix. The dialog only collects the
//! four values; the app sends them to the backend.

use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length};

use crate::domain::field::{Commit, NumberSpec, commit_number, step_number};
use crate::fl;
use crate::widgets::field::{self, Drafts, FieldId, Outcome, text_field};
use crate::widgets::inputs::{NumberInput, labeled, number_input};

const START: FieldId = FieldId("renumber.start");
const INCREMENT: FieldId = FieldId("renumber.increment");
const PREFIX: FieldId = FieldId("renumber.prefix");
const SUFFIX: FieldId = FieldId("renumber.suffix");

pub fn owns(id: FieldId) -> bool {
    [START, INCREMENT, PREFIX, SUFFIX].contains(&id)
}

fn start_spec() -> NumberSpec {
    NumberSpec {
        min: Some(0.0),
        ..NumberSpec::default()
    }
}

fn increment_spec() -> NumberSpec {
    NumberSpec {
        min: Some(1.0),
        ..NumberSpec::default()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Field(field::Message),
    Run,
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dialog {
    start: f64,
    increment: f64,
    prefix: String,
    suffix: String,
}

impl Default for Dialog {
    fn default() -> Self {
        Self {
            start: 1.0,
            increment: 1.0,
            prefix: String::new(),
            suffix: String::new(),
        }
    }
}

/// What to send to the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub start: usize,
    pub increment: usize,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

impl Dialog {
    /// A finished edit or a step button, if it belongs to this dialog.
    pub fn apply(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Commit(commit) if commit.field == START => {
                if let Commit::Value(value) =
                    commit_number(&commit.text, Some(self.start), &start_spec())
                {
                    self.start = value;
                }
            }
            Outcome::Commit(commit) if commit.field == INCREMENT => {
                if let Commit::Value(value) =
                    commit_number(&commit.text, Some(self.increment), &increment_spec())
                {
                    self.increment = value;
                }
            }
            Outcome::Commit(commit) if commit.field == PREFIX => {
                self.prefix = commit.text.clone();
            }
            Outcome::Commit(commit) if commit.field == SUFFIX => {
                self.suffix = commit.text.clone();
            }
            Outcome::Step(id, direction) if *id == START => {
                self.start = step_number(Some(self.start), &start_spec(), *direction);
            }
            Outcome::Step(id, direction) if *id == INCREMENT => {
                self.increment = step_number(Some(self.increment), &increment_spec(), *direction);
            }
            _ => {}
        }
    }

    /// `None` while the values cannot be used (the original's guards:
    /// finite numbers and an increment of at least 1). Prefix and suffix
    /// are trimmed, and empty ones are left out.
    pub fn request(&self) -> Option<Request> {
        if !self.start.is_finite() || !self.increment.is_finite() || self.increment < 1.0 {
            return None;
        }
        let trimmed = |text: &str| Some(text.trim()).filter(|t| !t.is_empty()).map(str::to_owned);
        Some(Request {
            start: self.start.max(0.0) as usize,
            increment: self.increment as usize,
            prefix: trimmed(&self.prefix),
            suffix: trimmed(&self.suffix),
        })
    }
}

/// The first three numbers as they will look, then "...".
pub fn preview(start: f64, increment: f64, prefix: &str, suffix: &str) -> String {
    let mut result = String::new();
    for i in 0..3 {
        let number = start + increment * f64::from(i);
        result.push_str(&format!("{prefix}{number}{suffix}  "));
    }
    result.push_str("...");
    result
}

pub fn view<'a>(dialog: &Dialog, drafts: &Drafts) -> Element<'a, Message> {
    let number = |id, label: String, value, spec| {
        number_input(
            NumberInput {
                id,
                label: Some(label.as_str()),
                prefix: None,
                suffix: None,
                value: Some(value),
                spec,
                show_buttons: true,
                enabled: true,
            },
            drafts,
        )
        .map(Message::Field)
    };
    let label = |id, label: String, shown: &str| {
        labeled(
            Some(label.as_str()),
            text_field(id, shown, drafts, true).width(Length::Fill),
        )
        .map(Message::Field)
    };

    // The preview follows what is being typed, not only what is settled.
    let prefix = drafts.text(PREFIX, &dialog.prefix);
    let suffix = drafts.text(SUFFIX, &dialog.suffix);
    let shown = preview(dialog.start, dialog.increment, prefix, suffix);

    column![
        text(fl!("dialog-renumber-title")).size(18),
        number(
            START,
            fl!("dialog-renumber-start-number"),
            dialog.start,
            start_spec()
        ),
        number(
            INCREMENT,
            fl!("dialog-renumber-increment"),
            dialog.increment,
            increment_spec()
        ),
        label(PREFIX, fl!("dialog-renumber-prefix"), &dialog.prefix),
        label(SUFFIX, fl!("dialog-renumber-suffix"), &dialog.suffix),
        labeled(
            Some(fl!("dialog-renumber-preview").as_str()),
            text(shown),
        ),
        row![
            button(text(fl!("general-cancel")))
                .style(button::secondary)
                .on_press(Message::Cancel),
            button(text(fl!("general-run")))
                .style(button::primary)
                .on_press_maybe(dialog.request().map(|_| Message::Run)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .width(320)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::field::Commit as FieldCommit;

    fn commit(field: FieldId, text: &str) -> Outcome {
        Outcome::Commit(FieldCommit {
            field,
            text: text.to_owned(),
        })
    }

    #[test]
    fn the_preview_shows_three_numbers_and_dots() {
        assert_eq!(preview(1.0, 1.0, "", ""), "1  2  3  ...");
        assert_eq!(preview(10.0, 5.0, "Q", "-A"), "Q10-A  Q15-A  Q20-A  ...");
    }

    #[test]
    fn the_defaults_start_at_one_by_one() {
        let request = Dialog::default().request().unwrap();
        assert_eq!(
            request,
            Request {
                start: 1,
                increment: 1,
                prefix: None,
                suffix: None
            }
        );
    }

    #[test]
    fn typed_values_are_clamped_to_their_ranges() {
        let mut dialog = Dialog::default();
        dialog.apply(&commit(START, "-5"));
        assert_eq!(dialog.start, 0.0);
        dialog.apply(&commit(INCREMENT, "0"));
        assert_eq!(dialog.increment, 1.0, "an increment below 1 is raised to 1");
        dialog.apply(&commit(INCREMENT, "5"));
        assert_eq!(dialog.increment, 5.0);
    }

    #[test]
    fn unusable_text_leaves_the_value_alone() {
        let mut dialog = Dialog::default();
        dialog.apply(&commit(START, "abc"));
        dialog.apply(&commit(START, ""));
        assert_eq!(dialog.start, 1.0);
    }

    #[test]
    fn the_step_buttons_stay_within_range() {
        let mut dialog = Dialog::default();
        dialog.apply(&Outcome::Step(START, -1));
        dialog.apply(&Outcome::Step(START, -1));
        assert_eq!(dialog.start, 0.0);
        dialog.apply(&Outcome::Step(INCREMENT, -1));
        assert_eq!(dialog.increment, 1.0);
        dialog.apply(&Outcome::Step(INCREMENT, 1));
        assert_eq!(dialog.increment, 2.0);
    }

    #[test]
    fn prefix_and_suffix_are_trimmed_and_empty_ones_dropped() {
        let mut dialog = Dialog::default();
        dialog.apply(&commit(PREFIX, "  Q "));
        dialog.apply(&commit(SUFFIX, "   "));
        let request = dialog.request().unwrap();
        assert_eq!(request.prefix.as_deref(), Some("Q"));
        assert_eq!(request.suffix, None);
    }

    #[test]
    fn a_bad_increment_has_no_request() {
        let dialog = Dialog {
            increment: 0.0,
            ..Dialog::default()
        };
        assert_eq!(dialog.request(), None);
        let dialog = Dialog {
            start: f64::NAN,
            ..Dialog::default()
        };
        assert_eq!(dialog.request(), None);
    }

    #[test]
    fn other_fields_are_not_ours() {
        let mut dialog = Dialog::default();
        dialog.apply(&commit(FieldId("editor.number"), "7"));
        assert_eq!(dialog, Dialog::default());
        assert!(owns(START) && !owns(FieldId("editor.number")));
    }
}
