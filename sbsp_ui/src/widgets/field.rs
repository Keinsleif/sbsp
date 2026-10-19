// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The edit buffer behind every text field.
//!
//! The original inputs keep what the user is typing apart from the value
//! it edits, and apply it when the edit is finished: on Enter or when the
//! field loses focus (Escape throws the edit away). iced's `text_input`
//! only reports typing and Enter, not losing focus, so the missing piece
//! is rebuilt here.
//!
//! There is one [`Drafts`] for the whole app, holding at most one draft:
//! only one field has the keyboard at a time, so a draft for another
//! field means the previous one has been left. An edit is finished when
//!
//! - Enter is pressed in the field (`on_submit`);
//! - typing starts in a *different* field;
//! - a pointer press or Tab happens elsewhere and the field turns out no
//!   longer focused (checked with `operation::is_focused` after the
//!   event, so the focus change has already happened);
//!
//! and abandoned when Escape reaches the focused field.

use iced::widget::{TextInput, text_input};
use iced::{Task, widget::operation};

/// A field's identity: both the key of its draft and the `Id` of the
/// widget (so its focus can be queried). Static because a field is a spot
/// in the layout, not data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub &'static str);

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Input(FieldId, String),
    Submit(FieldId),
    /// The +/- buttons of a number field.
    Step(FieldId, i8),
    /// A pointer press or Tab happened; the focused field may have changed.
    FocusMayHaveChanged,
    FocusChecked(FieldId, bool),
    /// Escape reached a focused field.
    Escape,
}

/// A finished edit: the text to turn back into a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub field: FieldId,
    pub text: String,
}

/// What a message asks the owner of the values to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Commit(Commit),
    Step(FieldId, i8),
}

#[derive(Debug, Default)]
pub struct Drafts {
    active: Option<(FieldId, String)>,
}

impl Drafts {
    /// What `id` should show: its draft while it is being edited, else the
    /// formatted current value.
    pub fn text<'a>(&'a self, id: FieldId, shown: &'a str) -> &'a str {
        match &self.active {
            Some((field, text)) if *field == id => text,
            _ => shown,
        }
    }

    pub fn is_editing(&self, id: FieldId) -> bool {
        self.active.as_ref().is_some_and(|(field, _)| *field == id)
    }

    pub fn update(&mut self, message: Message) -> (Option<Outcome>, Task<Message>) {
        match message {
            Message::Input(id, text) => {
                let previous = self.active.replace((id, text));
                let finished = previous.filter(|(field, _)| *field != id);
                (finished.map(commit_of), Task::none())
            }
            Message::Submit(id) => (self.take_if(id).map(commit_of), Task::none()),
            Message::Step(id, direction) => (Some(Outcome::Step(id, direction)), Task::none()),
            Message::FocusMayHaveChanged => {
                let task = match &self.active {
                    Some((id, _)) => {
                        let id = *id;
                        operation::is_focused(id.0).map(move |focused| Message::FocusChecked(id, focused))
                    }
                    None => Task::none(),
                };
                (None, task)
            }
            Message::FocusChecked(id, focused) => {
                let outcome = if focused { None } else { self.take_if(id) };
                (outcome.map(commit_of), Task::none())
            }
            Message::Escape => {
                self.active = None;
                (None, Task::none())
            }
        }
    }

    /// Finishes whatever is being edited now. For a button that acts on
    /// the values: pressing it takes the focus away, but the field only
    /// notices a moment later, after the button's own message.
    pub fn flush(&mut self) -> Option<Outcome> {
        self.active.take().map(commit_of)
    }

    fn take_if(&mut self, id: FieldId) -> Option<(FieldId, String)> {
        match &self.active {
            Some((field, _)) if *field == id => self.active.take(),
            _ => None,
        }
    }
}

fn commit_of((field, text): (FieldId, String)) -> Outcome {
    Outcome::Commit(Commit { field, text })
}

/// A text field editing through `drafts`. `shown` is the formatted
/// current value. Without `enabled` it is greyed out and ignores input.
pub fn text_field<'a>(
    id: FieldId,
    shown: &str,
    drafts: &Drafts,
    enabled: bool,
) -> TextInput<'a, Message> {
    text_field_with_placeholder(id, "", shown, drafts, enabled)
}

/// [`text_field`] with a hint shown while the field is empty.
pub fn text_field_with_placeholder<'a>(
    id: FieldId,
    placeholder: &str,
    shown: &str,
    drafts: &Drafts,
    enabled: bool,
) -> TextInput<'a, Message> {
    let field = text_input(placeholder, drafts.text(id, shown)).id(id.0);
    if enabled {
        field
            .on_input(move |text| Message::Input(id, text))
            .on_submit(Message::Submit(id))
    } else {
        field
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: FieldId = FieldId("a");
    const B: FieldId = FieldId("b");

    #[test]
    fn flush_finishes_the_edit_in_progress() {
        let mut drafts = Drafts::default();
        assert_eq!(drafts.flush(), None);
        typed(&mut drafts, A, "12");
        assert_eq!(drafts.flush(), commit(A, "12"));
        assert!(!drafts.is_editing(A));
        assert_eq!(drafts.flush(), None);
    }

    fn typed(drafts: &mut Drafts, id: FieldId, text: &str) -> Option<Outcome> {
        drafts.update(Message::Input(id, text.to_string())).0
    }

    fn commit(field: FieldId, text: &str) -> Option<Outcome> {
        Some(Outcome::Commit(Commit {
            field,
            text: text.to_string(),
        }))
    }

    #[test]
    fn typing_shows_the_draft_until_it_is_finished() {
        let mut drafts = Drafts::default();
        assert_eq!(drafts.text(A, "1.0"), "1.0");

        assert_eq!(typed(&mut drafts, A, "1.5"), None);
        assert_eq!(drafts.text(A, "1.0"), "1.5");
        assert_eq!(drafts.text(B, "other"), "other");
        assert!(drafts.is_editing(A));
    }

    #[test]
    fn enter_commits_the_draft_once() {
        let mut drafts = Drafts::default();
        typed(&mut drafts, A, "2");

        assert_eq!(drafts.update(Message::Submit(A)).0, commit(A, "2"));
        assert_eq!(drafts.text(A, "shown"), "shown");
        assert_eq!(drafts.update(Message::Submit(A)).0, None);
    }

    #[test]
    fn enter_in_a_field_with_no_draft_does_nothing() {
        let mut drafts = Drafts::default();
        assert_eq!(drafts.update(Message::Submit(A)).0, None);
    }

    #[test]
    fn typing_in_another_field_commits_the_previous_one() {
        let mut drafts = Drafts::default();
        typed(&mut drafts, A, "2");

        assert_eq!(typed(&mut drafts, B, "3"), commit(A, "2"));
        assert!(drafts.is_editing(B));
        assert!(!drafts.is_editing(A));
    }

    #[test]
    fn more_typing_in_the_same_field_keeps_one_draft() {
        let mut drafts = Drafts::default();
        typed(&mut drafts, A, "2");
        assert_eq!(typed(&mut drafts, A, "25"), None);
        assert_eq!(drafts.text(A, ""), "25");
    }

    #[test]
    fn escape_throws_the_draft_away() {
        let mut drafts = Drafts::default();
        typed(&mut drafts, A, "2");

        assert_eq!(drafts.update(Message::Escape).0, None);
        assert_eq!(drafts.text(A, "shown"), "shown");
        assert_eq!(drafts.update(Message::FocusChecked(A, false)).0, None);
    }

    #[test]
    fn losing_focus_commits_and_keeping_it_does_not() {
        let mut drafts = Drafts::default();
        typed(&mut drafts, A, "2");

        assert_eq!(drafts.update(Message::FocusChecked(A, true)).0, None);
        assert!(drafts.is_editing(A));

        assert_eq!(drafts.update(Message::FocusChecked(A, false)).0, commit(A, "2"));
        assert!(!drafts.is_editing(A));
    }

    #[test]
    fn a_stale_focus_answer_for_another_field_is_ignored() {
        let mut drafts = Drafts::default();
        typed(&mut drafts, B, "9");

        assert_eq!(drafts.update(Message::FocusChecked(A, false)).0, None);
        assert!(drafts.is_editing(B));
    }

    #[test]
    fn step_is_passed_through() {
        let mut drafts = Drafts::default();
        assert_eq!(
            drafts.update(Message::Step(A, -1)).0,
            Some(Outcome::Step(A, -1))
        );
    }
}
