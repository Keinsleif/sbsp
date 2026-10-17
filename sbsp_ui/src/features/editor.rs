// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The bottom editor: in edit mode it takes the viewer's place and edits
//! the selected cue. Ported from `BottomEditor.vue`, `BasicEditor.vue` and
//! `TriggerEditor.vue`.
//!
//! This part has the tabs every cue has (Basics, Trigger). The tabs of
//! the individual cue types (audio, time & levels, fade, playback, group)
//! follow in the next patch.
//!
//! Every edit sends the whole edited cue (`ModelEdit::UpdateCue`), as the
//! original does. Text fields go through the shared [`Drafts`]: what is
//! typed is applied when the edit ends (see `widgets::field`). The notes
//! are a multi-line `text_editor`, which keeps its own `Content`; a copy
//! of its text goes into the same drafts so it ends the same way.

use std::collections::HashMap;
use std::fmt;

use iced::widget::{Space, button, center, column, container, pick_list, row, text, text_editor};
use iced::{Alignment, Element, Length, Theme};

use sbsp_backend::model::cue::{Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, Uuid};
use sbsp_backend::model::settings::CursorAdvanceTrigger;
use sbsp_frontend_settings::NameFormatSettings;

use crate::domain::cue::display_name;
use crate::domain::cue_edit::{self, ChainKind};
use crate::domain::field::TimeSpec;
use crate::domain::flat_list::FlatCueEntry;
use crate::fl;
use crate::state::model::ShowModelState;
use crate::widgets::field::{self, Drafts, FieldId, Outcome, text_field, text_field_with_placeholder};
use crate::widgets::icon::{Icon, icon};
use crate::widgets::inputs::{labeled, time_input};

/// The original's `h-62`.
pub const HEIGHT: f32 = 248.0;

const NUMBER: FieldId = FieldId("editor.number");
const NAME: FieldId = FieldId("editor.name");
const PRE_WAIT: FieldId = FieldId("editor.pre-wait");
const DURATION: FieldId = FieldId("editor.duration");
const NOTES: FieldId = FieldId("editor.notes");

/// Whether `id` is one of this editor's fields.
pub fn owns(id: FieldId) -> bool {
    [NUMBER, NAME, PRE_WAIT, DURATION, NOTES].contains(&id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Basics,
    Trigger,
}

#[derive(Clone)]
pub enum Message {
    Field(field::Message),
    Notes(text_editor::Action),
    Tab(Tab),
    ChainKind(ChainKind),
    ChainTarget(Option<Uuid>),
    Color(CueColor),
    Advance(CueCursorAdvanceTriggerOverride),
    TreatStop(bool),
    /// Append the playing position to the notes.
    Timestamp,
}

/// What the app has to do after an editor message.
pub enum Update {
    None,
    /// A text field message, for the shared drafts.
    Field(field::Message),
    /// Send this edited cue to the backend.
    Edit(Cue),
}

pub struct Editor {
    tab: Tab,
    notes: text_editor::Content,
    /// The cue `notes` shows.
    notes_cue: Option<Uuid>,
    /// The cue an edit in progress belongs to. Not the current selection:
    /// the click that ends an edit may already have selected another cue.
    pub draft_cue: Option<Uuid>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            tab: Tab::Basics,
            notes: text_editor::Content::new(),
            notes_cue: None,
            draft_cue: None,
        }
    }
}

/// `Content::text` ends with a newline the text does not have; both sides
/// of a comparison lose it.
fn plain(text: String) -> String {
    match text.strip_suffix('\n') {
        Some(stripped) => stripped.to_owned(),
        None => text,
    }
}

impl Editor {
    pub fn update(&mut self, message: Message, cue: Option<&Cue>, position: Option<f64>) -> Update {
        match message {
            Message::Field(message) => Update::Field(message),
            Message::Tab(tab) => {
                self.tab = tab;
                Update::None
            }
            Message::Notes(action) => {
                let is_edit = action.is_edit();
                self.notes.perform(action);
                if is_edit {
                    Update::Field(field::Message::Input(NOTES, plain(self.notes.text())))
                } else {
                    Update::None
                }
            }
            message => {
                let Some(cue) = cue else {
                    return Update::None;
                };
                let edited = match message {
                    Message::ChainKind(kind) => cue_edit::set_chain_kind(cue, kind),
                    Message::ChainTarget(target) => cue_edit::set_chain_target(cue, target),
                    Message::Color(color) => cue_edit::set_color(cue, color),
                    Message::Advance(value) => cue_edit::set_cursor_advance_override(cue, value),
                    Message::TreatStop(value) => cue_edit::set_treat_stop_as_completed(cue, value),
                    Message::Timestamp => position.and_then(|p| cue_edit::append_timestamp(cue, p)),
                    Message::Field(_) | Message::Notes(_) | Message::Tab(_) => None,
                };
                edited.map_or(Update::None, Update::Edit)
            }
        }
    }

    /// A finished text edit of one of this editor's fields, applied to the
    /// cue it was made on.
    pub fn apply(&self, outcome: &Outcome, cue: &Cue) -> Option<Cue> {
        let Outcome::Commit(commit) = outcome else {
            return None;
        };
        let text = commit.text.as_str();
        match commit.field {
            NUMBER => cue_edit::set_number(cue, text),
            NAME => cue_edit::set_name(cue, text),
            PRE_WAIT => cue_edit::set_pre_wait(cue, text),
            DURATION => cue_edit::set_duration(cue, text),
            NOTES => cue_edit::set_notes(cue, text),
            _ => None,
        }
    }

    /// Keeps the notes editor showing the selected cue's notes, unless
    /// they are being edited right now.
    pub fn sync(&mut self, cue: Option<&Cue>, drafts: &Drafts) {
        if drafts.is_editing(NOTES) {
            return;
        }
        let id = cue.map(|cue| cue.id);
        let wanted = cue.map(|cue| cue.notes.as_str()).unwrap_or("");
        if self.notes_cue != id
            || plain(self.notes.text()) != plain(text_editor::Content::with_text(wanted).text())
        {
            self.notes = text_editor::Content::with_text(wanted);
            self.notes_cue = id;
        }
    }
}

// ---------------------------------------------------------------------
// View

pub struct Context<'a> {
    pub cue: Option<&'a Cue>,
    pub model: &'a ShowModelState,
    pub name_format: &'a NameFormatSettings,
    pub drafts: &'a Drafts,
    /// The cue's duration as the list shows it.
    pub duration: Option<f64>,
    /// Whether the cue is playing (its chain mode is locked, and a
    /// timestamp can be taken).
    pub is_active: bool,
    /// The chain a surrounding group imposes, if any.
    pub chain_override: Option<CueChain>,
}

/// `flat` is the cue list in display order (hidden rows included), for
/// the list of cues a chain can continue with.
pub fn view<'a>(
    editor: &'a Editor,
    ctx: Context<'a>,
    flat: &[FlatCueEntry],
) -> Element<'a, Message> {
    let Some(cue) = ctx.cue else {
        return center(text(fl!("main-bottom-editor-no-selection")).size(24))
            .width(Length::Fill)
            .height(HEIGHT)
            .into();
    };

    let tabs = row![
        tab_button(editor.tab, Tab::Basics, fl!("main-bottom-editor-basics-title")),
        tab_button(editor.tab, Tab::Trigger, fl!("main-bottom-editor-trigger-title")),
    ]
    .spacing(4);

    let body = match editor.tab {
        Tab::Basics => basics(editor, &ctx, cue, flat),
        Tab::Trigger => trigger(cue),
    };

    column![tabs, container(body).width(Length::Fill).height(Length::Fill)]
        .spacing(8)
        .padding(8)
        .height(HEIGHT)
        .into()
}

fn tab_button<'a>(current: Tab, tab: Tab, label: String) -> Element<'a, Message> {
    let active = current == tab;
    button(text(label).size(14))
        .padding([4, 12])
        .style(move |theme: &Theme, status| {
            if active {
                button::primary(theme, status)
            } else {
                button::secondary(theme, status)
            }
        })
        .on_press(Message::Tab(tab))
        .into()
}

fn basics<'a>(
    editor: &'a Editor,
    ctx: &Context<'a>,
    cue: &'a Cue,
    flat: &[FlatCueEntry],
) -> Element<'a, Message> {
    let drafts = ctx.drafts;
    let spec = TimeSpec::default();

    let number: Element<'a, field::Message> = text_field(NUMBER, &cue.number, drafts, true)
        .align_x(iced::alignment::Horizontal::Center)
        .into();
    let number = labeled(Some(&fl!("main-number")), number.map(Message::Field));

    let duration = time_input(
        DURATION,
        Some(&fl!("main-duration")),
        ctx.duration,
        &spec,
        drafts,
        cue_edit::duration_is_editable(cue),
    )
    .map(Message::Field);
    let pre_wait = time_input(
        PRE_WAIT,
        Some(&fl!("main-pre-wait")),
        Some(cue.pre_wait),
        &spec,
        drafts,
        true,
    )
    .map(Message::Field);

    let chain = ctx.chain_override.as_ref().unwrap_or(&cue.chain);
    let chain_kind = cue_edit::chain_kind(chain);
    let locked = ctx.is_active || ctx.chain_override.is_some();
    let chain_mode = labeled(
        Some(&fl!("main-chain-mode-title")),
        picker(
            [ChainKind::DoNotChain, ChainKind::AfterStart, ChainKind::AfterComplete]
                .into_iter()
                .map(|kind| Choice::new(kind, chain_label(kind)))
                .collect(),
            &chain_kind,
            !locked,
            Message::ChainKind,
        ),
    );

    let left = column![number, duration, pre_wait, chain_mode]
        .spacing(8)
        .width(Length::Fixed(160.0));

    let name: Element<'a, field::Message> = text_field_with_placeholder(
        NAME,
        &display_name(
            &Cue {
                name: None,
                ..cue.clone()
            },
            &ctx.model.cue_list.cues,
            ctx.name_format,
        ),
        cue.name.as_deref().unwrap_or(""),
        drafts,
        true,
    )
    .into();
    let name = labeled(Some(&fl!("main-name")), name.map(Message::Field));

    let notes = labeled(
        Some(&fl!("main-notes")),
        text_editor(&editor.notes)
            .id(NOTES.0)
            .on_action(Message::Notes)
            .height(Length::Fill),
    );

    // The cue to continue with: any other cue, or the next in the list.
    let cues = &ctx.model.cue_list.cues;
    let mut targets = vec![Choice::new(None, fl!("main-bottom-editor-basics-next-cue"))];
    targets.extend(
        flat.iter()
            .filter(|entry| entry.cue_id != cue.id)
            .filter_map(|entry| cues.get(&entry.cue_id))
            .map(|other| {
                Choice::new(Some(other.id), choice_name(other, cues, ctx.name_format))
            }),
    );
    let target = labeled(
        Some(&fl!("main-bottom-editor-continue-target-cue")),
        picker(
            targets,
            &cue_edit::chain_target(chain),
            !locked && chain_kind != ChainKind::DoNotChain,
            Message::ChainTarget,
        ),
    );

    let colors = labeled(
        Some(&fl!("main-bottom-editor-basics-color")),
        picker(
            COLORS.iter().map(|&color| Choice::new(color, color_label(color))).collect(),
            &cue.color,
            true,
            Message::Color,
        ),
    );

    let timestamp = button(text(fl!("main-bottom-editor-basics-timestamp")))
        .on_press_maybe(ctx.is_active.then_some(Message::Timestamp));

    let bottom = row![
        container(target).width(Length::FillPortion(1)).max_width(600.0),
        Space::new().width(Length::Fill),
        container(colors).width(Length::Fixed(150.0)),
        timestamp,
    ]
    .spacing(12)
    .align_y(Alignment::End);

    let right = column![name, notes, bottom].spacing(8).width(Length::Fill);

    row![left, right].spacing(8).into()
}

fn trigger(cue: &Cue) -> Element<'_, Message> {
    let advance = labeled(
        Some(&fl!("main-bottom-editor-trigger-cursor-advance-trigger-override-title")),
        picker(
            ADVANCE
                .iter()
                .map(|&value| Choice::new(value, advance_label(value)))
                .collect(),
            &cue.cursor_advance_trigger_override,
            true,
            Message::Advance,
        ),
    );

    column![
        container(advance).max_width(500.0),
        check(
            fl!("main-bottom-editor-trigger-treat-stop-as-completed"),
            cue.treat_stop_as_completed,
            Message::TreatStop(!cue.treat_stop_as_completed),
        ),
    ]
    .spacing(12)
    .into()
}

// ---------------------------------------------------------------------
// Parts

/// An option with its translated label. Equality is on the value alone,
/// so the selection survives a language switch.
#[derive(Clone)]
struct Choice<T> {
    value: T,
    label: String,
}

impl<T> Choice<T> {
    fn new(value: T, label: String) -> Self {
        Self { value, label }
    }
}

impl<T: PartialEq> PartialEq for Choice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// A drop-down list; without `enabled` just the current choice, greyed
/// out (`pick_list` has no disabled state).
fn picker<'a, T>(
    options: Vec<Choice<T>>,
    selected: &T,
    enabled: bool,
    on_select: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: Clone + PartialEq + 'a,
{
    let current = options.iter().find(|option| option.value == *selected).cloned();

    if !enabled {
        let label = current.map(|choice| choice.label).unwrap_or_default();
        return container(text(label))
            .padding([5, 10])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let mut style = container::bordered_box(theme);
                style.text_color = Some(
                    theme.extended_palette().background.base.text.scale_alpha(0.5),
                );
                style
            })
            .into();
    }

    pick_list(options, current, move |choice: Choice<T>| on_select(choice.value))
        .width(Length::Fill)
        .into()
}

/// A labeled check box (a button, as iced's own check box is not used
/// here).
fn check<'a>(label: String, checked: bool, on_press: Message) -> Element<'a, Message> {
    let glyph = if checked {
        Icon::CheckboxMarked
    } else {
        Icon::CheckboxBlankOutline
    };
    button(row![icon(glyph, 20.0), text(label)].spacing(8).align_y(Alignment::Center))
        .style(button::text)
        .on_press(on_press)
        .into()
}

fn choice_name(cue: &Cue, cues: &HashMap<Uuid, Cue>, name_format: &NameFormatSettings) -> String {
    crate::features::header::cue_title(cue, display_name(cue, cues, name_format))
}

const COLORS: [CueColor; 9] = [
    CueColor::None,
    CueColor::Red,
    CueColor::Purple,
    CueColor::Blue,
    CueColor::Cyan,
    CueColor::Green,
    CueColor::Yellow,
    CueColor::Orange,
    CueColor::Grey,
];

fn color_label(color: CueColor) -> String {
    match color {
        CueColor::None => fl!("general-none"),
        CueColor::Red => "Red".to_string(),
        CueColor::Purple => "Purple".to_string(),
        CueColor::Blue => "Blue".to_string(),
        CueColor::Cyan => "Cyan".to_string(),
        CueColor::Green => "Green".to_string(),
        CueColor::Yellow => "Yellow".to_string(),
        CueColor::Orange => "Orange".to_string(),
        CueColor::Grey => "Grey".to_string(),
    }
}

fn chain_label(kind: ChainKind) -> String {
    match kind {
        ChainKind::DoNotChain => fl!("main-chain-mode-do-not-chain"),
        ChainKind::AfterStart => fl!("main-chain-mode-after-start"),
        ChainKind::AfterComplete => fl!("main-chain-mode-after-complete"),
    }
}

const ADVANCE: [CueCursorAdvanceTriggerOverride; 4] = [
    CueCursorAdvanceTriggerOverride::None,
    CueCursorAdvanceTriggerOverride::Override(CursorAdvanceTrigger::OnTriggered),
    CueCursorAdvanceTriggerOverride::Override(CursorAdvanceTrigger::OnCompleted),
    CueCursorAdvanceTriggerOverride::Override(CursorAdvanceTrigger::Manual),
];

fn advance_label(value: CueCursorAdvanceTriggerOverride) -> String {
    match value {
        CueCursorAdvanceTriggerOverride::None => {
            fl!("main-bottom-editor-trigger-cursor-advance-trigger-none")
        }
        CueCursorAdvanceTriggerOverride::Override(CursorAdvanceTrigger::OnTriggered) => {
            fl!("main-bottom-editor-trigger-cursor-advance-trigger-on-triggered")
        }
        CueCursorAdvanceTriggerOverride::Override(CursorAdvanceTrigger::OnCompleted) => {
            fl!("main-bottom-editor-trigger-cursor-advance-trigger-on-completed")
        }
        CueCursorAdvanceTriggerOverride::Override(CursorAdvanceTrigger::Manual) => {
            fl!("main-bottom-editor-trigger-cursor-advance-trigger-manual")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::field::Commit;
    use sbsp_backend::model::cue::{CueCursorAdvanceTriggerOverride as Advance, CueParam, WaitCueParam};

    fn cue() -> Cue {
        Cue {
            id: Uuid::from_u128(1),
            number: "1".to_string(),
            name: None,
            notes: "memo".to_string(),
            color: CueColor::default(),
            pre_wait: 0.0,
            chain: CueChain::default(),
            treat_stop_as_completed: false,
            cursor_advance_trigger_override: Advance::default(),
            parent_id: None,
            params: CueParam::Wait(WaitCueParam { duration: 1.0 }),
        }
    }

    fn commit(field: FieldId, text: &str) -> Outcome {
        Outcome::Commit(Commit {
            field,
            text: text.to_string(),
        })
    }

    #[test]
    fn a_committed_field_edits_the_cue_it_belongs_to() {
        let editor = Editor::default();
        let cue = cue();

        assert_eq!(editor.apply(&commit(NUMBER, "7"), &cue).unwrap().number, "7");
        assert_eq!(
            editor.apply(&commit(NAME, "Intro"), &cue).unwrap().name.as_deref(),
            Some("Intro")
        );
        assert_eq!(editor.apply(&commit(NOTES, "new"), &cue).unwrap().notes, "new");
        assert!(editor.apply(&commit(NUMBER, "1"), &cue).is_none(), "unchanged");
        assert!(editor.apply(&commit(FieldId("other"), "x"), &cue).is_none());
        assert!(editor.apply(&Outcome::Step(NUMBER, 1), &cue).is_none());
    }

    #[test]
    fn the_notes_editor_follows_the_selected_cue() {
        let mut editor = Editor::default();
        let drafts = Drafts::default();
        let cue = cue();

        editor.sync(Some(&cue), &drafts);
        assert_eq!(plain(editor.notes.text()), "memo");
        assert_eq!(editor.notes_cue, Some(cue.id));

        let mut changed = cue.clone();
        changed.notes = "changed".to_string();
        editor.sync(Some(&changed), &drafts);
        assert_eq!(plain(editor.notes.text()), "changed");

        editor.sync(None, &drafts);
        assert_eq!(plain(editor.notes.text()), "");
        assert_eq!(editor.notes_cue, None);
    }

    #[test]
    fn notes_being_typed_are_not_overwritten() {
        let mut editor = Editor::default();
        let mut drafts = Drafts::default();
        let cue = cue();
        editor.sync(Some(&cue), &drafts);

        let _ = drafts.update(field::Message::Input(NOTES, "typing".to_string()));
        let mut changed = cue.clone();
        changed.notes = "from the backend".to_string();
        editor.sync(Some(&changed), &drafts);

        assert_eq!(plain(editor.notes.text()), "memo");
    }

    #[test]
    fn a_tab_message_switches_tabs() {
        let mut editor = Editor::default();
        assert!(matches!(
            editor.update(Message::Tab(Tab::Trigger), None, None),
            Update::None
        ));
        assert_eq!(editor.tab, Tab::Trigger);
    }

    #[test]
    fn a_trigger_choice_becomes_an_edited_cue() {
        let mut editor = Editor::default();
        let cue = cue();

        match editor.update(Message::TreatStop(true), Some(&cue), None) {
            Update::Edit(edited) => assert!(edited.treat_stop_as_completed),
            _ => panic!("expected an edit"),
        }
        assert!(matches!(
            editor.update(Message::TreatStop(false), Some(&cue), None),
            Update::None
        ));
        assert!(matches!(
            editor.update(Message::TreatStop(true), None, None),
            Update::None
        ));
    }

    #[test]
    fn a_timestamp_needs_a_position() {
        let mut editor = Editor::default();
        let cue = cue();

        assert!(matches!(
            editor.update(Message::Timestamp, Some(&cue), None),
            Update::None
        ));
        match editor.update(Message::Timestamp, Some(&cue), Some(3.0)) {
            Update::Edit(edited) => assert_eq!(edited.notes, "memo\n[00:03.00] "),
            _ => panic!("expected an edit"),
        }
    }

    #[test]
    fn only_its_own_fields_are_owned() {
        assert!(owns(NOTES));
        assert!(!owns(FieldId("gallery.number")));
    }
}
