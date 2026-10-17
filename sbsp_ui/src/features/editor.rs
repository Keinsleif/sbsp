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

use std::path::PathBuf;
use std::time::Instant;

use iced::widget::{Space, button, center, column, container, row, text, text_editor};
use iced::{Alignment, Element, Length, Theme};

use sbsp_backend::model::cue::{
    Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid,
};
use sbsp_backend::model::settings::CursorAdvanceTrigger;
use sbsp_frontend_settings::NameFormatSettings;

use crate::domain::cue::display_name;
use crate::domain::cue_edit::{self, ChainKind, FadeSlot, GroupModeKind};
use crate::domain::easing::CurveKind;
use crate::domain::field::{Commit, commit_volume};
use crate::domain::field::TimeSpec;
use crate::domain::flat_list::FlatCueEntry;
use crate::fl;
use crate::state::model::ShowModelState;
use crate::widgets::field::{self, Drafts, FieldId, Outcome, text_field, text_field_with_placeholder};
use crate::widgets::inputs::{DoubleRelease, FaderMessage, labeled, time_input};

mod parts;
mod type_tabs;

use parts::{Choice, check, choice_name, picker};

/// The original's `h-62`.
pub const HEIGHT: f32 = 248.0;

const NUMBER: FieldId = FieldId("editor.number");
const NAME: FieldId = FieldId("editor.name");
const PRE_WAIT: FieldId = FieldId("editor.pre-wait");
const DURATION: FieldId = FieldId("editor.duration");
const NOTES: FieldId = FieldId("editor.notes");
const AUDIO_TARGET: FieldId = FieldId("editor.audio.target");
const VOLUME: FieldId = FieldId("editor.volume");
const START: FieldId = FieldId("editor.start-time");
const END: FieldId = FieldId("editor.end-time");
const PAN: FieldId = FieldId("editor.pan");

/// The duration and intensity fields of a fade parameter.
fn fade_fields(slot: FadeSlot) -> (FieldId, FieldId) {
    match slot {
        FadeSlot::AudioIn => (
            FieldId("editor.fade-in.duration"),
            FieldId("editor.fade-in.power"),
        ),
        FadeSlot::AudioOut => (
            FieldId("editor.fade-out.duration"),
            FieldId("editor.fade-out.power"),
        ),
        FadeSlot::Fade => (
            FieldId("editor.fade.duration"),
            FieldId("editor.fade.power"),
        ),
    }
}

const FADE_SLOTS: [FadeSlot; 3] = [FadeSlot::AudioIn, FadeSlot::AudioOut, FadeSlot::Fade];

/// Whether `id` is one of this editor's fields.
pub fn owns(id: FieldId) -> bool {
    [NUMBER, NAME, PRE_WAIT, DURATION, NOTES, AUDIO_TARGET, VOLUME, START, END, PAN].contains(&id)
        || FADE_SLOTS.iter().any(|&slot| {
            let (duration, power) = fade_fields(slot);
            id == duration || id == power
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Basics,
    Trigger,
    Audio,
    Levels,
    Fade,
    Playback,
    Group,
}

impl Tab {
    /// The tabs a cue has, in order.
    fn of(cue: &Cue) -> Vec<Tab> {
        let mut tabs = vec![Tab::Basics, Tab::Trigger];
        match &cue.params {
            CueParam::Audio(_) => tabs.extend([Tab::Audio, Tab::Levels]),
            CueParam::Fade(_) => tabs.push(Tab::Fade),
            CueParam::Start(_) | CueParam::Stop(_) | CueParam::Pause(_) | CueParam::Load(_) => {
                tabs.push(Tab::Playback);
            }
            CueParam::Group { .. } => tabs.push(Tab::Group),
            CueParam::Wait(_) => {}
        }
        tabs
    }

    fn label(self) -> String {
        match self {
            Tab::Basics => fl!("main-bottom-editor-basics-title"),
            Tab::Trigger => fl!("main-bottom-editor-trigger-title"),
            Tab::Audio => fl!("main-bottom-editor-audio-title"),
            Tab::Levels => fl!("main-bottom-editor-time-levels-title"),
            Tab::Fade => fl!("main-bottom-editor-fade-title"),
            Tab::Playback => fl!("main-bottom-editor-playback-title"),
            Tab::Group => fl!("main-bottom-editor-group-title"),
        }
    }
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
    /// Choose the audio file in a dialog.
    PickAudio,
    SoundStatic(bool),
    FadeEnabled(FadeSlot, bool),
    FadeKind(FadeSlot, CurveKind),
    FadeTarget(Uuid),
    /// The volume fader of an audio or fade cue.
    Volume(FaderMessage),
    Pan(FaderMessage),
    Repeat(bool),
    /// Set the volume from the file's loudness / peak.
    VolumeToLufs,
    VolumeToPeak,
    PlaybackTarget(Uuid),
    StopHard(bool),
    GroupMode(GroupModeKind),
    GroupRepeat(bool),
    GroupEnter(bool),
}

/// What the app has to do after an editor message.
pub enum Update {
    None,
    /// A text field message, for the shared drafts.
    Field(field::Message),
    /// Send this edited cue to the backend.
    Edit(Box<Cue>),
    /// Ask for an audio file for this cue.
    PickAudio(Uuid),
    /// Set the cue's volume from the analyzed file (the app knows it).
    MatchVolume(Match),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    Lufs,
    Peak,
}

pub struct Editor {
    tab: Tab,
    notes: text_editor::Content,
    /// The cue `notes` shows.
    notes_cue: Option<Uuid>,
    /// The cue an edit in progress belongs to. Not the current selection:
    /// the click that ends an edit may already have selected another cue.
    pub draft_cue: Option<Uuid>,
    /// The volume of a fade cue while its fader is being dragged: the
    /// value goes to the backend on release, but the fader must follow the
    /// pointer meanwhile (and until the backend's answer arrives).
    drag_volume: Option<(Uuid, f64)>,
    volume_release: DoubleRelease,
    /// The same for the panning fader.
    drag_pan: Option<(Uuid, f64)>,
    pan_release: DoubleRelease,
    /// The duration of the selected audio cue's file, if analyzed (the
    /// app keeps it current): an end time at the duration is no end time.
    pub file_duration: Option<f64>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            tab: Tab::Basics,
            notes: text_editor::Content::new(),
            notes_cue: None,
            draft_cue: None,
            drag_volume: None,
            volume_release: DoubleRelease::default(),
            drag_pan: None,
            pan_release: DoubleRelease::default(),
            file_duration: None,
        }
    }
}

fn edit(edited: Option<Cue>) -> Update {
    edited.map_or(Update::None, |cue| Update::Edit(Box::new(cue)))
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
    pub fn update(
        &mut self,
        message: Message,
        cue: Option<&Cue>,
        position: Option<f64>,
        now: Instant,
    ) -> Update {
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
            Message::Volume(FaderMessage::Field(message))
            | Message::Pan(FaderMessage::Field(message)) => Update::Field(message),
            Message::Volume(FaderMessage::Moved(decibels)) => {
                self.drag_volume = cue.map(|cue| (cue.id, decibels));
                Update::None
            }
            Message::Volume(FaderMessage::Released) => {
                let Some(cue) = cue else {
                    return Update::None;
                };
                let value = if self.volume_release.released(now) {
                    Some(0.0)
                } else {
                    self.drag_volume
                        .filter(|(id, _)| *id == cue.id)
                        .map(|(_, value)| value)
                };
                let Some(value) = value else {
                    return Update::None;
                };
                self.drag_volume = Some((cue.id, value));
                edit(cue_edit::set_level(cue, value))
            }
            Message::Pan(FaderMessage::Moved(pan)) => {
                self.drag_pan = cue.map(|cue| (cue.id, pan));
                Update::None
            }
            Message::Pan(FaderMessage::Released) => {
                let Some(cue) = cue else {
                    return Update::None;
                };
                let value = if self.pan_release.released(now) {
                    Some(0.0)
                } else {
                    self.drag_pan
                        .filter(|(id, _)| *id == cue.id)
                        .map(|(_, value)| value)
                };
                let Some(value) = value else {
                    return Update::None;
                };
                self.drag_pan = Some((cue.id, value));
                edit(cue_edit::set_pan(cue, value))
            }
            Message::Volume(FaderMessage::Nothing) | Message::Pan(FaderMessage::Nothing) => {
                Update::None
            }
            Message::VolumeToLufs => Update::MatchVolume(Match::Lufs),
            Message::VolumeToPeak => Update::MatchVolume(Match::Peak),
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
                    Message::PickAudio => return Update::PickAudio(cue.id),
                    Message::SoundStatic(value) => cue_edit::set_sound_static(cue, value),
                    Message::FadeEnabled(slot, enabled) => cue_edit::set_fade_param(
                        cue,
                        slot,
                        cue_edit::fade_enabled(cue_edit::fade_param(cue, slot), enabled),
                    ),
                    Message::FadeKind(slot, kind) => cue_edit::fade_param(cue, slot)
                        .and_then(|param| cue_edit::fade_with_kind(param, kind))
                        .and_then(|param| cue_edit::set_fade_param(cue, slot, Some(param))),
                    Message::FadeTarget(target) => cue_edit::set_fade_target(cue, target),
                    Message::PlaybackTarget(target) => cue_edit::set_playback_target(cue, target),
                    Message::StopHard(hard) => cue_edit::set_stop_hard(cue, hard),
                    Message::GroupMode(kind) => cue_edit::set_group_mode_kind(cue, kind),
                    Message::GroupRepeat(repeat) => cue_edit::set_group_repeat(cue, repeat),
                    Message::GroupEnter(enter) => cue_edit::set_group_enter(cue, enter),
                    Message::Repeat(repeat) => cue_edit::set_repeat(cue, repeat),
                    Message::Field(_)
                    | Message::Notes(_)
                    | Message::Tab(_)
                    | Message::Volume(_)
                    | Message::Pan(_)
                    | Message::VolumeToLufs
                    | Message::VolumeToPeak => None,
                };
                edit(edited)
            }
        }
    }

    /// A finished text edit of one of this editor's fields (or a press on
    /// a step button), applied to the cue it was made on.
    pub fn apply(&self, outcome: &Outcome, cue: &Cue) -> Option<Cue> {
        match outcome {
            Outcome::Commit(commit) => self.apply_commit(commit.field, commit.text.as_str(), cue),
            Outcome::Step(id, direction) => FADE_SLOTS.into_iter().find_map(|slot| {
                let (_, power) = fade_fields(slot);
                if *id != power {
                    return None;
                }
                let param = cue_edit::fade_param(cue, slot)?;
                let stepped = cue_edit::fade_step_power(param, *direction)?;
                cue_edit::set_fade_param(cue, slot, Some(stepped))
            }),
        }
    }

    fn apply_commit(&self, id: FieldId, text: &str, cue: &Cue) -> Option<Cue> {
        match id {
            NUMBER => return cue_edit::set_number(cue, text),
            NAME => return cue_edit::set_name(cue, text),
            PRE_WAIT => return cue_edit::set_pre_wait(cue, text),
            DURATION => return cue_edit::set_duration(cue, text),
            NOTES => return cue_edit::set_notes(cue, text),
            AUDIO_TARGET => return cue_edit::set_audio_target(cue, PathBuf::from(text)),
            VOLUME => {
                let current = cue_edit::level(cue)?;
                return match commit_volume(text, current) {
                    Commit::Value(value) => cue_edit::set_level(cue, value),
                    Commit::Unchanged | Commit::Null => None,
                };
            }
            START => return cue_edit::set_start_time(cue, text, self.file_duration),
            END => return cue_edit::set_end_time(cue, text, self.file_duration),
            PAN => return cue_edit::set_pan_text(cue, text),
            _ => {}
        }

        FADE_SLOTS.into_iter().find_map(|slot| {
            let (duration, power) = fade_fields(slot);
            let param = cue_edit::fade_param(cue, slot)?;
            let changed = if id == duration {
                cue_edit::fade_with_duration(param, text)
            } else if id == power {
                cue_edit::fade_with_power(param, text)
            } else {
                None
            }?;
            cue_edit::set_fade_param(cue, slot, Some(changed))
        })
    }

    /// The volume to show for an audio or fade cue: the dragged one while
    /// dragging.
    fn volume_for(&self, cue: &Cue) -> f64 {
        match self.drag_volume {
            Some((id, value)) if id == cue.id => value,
            _ => cue_edit::level(cue).unwrap_or(0.0),
        }
    }

    fn pan_for(&self, cue: &Cue) -> f64 {
        match self.drag_pan {
            Some((id, value)) if id == cue.id => value,
            _ => cue_edit::pan(cue).unwrap_or(0.0),
        }
    }

    /// Keeps the notes editor showing the selected cue's notes, unless
    /// they are being edited right now.
    pub fn sync(&mut self, cue: Option<&Cue>, drafts: &Drafts) {
        // The dragged volume is dropped once the backend's answer matches it
        // (or another cue is shown).
        if let Some((id, value)) = self.drag_volume {
            let stored = cue.filter(|cue| cue.id == id).and_then(cue_edit::level);
            if stored.is_none_or(|stored| (stored - value).abs() < 1e-3) {
                self.drag_volume = None;
            }
        }
        if let Some((id, value)) = self.drag_pan {
            let stored = cue.filter(|cue| cue.id == id).and_then(cue_edit::pan);
            if stored.is_none_or(|stored| (stored - value).abs() < 1e-3) {
                self.drag_pan = None;
            }
        }

        if drafts.is_editing(NOTES) {
            return;
        }
        let id = cue.map(|cue| cue.id);
        let wanted = cue.map(|cue| cue.notes.as_str()).unwrap_or("");
        if self.notes_cue != id
            || plain(self.notes.text()) != plain(wanted.to_string())
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
    /// The selected audio cue's file, as far as it has been analyzed.
    pub file: FileInfo,
    /// The loudness the "LUFS" button aims for.
    pub lufs_target: f64,
}

/// What the analysis of an audio cue's file says.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FileInfo {
    pub duration: Option<f64>,
    pub integrated_lufs: Option<f64>,
    /// dBFS.
    pub peak: Option<f64>,
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

    // A tab left over from another kind of cue gives way to Basics.
    let available = Tab::of(cue);
    let current = if available.contains(&editor.tab) {
        editor.tab
    } else {
        Tab::Basics
    };
    let tabs = row(available
        .iter()
        .map(|&tab| tab_button(current, tab, tab.label())))
    .spacing(1)
    .padding(1);

    let body = match current {
        Tab::Basics => basics(editor, &ctx, cue, flat),
        Tab::Trigger => trigger(cue),
        Tab::Audio => type_tabs::audio(&ctx, cue),
        Tab::Levels => type_tabs::levels(editor, &ctx, cue),
        Tab::Fade => type_tabs::fade(editor, &ctx, cue, flat),
        Tab::Playback => type_tabs::playback(&ctx, cue, flat),
        Tab::Group => type_tabs::group(&ctx, cue),
    };

    column![tabs, container(body).padding(8).width(Length::Fill).height(Length::Fill)]
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
            Some(&chain_kind),
            !locked,
            Message::ChainKind,
        ),
    );

    let left = column![number, duration, pre_wait, chain_mode]
        .spacing(4)
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
            Some(&cue_edit::chain_target(chain)),
            !locked && chain_kind != ChainKind::DoNotChain,
            Message::ChainTarget,
        ),
    );

    let colors = labeled(
        Some(&fl!("main-bottom-editor-basics-color")),
        picker(
            COLORS.iter().map(|&color| Choice::new(color, color_label(color))).collect(),
            Some(&cue.color),
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

    let right = column![name, notes, bottom].spacing(4).width(Length::Fill);

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
            Some(&cue.cursor_advance_trigger_override),
            true,
            Message::Advance,
        ),
    );

    column![
        container(advance).max_width(500.0),
        check(
            fl!("main-bottom-editor-trigger-treat-stop-as-completed"),
            cue.treat_stop_as_completed,
            Some(Message::TreatStop(!cue.treat_stop_as_completed)),
        ),
    ]
    .spacing(12)
    .into()
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
            editor.update(Message::Tab(Tab::Trigger), None, None, Instant::now()),
            Update::None
        ));
        assert_eq!(editor.tab, Tab::Trigger);
    }

    #[test]
    fn a_trigger_choice_becomes_an_edited_cue() {
        let mut editor = Editor::default();
        let cue = cue();

        match editor.update(Message::TreatStop(true), Some(&cue), None, Instant::now()) {
            Update::Edit(edited) => assert!(edited.treat_stop_as_completed),
            _ => panic!("expected an edit"),
        }
        assert!(matches!(
            editor.update(Message::TreatStop(false), Some(&cue), None, Instant::now()),
            Update::None
        ));
        assert!(matches!(
            editor.update(Message::TreatStop(true), None, None, Instant::now()),
            Update::None
        ));
    }

    #[test]
    fn a_timestamp_needs_a_position() {
        let mut editor = Editor::default();
        let cue = cue();

        assert!(matches!(
            editor.update(Message::Timestamp, Some(&cue), None, Instant::now()),
            Update::None
        ));
        match editor.update(Message::Timestamp, Some(&cue), Some(3.0), Instant::now()) {
            Update::Edit(edited) => assert_eq!(edited.notes, "memo\n[00:03.00] "),
            _ => panic!("expected an edit"),
        }
    }

    fn fade_cue() -> Cue {
        use sbsp_backend::model::cue::{FadeCueParam, audio::{Decibels, Easing, FadeParam}};
        let mut cue = cue();
        cue.params = CueParam::Fade(FadeCueParam {
            target: Uuid::from_u128(2),
            volume: Decibels::from(-6.0),
            fade_param: FadeParam {
                duration: 3.0,
                easing: Easing::InOutPow(2.0),
            },
        });
        cue
    }

    #[test]
    fn each_kind_of_cue_has_its_tabs() {
        assert_eq!(Tab::of(&cue()), vec![Tab::Basics, Tab::Trigger]);
        assert_eq!(
            Tab::of(&fade_cue()),
            vec![Tab::Basics, Tab::Trigger, Tab::Fade]
        );
    }

    #[test]
    fn fade_fields_edit_the_fade() {
        let editor = Editor::default();
        let cue = fade_cue();
        let (duration, power) = fade_fields(FadeSlot::Fade);

        let edited = editor.apply(&commit(duration, "5"), &cue).unwrap();
        assert_eq!(cue_edit::fade_param(&edited, FadeSlot::Fade).unwrap().duration, 5.0);

        assert!(editor.apply(&Outcome::Step(power, 1), &cue).is_some());
        assert!(editor.apply(&commit(duration, "3"), &cue).is_none(), "unchanged");
    }

    #[test]
    fn the_fade_volume_follows_the_drag_until_it_is_stored() {
        let mut editor = Editor::default();
        let cue = fade_cue();
        let now = Instant::now();

        let _ = editor.update(Message::Volume(FaderMessage::Moved(-12.0)), Some(&cue), None, now);
        assert_eq!(editor.volume_for(&cue), -12.0);

        match editor.update(Message::Volume(FaderMessage::Released), Some(&cue), None, now) {
            Update::Edit(edited) => assert_eq!(cue_edit::fade_volume(&edited), Some(-12.0)),
            _ => panic!("expected an edit"),
        }

        // The backend's answer arrives: the drag value is dropped.
        let mut stored = cue.clone();
        stored.params = cue_edit::set_fade_volume(&cue, -12.0).unwrap().params;
        editor.sync(Some(&stored), &Drafts::default());
        assert!(editor.drag_volume.is_none());
    }

    fn audio_cue() -> Cue {
        use sbsp_backend::model::cue::audio::{AudioCueParam, Decibels, SoundType};
        let mut cue = cue();
        cue.params = CueParam::Audio(AudioCueParam {
            target: PathBuf::from("/a.wav"),
            start_time: None,
            fade_in_param: None,
            end_time: None,
            fade_out_param: None,
            volume: Decibels::from(0.0),
            pan: 0.0,
            repeat: false,
            sound_type: SoundType::default(),
            envelope: Vec::new(),
        });
        cue
    }

    #[test]
    fn audio_cues_have_a_levels_tab() {
        assert_eq!(
            Tab::of(&audio_cue()),
            vec![Tab::Basics, Tab::Trigger, Tab::Audio, Tab::Levels]
        );
    }

    #[test]
    fn time_fields_use_the_files_duration() {
        let editor = Editor {
            file_duration: Some(10.0),
            ..Editor::default()
        };
        let cue = audio_cue();

        let edited = editor.apply(&commit(END, "4"), &cue).unwrap();
        assert_eq!(cue_edit::audio_end(&edited, Some(10.0)), Some(4.0));
        assert!(editor.apply(&commit(END, "10"), &cue).is_none(), "the duration is no end");
        let started = editor.apply(&commit(START, "1"), &cue).unwrap();
        assert_eq!(cue_edit::audio_start(&started), Some(1.0));
        assert_eq!(cue_edit::pan(&editor.apply(&commit(PAN, "0.5"), &cue).unwrap()), Some(0.5));
        assert_eq!(cue_edit::level(&editor.apply(&commit(VOLUME, "-6"), &cue).unwrap()), Some(-6.0));
    }

    #[test]
    fn the_pan_follows_the_drag_and_a_double_click_centers_it() {
        let mut editor = Editor::default();
        let mut cue = audio_cue();
        cue = cue_edit::set_pan(&cue, 0.5).unwrap();
        let now = Instant::now();

        let _ = editor.update(Message::Pan(FaderMessage::Moved(-0.25)), Some(&cue), None, now);
        assert_eq!(editor.pan_for(&cue), -0.25);
        match editor.update(Message::Pan(FaderMessage::Released), Some(&cue), None, now) {
            Update::Edit(edited) => assert_eq!(cue_edit::pan(&edited), Some(-0.25)),
            _ => panic!("expected an edit"),
        }
        // A quick second release resets to the center.
        match editor.update(Message::Pan(FaderMessage::Released), Some(&cue), None, now) {
            Update::Edit(edited) => assert_eq!(cue_edit::pan(&edited), Some(0.0)),
            _ => panic!("expected an edit"),
        }
    }

    #[test]
    fn repeat_and_volume_matching_are_requests() {
        let mut editor = Editor::default();
        let cue = audio_cue();
        let now = Instant::now();

        match editor.update(Message::Repeat(true), Some(&cue), None, now) {
            Update::Edit(edited) => assert!(matches!(&edited.params, CueParam::Audio(a) if a.repeat)),
            _ => panic!("expected an edit"),
        }
        assert!(matches!(
            editor.update(Message::VolumeToLufs, Some(&cue), None, now),
            Update::MatchVolume(Match::Lufs)
        ));
        assert!(matches!(
            editor.update(Message::VolumeToPeak, Some(&cue), None, now),
            Update::MatchVolume(Match::Peak)
        ));
    }

    #[test]
    fn picking_audio_asks_the_app() {
        let mut editor = Editor::default();
        let cue = cue();
        assert!(matches!(
            editor.update(Message::PickAudio, Some(&cue), None, Instant::now()),
            Update::PickAudio(id) if id == cue.id
        ));
    }

    #[test]
    fn only_its_own_fields_are_owned() {
        assert!(owns(NOTES));
        assert!(!owns(FieldId("gallery.number")));
    }
}
