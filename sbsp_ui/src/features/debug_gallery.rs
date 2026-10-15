// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Debug aid: the input parts (`widgets::inputs`) on a dialog of their own,
//! each holding a plain local value, so they can be tried before the
//! editors that use them exist. Opened from the Debug menu and removed
//! together with it at the end of Phase 6.

use std::time::Instant;

use iced::widget::{button, column, text};
use iced::{Element, Length};

use crate::domain::field::{
    Commit, NumberSpec, TimeSpec, commit_number, commit_time, commit_volume, step_number,
};
use crate::widgets::field::{self, Drafts, FieldId, Outcome};
use crate::widgets::inputs::{
    DoubleRelease, FaderMessage, NumberInput, PanningFader, ThumbAmount, VolumeFader, number_input,
    panning_fader, time_input, volume_fader,
};

const NUMBER: FieldId = FieldId("gallery.number");
const TIME: FieldId = FieldId("gallery.time");
const VOLUME: FieldId = FieldId("gallery.volume");
const PANNING: FieldId = FieldId("gallery.panning");

#[derive(Clone)]
pub enum Message {
    Field(field::Message),
    Volume(FaderMessage),
    Panning(FaderMessage),
    Close,
}

pub struct Gallery {
    pub open: bool,
    pub(crate) number: Option<f64>,
    time: f64,
    volume: f64,
    panning: f64,
    volume_release: DoubleRelease,
    panning_release: DoubleRelease,
    /// How many times a "send to the backend" would have happened: once per
    /// finished drag or committed edit, not per movement.
    sent: u32,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            open: false,
            number: Some(5.0),
            time: 0.0,
            volume: 0.0,
            panning: 0.0,
            volume_release: DoubleRelease::default(),
            panning_release: DoubleRelease::default(),
            sent: 0,
        }
    }
}

fn number_spec() -> NumberSpec {
    NumberSpec {
        min: Some(0.0),
        max: Some(10.0),
        step: 0.5,
        precision: 1,
        accept_null: true,
    }
}

fn time_spec() -> TimeSpec {
    TimeSpec {
        max: Some(3600.0),
        ..TimeSpec::default()
    }
}

fn panning_spec() -> NumberSpec {
    NumberSpec {
        min: Some(-1.0),
        max: Some(1.0),
        step: 1.0 / 8.0,
        precision: 3,
        accept_null: false,
    }
}

impl Gallery {
    /// A finished edit or a step button, if it belongs to one of this
    /// dialog's fields.
    pub fn apply(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Commit(commit) if commit.field == NUMBER => {
                match commit_number(&commit.text, self.number, &number_spec()) {
                    Commit::Value(value) => self.number = Some(value),
                    Commit::Null => self.number = None,
                    Commit::Unchanged => {}
                }
            }
            Outcome::Commit(commit) if commit.field == TIME => {
                if let Commit::Value(value) =
                    commit_time(&commit.text, Some(self.time), &time_spec())
                {
                    self.time = value;
                }
            }
            Outcome::Commit(commit) if commit.field == VOLUME => {
                if let Commit::Value(value) = commit_volume(&commit.text, self.volume) {
                    self.volume = value;
                    self.sent += 1;
                }
            }
            Outcome::Commit(commit) if commit.field == PANNING => {
                if let Commit::Value(value) =
                    commit_number(&commit.text, Some(self.panning), &panning_spec())
                {
                    self.panning = value;
                    self.sent += 1;
                }
            }
            Outcome::Step(id, direction) if *id == NUMBER => {
                self.number = Some(step_number(self.number, &number_spec(), *direction));
            }
            _ => {}
        }
    }

    /// A volume fader message. A number-field message inside it is handed
    /// back for the caller to run through the shared drafts.
    pub fn volume_message(&mut self, message: FaderMessage, now: Instant) -> Option<field::Message> {
        match message {
            FaderMessage::Field(message) => return Some(message),
            FaderMessage::Moved(decibels) => self.volume = decibels,
            FaderMessage::Released => {
                if self.volume_release.released(now) {
                    self.volume = 0.0;
                }
                self.sent += 1;
            }
            FaderMessage::Nothing => {}
        }
        None
    }

    pub fn panning_message(
        &mut self,
        message: FaderMessage,
        now: Instant,
    ) -> Option<field::Message> {
        match message {
            FaderMessage::Field(message) => return Some(message),
            FaderMessage::Moved(panning) => self.panning = panning,
            FaderMessage::Released => {
                if self.panning_release.released(now) {
                    self.panning = 0.0;
                }
                self.sent += 1;
            }
            FaderMessage::Nothing => {}
        }
        None
    }

    pub fn view<'a>(&'a self, drafts: &Drafts) -> Element<'a, Message> {
        let number = number_input(
            NumberInput {
                id: NUMBER,
                label: Some("Number (0-10, step 0.5, empty allowed)"),
                prefix: None,
                suffix: Some("pt".to_string()),
                value: self.number,
                spec: number_spec(),
                show_buttons: true,
                enabled: true,
            },
            drafts,
        )
        .map(Message::Field);

        let time = time_input(
            TIME,
            Some("Time (mm:ss.cc, max 60:00)"),
            Some(self.time),
            &time_spec(),
            drafts,
            true,
        )
        .map(Message::Field);

        let volume = volume_fader(
            VolumeFader {
                id: VOLUME,
                label: "Volume",
                volume: self.volume,
                thumb: ThumbAmount::Full,
                enabled: true,
            },
            drafts,
        )
        .map(Message::Volume);

        let panning = panning_fader(
            PanningFader {
                id: PANNING,
                label: "Panning",
                panning: self.panning,
                enabled: true,
            },
            drafts,
        )
        .map(Message::Panning);

        let readout = text(format!(
            "number={:?}  time={:.2}  volume={:.2} dB  panning={:.3}  sent={}",
            self.number, self.time, self.volume, self.panning, self.sent
        ))
        .size(12);

        column![
            text("Input Gallery (debug)").size(18),
            number,
            time,
            volume,
            panning,
            readout,
            button("Close").on_press(Message::Close),
        ]
        .spacing(12)
        .width(Length::Fixed(420.0))
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::field::Commit as FieldCommit;

    fn commit(field: FieldId, text: &str) -> Outcome {
        Outcome::Commit(FieldCommit {
            field,
            text: text.to_string(),
        })
    }

    #[test]
    fn a_committed_number_is_clamped_and_an_empty_one_clears() {
        let mut gallery = Gallery::default();

        gallery.apply(&commit(NUMBER, "99"));
        assert_eq!(gallery.number, Some(10.0));

        gallery.apply(&commit(NUMBER, ""));
        assert_eq!(gallery.number, None);
    }

    #[test]
    fn the_step_buttons_move_the_number() {
        let mut gallery = Gallery::default();
        gallery.apply(&Outcome::Step(NUMBER, 1));
        assert_eq!(gallery.number, Some(5.5));
        gallery.apply(&Outcome::Step(NUMBER, -1));
        gallery.apply(&Outcome::Step(NUMBER, -1));
        assert_eq!(gallery.number, Some(4.5));
    }

    #[test]
    fn other_fields_are_ignored() {
        let mut gallery = Gallery::default();
        gallery.apply(&commit(FieldId("somewhere.else"), "3"));
        assert_eq!(gallery.number, Some(5.0));
    }

    #[test]
    fn dragging_a_fader_sends_once_on_release() {
        let mut gallery = Gallery::default();
        let now = Instant::now();

        for decibels in [-1.0, -2.0, -3.0] {
            assert_eq!(gallery.volume_message(FaderMessage::Moved(decibels), now), None);
        }
        assert_eq!(gallery.volume, -3.0);
        assert_eq!(gallery.sent, 0);

        gallery.volume_message(FaderMessage::Released, now);
        assert_eq!(gallery.sent, 1);
    }

    #[test]
    fn a_double_release_resets_the_fader() {
        let mut gallery = Gallery::default();
        let now = Instant::now();

        gallery.panning_message(FaderMessage::Moved(0.5), now);
        gallery.panning_message(FaderMessage::Released, now);
        gallery.panning_message(
            FaderMessage::Released,
            now + std::time::Duration::from_millis(100),
        );

        assert_eq!(gallery.panning, 0.0);
    }

    #[test]
    fn a_field_message_inside_a_fader_is_handed_back() {
        let mut gallery = Gallery::default();
        let inner = field::Message::Submit(VOLUME);

        assert_eq!(
            gallery.volume_message(FaderMessage::Field(inner.clone()), Instant::now()),
            Some(inner)
        );
    }
}
