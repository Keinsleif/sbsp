// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The settings dialog. Ported from `SettingsDialog.vue`.
//!
//! The dialog edits a copy of the settings (the show's and the global
//! ones); Done hands the copy back to the app, Cancel throws it away.
//! Tabs arrive one at a time: this has the presets, the show's own
//! settings, the global general and appearance settings, the hotkeys and
//! the templates and the name formats. The audio hardware comes with its
//! own step.

mod hardware_tab;
mod hotkey_tab;
mod template_tab;

pub use hotkey_tab::Slot;
pub use template_tab::{Editing, Kind as TemplateKind};

use std::fmt;

use iced::widget::{
    Space, button, checkbox, column, container, pick_list, row, scrollable, text,
};
use iced::{Alignment, Element, Length, Theme};
use sbsp_backend::model::settings::{CursorAdvanceTrigger, ShowSettings};
use crate::settings::{AudioHardwareSettings, DarkMode};

use crate::domain::field::{Commit, NumberSpec, commit_number, step_number};
use crate::fl;
use crate::state::preferences::Preferences;
use crate::widgets::field::{self, Drafts, FieldId, Outcome, text_field};
use crate::widgets::inputs::{NumberInput, labeled, number_input};
use crate::widgets::divider;

const SHOW_NAME: FieldId = FieldId("settings.show-name");
const ASSETS_DIRECTORY: FieldId = FieldId("settings.assets-directory");
const LUFS_TARGET: FieldId = FieldId("settings.lufs-target");
const SEEK_AMOUNT: FieldId = FieldId("settings.seek-amount");

const FORMAT_AUDIO: FieldId = FieldId("settings.format.audio");
const FORMAT_WAIT: FieldId = FieldId("settings.format.wait");
const FORMAT_FADE: FieldId = FieldId("settings.format.fade");
const FORMAT_START: FieldId = FieldId("settings.format.start");
const FORMAT_STOP: FieldId = FieldId("settings.format.stop");
const FORMAT_PAUSE: FieldId = FieldId("settings.format.pause");
const FORMAT_LOAD: FieldId = FieldId("settings.format.load");
const FORMAT_GROUP: FieldId = FieldId("settings.format.group");

const ALL_FIELDS: [FieldId; 12] = [
    SHOW_NAME,
    ASSETS_DIRECTORY,
    LUFS_TARGET,
    SEEK_AMOUNT,
    FORMAT_AUDIO,
    FORMAT_WAIT,
    FORMAT_FADE,
    FORMAT_START,
    FORMAT_STOP,
    FORMAT_PAUSE,
    FORMAT_LOAD,
    FORMAT_GROUP,
];

pub fn owns(id: FieldId) -> bool {
    ALL_FIELDS.contains(&id)
}

fn lufs_spec() -> NumberSpec {
    NumberSpec {
        precision: 2,
        ..NumberSpec::default()
    }
}

fn seek_spec() -> NumberSpec {
    NumberSpec {
        min: Some(0.0),
        precision: 2,
        ..NumberSpec::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Preset,
    ShowGeneral,
    AudioLogic,
    GlobalGeneral,
    Appearance,
    AudioHardware,
    Hotkey,
    Template,
    NameFormat,
}

/// Which hotkey layout a preset button applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    MusicBee,
    QLab,
}

/// A message of the editor under the template list. (`editor::Message`
/// has no `Debug`.)
#[derive(Clone)]
pub struct EditorMessage(pub crate::features::editor::Message);

impl fmt::Debug for EditorMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EditorMessage(..)")
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Field(field::Message),
    SelectTab(Tab),
    Preset(Preset),
    CursorAdvance(CursorAdvanceTrigger),
    MonoOutput(bool),
    LockCursor(bool),
    CopyAssets(bool),
    Language(Option<String>),
    DarkMode(DarkMode),
    HideControls(bool),
    /// A hotkey input was pressed: it waits for a key (or stops waiting).
    Capture(hotkey_tab::Slot),
    ClearHotkey(hotkey_tab::Slot),
    SelectTemplate(TemplateKind),
    Device(Option<String>),
    ChannelCount(Option<u16>),
    SampleRate(Option<u32>),
    BufferSize(Option<u32>),
    Editor(EditorMessage),
    Cancel,
    Done,
}

/// The settings being edited.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsDialog {
    pub tab: Tab,
    pub show_name: String,
    pub show: ShowSettings,
    pub preferences: Preferences,
    /// The hotkey input waiting for a key press.
    pub capturing: Option<hotkey_tab::Slot>,
    /// The template shown in the editor.
    pub template: Option<TemplateKind>,
    /// The audio hardware settings (the host has them, a remote does not).
    pub audio: Option<AudioHardwareSettings>,
}

impl SettingsDialog {
    pub fn new(show_name: &str, show: &ShowSettings, preferences: &Preferences) -> Self {
        Self {
            tab: Tab::ShowGeneral,
            show_name: show_name.to_owned(),
            show: show.clone(),
            preferences: preferences.clone(),
            capturing: None,
            template: None,
            audio: None,
        }
    }

    /// Adds the audio hardware settings (and so their tab).
    pub fn with_audio(mut self, audio: AudioHardwareSettings) -> Self {
        self.audio = Some(audio);
        self
    }

    /// The template being edited.
    pub fn template_cue(&self) -> Option<&sbsp_backend::model::cue::Cue> {
        self.template
            .map(|kind| kind.get(&self.preferences.template))
    }

    /// Replaces the template being edited.
    pub fn set_template_cue(&mut self, cue: sbsp_backend::model::cue::Cue) {
        if let Some(kind) = self.template {
            kind.set(&mut self.preferences.template, cue);
        }
    }

    /// The binding the next key press was turned into, for the input that
    /// is waiting for one.
    pub fn capture(&mut self, binding: String) {
        if let Some(slot) = self.capturing.take() {
            slot.set(&mut self.preferences.hotkey, Some(binding));
        }
    }

    /// A finished edit or a step button, if it belongs to this dialog.
    pub fn apply(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Commit(commit) => {
                let text = commit.text.as_str();
                match commit.field {
                    SHOW_NAME => self.show_name = text.to_owned(),
                    ASSETS_DIRECTORY => self.show.general.copy_assets_destination = text.to_owned(),
                    LUFS_TARGET => {
                        if let Commit::Value(value) =
                            commit_number(text, Some(self.show.audio.lufs_target), &lufs_spec())
                        {
                            self.show.audio.lufs_target = value;
                        }
                    }
                    SEEK_AMOUNT => {
                        if let Commit::Value(value) = commit_number(
                            text,
                            Some(self.preferences.general.seek_amount),
                            &seek_spec(),
                        ) {
                            self.preferences.general.seek_amount = value;
                        }
                    }
                    other => {
                        if let Some(format) = self.name_format(other) {
                            *format = text.to_owned();
                        }
                    }
                }
            }
            Outcome::Step(id, direction) => match *id {
                SEEK_AMOUNT => {
                    self.preferences.general.seek_amount = step_number(
                        Some(self.preferences.general.seek_amount),
                        &seek_spec(),
                        *direction,
                    );
                }
                LUFS_TARGET => {
                    self.show.audio.lufs_target = step_number(
                        Some(self.show.audio.lufs_target),
                        &lufs_spec(),
                        *direction,
                    );
                }
                _ => {}
            },
        }
    }

    /// Everything but the text fields (which arrive as [`Outcome`]s).
    /// `Cancel` and `Done` are the owner's business.
    pub fn update(&mut self, message: &Message) {
        match message {
            Message::SelectTab(tab) => {
                self.tab = *tab;
                self.capturing = None;
            }
            Message::Capture(slot) => {
                self.capturing = (self.capturing != Some(*slot)).then_some(*slot);
            }
            Message::SelectTemplate(kind) => self.template = Some(*kind),
            Message::Device(id) => self.audio_mut(|audio| audio.device_id = id.clone()),
            Message::ChannelCount(count) => self.audio_mut(|audio| audio.channel_count = *count),
            Message::SampleRate(rate) => self.audio_mut(|audio| audio.sample_rate = *rate),
            Message::BufferSize(size) => self.audio_mut(|audio| audio.buffer_size = *size),
            Message::ClearHotkey(slot) => {
                slot.set(&mut self.preferences.hotkey, None);
                self.capturing = None;
            }
            Message::Preset(preset) => self.recall(*preset),
            Message::CursorAdvance(trigger) => self.show.general.cursor_advance_trigger = *trigger,
            Message::MonoOutput(on) => self.show.audio.mono_output = *on,
            Message::LockCursor(on) => self.preferences.general.lock_cursor_to_selection = *on,
            Message::CopyAssets(on) => self.preferences.general.copy_assets_when_add = *on,
            Message::Language(language) => self.preferences.appearance.language = language.clone(),
            Message::DarkMode(mode) => self.preferences.appearance.dark_mode = *mode,
            Message::HideControls(on) => self.preferences.appearance.hide_controls = *on,
            Message::Field(_) | Message::Editor(_) | Message::Cancel | Message::Done => {}
        }
    }

    /// The hotkeys of another cue player's layout. Only the playback keys
    /// and the repeat toggle change, as in the original.
    fn recall(&mut self, preset: Preset) {
        use crate::settings::hotkey::{AudioActionHotkey, PlaybackHotkey};

        let key = |name: &str| Some(name.to_owned());
        let (execute, pause_and_resume, stop) = match preset {
            Preset::MusicBee => ("Enter", "Space", "Backspace"),
            Preset::QLab => ("Space", "P", "S"),
        };
        self.preferences.hotkey.playback = PlaybackHotkey {
            execute: key(execute),
            load: key("L"),
            pause_and_resume: key(pause_and_resume),
            pause_all: key("["),
            resume_all: key("]"),
            stop: key(stop),
            fade_out: None,
            hard_stop: None,
            stop_all: key("Escape"),
            fade_all_out: None,
            hard_stop_all: None,
            seek_forward: None,
            seek_backward: None,
        };
        self.preferences.hotkey.audio_action = AudioActionHotkey {
            toggle_repeat: key("R"),
        };
    }

    fn audio_mut(&mut self, change: impl FnOnce(&mut AudioHardwareSettings)) {
        if let Some(audio) = &mut self.audio {
            change(audio);
        }
    }

    fn name_format(&mut self, id: FieldId) -> Option<&mut String> {
        let formats = &mut self.preferences.name_format;
        match id {
            FORMAT_AUDIO => Some(&mut formats.audio),
            FORMAT_WAIT => Some(&mut formats.wait),
            FORMAT_FADE => Some(&mut formats.fade),
            FORMAT_START => Some(&mut formats.start),
            FORMAT_STOP => Some(&mut formats.stop),
            FORMAT_PAUSE => Some(&mut formats.pause),
            FORMAT_LOAD => Some(&mut formats.load),
            FORMAT_GROUP => Some(&mut formats.group),
            _ => None,
        }
    }
}

/// A value with the label a `pick_list` shows. Equality is on the value
/// alone, so the selection survives a language switch.
#[derive(Clone)]
struct Choice<T> {
    value: T,
    label: String,
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

fn choice<T>(value: T, label: impl Into<String>) -> Choice<T> {
    Choice {
        value,
        label: label.into(),
    }
}

/// A `pick_list` over `options`, with the one equal to `current` selected.
fn picker<'a, T: PartialEq + Clone + 'a>(
    options: Vec<Choice<T>>,
    current: &T,
    on_pick: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message> {
    let selected = options.iter().find(|option| option.value == *current).cloned();
    pick_list(options, selected, move |option: Choice<T>| on_pick(option.value))
        .width(Length::Fill)
        .into()
}

fn muted<'a>(content: String) -> Element<'a, Message> {
    text(content)
        .size(12)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.extended_palette().background.base.text.scale_alpha(0.7)),
        })
        .into()
}

fn tab_label(tab: Tab) -> String {
    match tab {
        Tab::Preset => fl!("dialog-settings-tab-preset"),
        Tab::ShowGeneral | Tab::GlobalGeneral => fl!("dialog-settings-tab-general"),
        Tab::AudioLogic => fl!("dialog-settings-tab-audio-logic"),
        Tab::Appearance => fl!("dialog-settings-tab-appearance"),
        Tab::AudioHardware => fl!("dialog-settings-tab-audio-hardware"),
        Tab::Hotkey => fl!("dialog-settings-tab-hotkey"),
        Tab::Template => fl!("dialog-settings-tab-template"),
        Tab::NameFormat => fl!("dialog-settings-tab-name-format"),
    }
}

pub fn view<'a>(
    dialog: &'a SettingsDialog,
    drafts: &'a Drafts,
    editing: Editing<'a, '_>,
) -> Element<'a, Message> {
    let tab_button = |tab: Tab| {
        button(text(tab_label(tab)).width(Length::Fill))
            .width(Length::Fill)
            .padding([3, 8])
            .style(if dialog.tab == tab {
                button::secondary
            } else {
                button::text
            })
            .on_press(Message::SelectTab(tab))
    };
    let heading = |label: String| {
        container(muted(label))
            .padding([6, 8])
            .width(Length::Fill)
    };

    let hardware_button: Element<'a, Message> = if dialog.audio.is_some() {
        tab_button(Tab::AudioHardware).into()
    } else {
        Space::new().into()
    };

    let tabs = column![
        tab_button(Tab::Preset),
        heading(fl!("dialog-settings-tab-category-in-this-show-model")),
        tab_button(Tab::ShowGeneral),
        tab_button(Tab::AudioLogic),
        heading(fl!("dialog-settings-tab-category-global")),
        tab_button(Tab::GlobalGeneral),
        tab_button(Tab::Appearance),
        hardware_button,
        tab_button(Tab::Hotkey),
        tab_button(Tab::Template),
        tab_button(Tab::NameFormat),
    ]
    .spacing(2)
    .width(200);

    let page = match dialog.tab {
        Tab::Preset => preset_page(),
        Tab::ShowGeneral => show_general_page(dialog, drafts),
        Tab::AudioLogic => audio_logic_page(dialog, drafts),
        Tab::GlobalGeneral => global_general_page(dialog, drafts),
        Tab::Appearance => appearance_page(dialog),
        Tab::AudioHardware => match &dialog.audio {
            Some(audio) => hardware_tab::view(editing.hardware, audio),
            None => Space::new().into(),
        },
        Tab::Hotkey => hotkey_tab::view(&dialog.preferences.hotkey, dialog.capturing),
        Tab::Template => template_tab::view(
            &dialog.preferences.template,
            dialog.template,
            &dialog.preferences.name_format,
            drafts,
            editing,
        ),
        Tab::NameFormat => name_format_page(dialog, drafts),
    };

    let buttons = row![
        Space::new().width(Length::Fill),
        button(text(fl!("general-cancel")))
            .style(button::secondary)
            .on_press(Message::Cancel),
        button(text(fl!("general-done"))).on_press(Message::Done),
    ]
    .spacing(8);

    // A fixed size: the dialog is centred by the modal, which gives its
    // content only what it asks for.
    container(column![
        row![
            tabs,
            divider::vertical(),
            scrollable(container(page).padding([0, 16]).width(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(8)
        .height(Length::Fill),
        buttons,
    ]
    .spacing(12))
    .width(960)
    .height(640)
    .into()
}

fn preset_page<'a>() -> Element<'a, Message> {
    let line = |label: &'static str, description: String, preset| {
        row![
            button(text(label).width(Length::Fill))
                .width(120)
                .on_press(Message::Preset(preset)),
            text(description),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
    };

    column![
        text(fl!("dialog-settings-preset-hotkey-cursor-title")).size(18),
        line(
            "MusicBee",
            fl!("dialog-settings-preset-hotkey-cursor-music-bee-description"),
            Preset::MusicBee
        ),
        line(
            "QLab",
            fl!("dialog-settings-preset-hotkey-cursor-q-lab-description"),
            Preset::QLab
        ),
    ]
    .spacing(16)
    .into()
}

fn text_row<'a>(
    id: FieldId,
    label: String,
    shown: &str,
    help: Option<String>,
    drafts: &Drafts,
) -> Element<'a, Message> {
    let input = labeled(
        Some(label.as_str()),
        text_field(id, shown, drafts, true).width(Length::Fill),
    )
    .map(Message::Field);

    match help {
        Some(help) => column![input, muted(help)].spacing(2).into(),
        None => input,
    }
}

fn show_general_page<'a>(dialog: &SettingsDialog, drafts: &Drafts) -> Element<'a, Message> {
    let general = &dialog.show.general;
    let triggers = vec![
        choice(
            CursorAdvanceTrigger::OnTriggered,
            fl!("dialog-settings-show-general-cursor-advance-trigger-on-triggered"),
        ),
        choice(
            CursorAdvanceTrigger::OnCompleted,
            fl!("dialog-settings-show-general-cursor-advance-trigger-on-completed"),
        ),
        choice(
            CursorAdvanceTrigger::Manual,
            fl!("dialog-settings-show-general-cursor-advance-trigger-manual"),
        ),
    ];

    column![
        text_row(
            SHOW_NAME,
            fl!("dialog-settings-show-general-show-model-name"),
            &dialog.show_name,
            None,
            drafts,
        ),
        labeled(
            Some(fl!("dialog-settings-show-general-cursor-advance-trigger-title").as_str()),
            picker(triggers, &general.cursor_advance_trigger, Message::CursorAdvance),
        ),
        text_row(
            ASSETS_DIRECTORY,
            fl!("dialog-settings-show-general-assets-directory-title"),
            &general.copy_assets_destination,
            Some(fl!("dialog-settings-show-general-assets-directory-description")),
            drafts,
        ),
    ]
    .spacing(16)
    .max_width(500)
    .into()
}

fn audio_logic_page<'a>(dialog: &SettingsDialog, drafts: &Drafts) -> Element<'a, Message> {
    let audio = &dialog.show.audio;

    column![
        checkbox(audio.mono_output)
            .label(fl!("dialog-settings-show-audio-logic-mono-output"))
            .on_toggle(Message::MonoOutput),
        number_input(
            NumberInput {
                id: LUFS_TARGET,
                label: Some(fl!("dialog-settings-show-audio-logic-target-lufs").as_str()),
                prefix: None,
                suffix: Some("LUFS".to_owned()),
                value: Some(audio.lufs_target),
                spec: lufs_spec(),
                show_buttons: false,
                enabled: true,
            },
            drafts,
        )
        .map(Message::Field),
    ]
    .spacing(16)
    .max_width(320)
    .into()
}

fn global_general_page<'a>(dialog: &SettingsDialog, drafts: &Drafts) -> Element<'a, Message> {
    let general = &dialog.preferences.general;

    column![
        checkbox(general.lock_cursor_to_selection)
            .label(fl!("dialog-settings-global-general-lock-cursor-to-selection"))
            .on_toggle(Message::LockCursor),
        checkbox(general.copy_assets_when_add)
            .label(fl!("dialog-settings-global-general-copy-assets-when-add"))
            .on_toggle(Message::CopyAssets),
        container(number_input(
            NumberInput {
                id: SEEK_AMOUNT,
                label: Some(fl!("dialog-settings-global-general-seek-amount").as_str()),
                prefix: None,
                suffix: None,
                value: Some(general.seek_amount),
                spec: seek_spec(),
                show_buttons: true,
                enabled: true,
            },
            drafts,
        )
        .map(Message::Field)).max_width(320),
    ]
    .spacing(16)
    .into()
}

fn appearance_page<'a>(dialog: &SettingsDialog) -> Element<'a, Message> {
    let appearance = &dialog.preferences.appearance;
    let languages = vec![
        choice(None, fl!("dialog-settings-global-appearance-system-language")),
        choice(Some("en".to_owned()), "English"),
        choice(Some("ja".to_owned()), "日本語"),
    ];
    let modes = vec![
        choice(
            DarkMode::System,
            fl!("dialog-settings-global-appearance-system-settings"),
        ),
        choice(DarkMode::Dark, "Dark"),
        choice(DarkMode::Light, "Light"),
    ];

    column![
        labeled(
            Some(fl!("dialog-settings-global-appearance-language").as_str()),
            picker(languages, &appearance.language, Message::Language),
        ),
        labeled(
            Some(fl!("dialog-settings-global-appearance-dark-mode").as_str()),
            picker(modes, &appearance.dark_mode, Message::DarkMode),
        ),
        checkbox(appearance.hide_controls)
            .label(fl!("dialog-settings-global-appearance-hide-controls"))
            .on_toggle(Message::HideControls),
    ]
    .spacing(16)
    .max_width(320)
    .into()
}

fn name_format_page<'a>(dialog: &SettingsDialog, drafts: &Drafts) -> Element<'a, Message> {
    let formats = &dialog.preferences.name_format;
    let playback = || fl!("dialog-settings-global-name-format-playback-description");

    column![
        text_row(
            FORMAT_AUDIO,
            fl!("dialog-settings-global-name-format-audio-title"),
            &formats.audio,
            Some(fl!("dialog-settings-global-name-format-audio-description")),
            drafts,
        ),
        text_row(
            FORMAT_WAIT,
            fl!("dialog-settings-global-name-format-wait-title"),
            &formats.wait,
            Some(fl!("dialog-settings-global-name-format-wait-description")),
            drafts,
        ),
        text_row(
            FORMAT_FADE,
            fl!("dialog-settings-global-name-format-fade-title"),
            &formats.fade,
            Some(fl!("dialog-settings-global-name-format-fade-description")),
            drafts,
        ),
        text_row(
            FORMAT_START,
            fl!("dialog-settings-global-name-format-start-title"),
            &formats.start,
            Some(playback()),
            drafts,
        ),
        text_row(
            FORMAT_STOP,
            fl!("dialog-settings-global-name-format-stop-title"),
            &formats.stop,
            Some(playback()),
            drafts,
        ),
        text_row(
            FORMAT_PAUSE,
            fl!("dialog-settings-global-name-format-pause-title"),
            &formats.pause,
            Some(playback()),
            drafts,
        ),
        text_row(
            FORMAT_LOAD,
            fl!("dialog-settings-global-name-format-load-title"),
            &formats.load,
            Some(playback()),
            drafts,
        ),
        text_row(
            FORMAT_GROUP,
            fl!("dialog-settings-global-name-format-group-title"),
            &formats.group,
            Some(fl!("dialog-settings-global-name-format-group-description")),
            drafts,
        ),
    ]
    .spacing(14)
    .max_width(500)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::field::Commit as FieldCommit;

    fn dialog() -> SettingsDialog {
        SettingsDialog::new("Show", &ShowSettings::default(), &Preferences::default())
    }

    fn commit(field: FieldId, text: &str) -> Outcome {
        Outcome::Commit(FieldCommit {
            field,
            text: text.to_owned(),
        })
    }

    #[test]
    fn the_dialog_starts_from_the_current_settings() {
        let mut preferences = Preferences::default();
        preferences.general.seek_amount = 3.0;
        let dialog = SettingsDialog::new("My show", &ShowSettings::default(), &preferences);

        assert_eq!(dialog.show_name, "My show");
        assert_eq!(dialog.preferences.general.seek_amount, 3.0);
        assert_eq!(dialog.tab, Tab::ShowGeneral);
    }

    #[test]
    fn text_fields_are_taken_as_typed() {
        let mut d = dialog();
        d.apply(&commit(SHOW_NAME, "Evening"));
        d.apply(&commit(ASSETS_DIRECTORY, "assets/audio"));
        d.apply(&commit(FORMAT_AUDIO, "{filename}!"));

        assert_eq!(d.show_name, "Evening");
        assert_eq!(d.show.general.copy_assets_destination, "assets/audio");
        assert_eq!(d.preferences.name_format.audio, "{filename}!");
    }

    #[test]
    fn numbers_are_rounded_and_limited() {
        let mut d = dialog();
        d.apply(&commit(SEEK_AMOUNT, "-4"));
        assert_eq!(d.preferences.general.seek_amount, 0.0, "not below 0");

        d.apply(&commit(SEEK_AMOUNT, "2.456"));
        assert_eq!(d.preferences.general.seek_amount, 2.46);

        d.apply(&commit(LUFS_TARGET, "-23"));
        assert_eq!(d.show.audio.lufs_target, -23.0);
        d.apply(&commit(LUFS_TARGET, "loud"));
        assert_eq!(d.show.audio.lufs_target, -23.0, "unusable text changes nothing");
    }

    #[test]
    fn the_step_buttons_move_by_one() {
        let mut d = dialog();
        let before = d.preferences.general.seek_amount;
        d.apply(&Outcome::Step(SEEK_AMOUNT, 1));
        assert_eq!(d.preferences.general.seek_amount, before + 1.0);
        d.apply(&Outcome::Step(SEEK_AMOUNT, -1));
        d.apply(&Outcome::Step(SEEK_AMOUNT, -1));
        d.apply(&Outcome::Step(SEEK_AMOUNT, -1));
        d.apply(&Outcome::Step(SEEK_AMOUNT, -1));
        d.apply(&Outcome::Step(SEEK_AMOUNT, -1));
        d.apply(&Outcome::Step(SEEK_AMOUNT, -1));
        d.apply(&Outcome::Step(SEEK_AMOUNT, -1));
        d.apply(&Outcome::Step(SEEK_AMOUNT, -1));
        assert!(d.preferences.general.seek_amount >= 0.0);
    }

    #[test]
    fn choices_and_checkboxes_land_in_the_right_place() {
        let mut d = dialog();
        d.update(&Message::CursorAdvance(CursorAdvanceTrigger::OnCompleted));
        d.update(&Message::MonoOutput(true));
        d.update(&Message::LockCursor(false));
        d.update(&Message::CopyAssets(true));
        d.update(&Message::Language(Some("ja".to_owned())));
        d.update(&Message::DarkMode(DarkMode::Light));
        d.update(&Message::HideControls(true));
        d.update(&Message::SelectTab(Tab::Appearance));

        assert_eq!(d.show.general.cursor_advance_trigger, CursorAdvanceTrigger::OnCompleted);
        assert!(d.show.audio.mono_output);
        assert!(!d.preferences.general.lock_cursor_to_selection);
        assert!(d.preferences.general.copy_assets_when_add);
        assert_eq!(d.preferences.appearance.language.as_deref(), Some("ja"));
        assert_eq!(d.preferences.appearance.dark_mode, DarkMode::Light);
        assert!(d.preferences.appearance.hide_controls);
        assert_eq!(d.tab, Tab::Appearance);
    }

    #[test]
    fn presets_replace_the_playback_keys_only() {
        let mut d = dialog();
        let file_before = d.preferences.hotkey.file.clone();
        let edit_before = d.preferences.hotkey.edit.clone();

        d.update(&Message::Preset(Preset::QLab));
        let playback = &d.preferences.hotkey.playback;
        assert_eq!(playback.execute.as_deref(), Some("Space"));
        assert_eq!(playback.stop.as_deref(), Some("S"));
        assert_eq!(playback.seek_forward, None);
        assert_eq!(d.preferences.hotkey.file, file_before);
        assert_eq!(d.preferences.hotkey.edit, edit_before);

        d.update(&Message::Preset(Preset::MusicBee));
        let playback = &d.preferences.hotkey.playback;
        assert_eq!(playback.execute.as_deref(), Some("Enter"));
        assert_eq!(playback.pause_and_resume.as_deref(), Some("Space"));
        assert_eq!(playback.stop.as_deref(), Some("Backspace"));
        assert_eq!(d.preferences.hotkey.audio_action.toggle_repeat.as_deref(), Some("R"));
    }

    #[test]
    fn only_its_own_fields_are_taken() {
        let mut d = dialog();
        d.apply(&commit(FieldId("editor.number"), "7"));
        assert_eq!(d, dialog());
        assert!(owns(SHOW_NAME) && !owns(FieldId("editor.number")));
    }

    #[test]
    fn a_hotkey_input_takes_the_next_press_once() {
        use hotkey_tab::Slot;

        let mut d = dialog();
        d.update(&Message::SelectTab(Tab::Hotkey));
        d.update(&Message::Capture(Slot::Load));
        assert_eq!(d.capturing, Some(Slot::Load));

        d.capture("$mod+K".to_owned());
        assert_eq!(d.preferences.hotkey.playback.load.as_deref(), Some("$mod+K"));
        assert_eq!(d.capturing, None);

        d.capture("X".to_owned());
        assert_eq!(d.preferences.hotkey.playback.load.as_deref(), Some("$mod+K"), "not waiting");
    }

    #[test]
    fn pressing_an_input_twice_stops_waiting() {
        use hotkey_tab::Slot;

        let mut d = dialog();
        d.update(&Message::Capture(Slot::Stop));
        d.update(&Message::Capture(Slot::Stop));
        assert_eq!(d.capturing, None);

        d.update(&Message::Capture(Slot::Stop));
        d.update(&Message::SelectTab(Tab::Appearance));
        assert_eq!(d.capturing, None, "leaving the tab stops it too");
    }

    #[test]
    fn the_cross_unbinds() {
        use hotkey_tab::Slot;

        let mut d = dialog();
        d.update(&Message::ClearHotkey(Slot::StopAll));
        assert_eq!(d.preferences.hotkey.playback.stop_all, None);
    }
}
