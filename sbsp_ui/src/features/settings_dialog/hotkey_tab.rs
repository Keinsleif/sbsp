// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The hotkey tab: one input per binding. Ported from `HotkeyInput.vue` and
//! the hotkey part of `SettingsDialog.vue`.
//!
//! An input is a button: pressing it makes the next key press its binding
//! (Escape included, since Stop All is bound to it by default), pressing
//! it again or switching tab stops waiting. The cross button unbinds.

use iced::widget::{Space, button, column, container, row, text, tooltip};
use iced::{Alignment, Element, Length};
use sbsp_frontend_settings::hotkey::HotkeySettings;

use super::Message;
use crate::domain::hotkey::{Hotkey, normalize_hotkey};
use crate::features::hotkeys::is_mac;
use crate::fl;
use crate::widgets::icon::{self, Icon, Tone};

/// One binding in the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    FileOpen,
    FileSave,
    FileSaveAs,
    FileExport,
    EditDelete,
    EditSelectAll,
    EditRenumber,
    MoveUp,
    ExtendUp,
    MoveDown,
    ExtendDown,
    Execute,
    Load,
    PauseAndResume,
    PauseAll,
    ResumeAll,
    Stop,
    StopAll,
    SeekForward,
    SeekBackward,
    ToggleRepeat,
}

const FILE: [Slot; 4] = [Slot::FileOpen, Slot::FileSave, Slot::FileSaveAs, Slot::FileExport];
const EDIT: [Slot; 7] = [
    Slot::EditDelete,
    Slot::EditSelectAll,
    Slot::EditRenumber,
    Slot::MoveUp,
    Slot::ExtendUp,
    Slot::MoveDown,
    Slot::ExtendDown,
];
const PLAYBACK: [Slot; 9] = [
    Slot::Execute,
    Slot::Load,
    Slot::PauseAndResume,
    Slot::PauseAll,
    Slot::ResumeAll,
    Slot::Stop,
    Slot::StopAll,
    Slot::SeekForward,
    Slot::SeekBackward,
];
const AUDIO: [Slot; 1] = [Slot::ToggleRepeat];

impl Slot {
    pub fn get(self, settings: &HotkeySettings) -> &Option<String> {
        let (file, edit, playback) = (&settings.file, &settings.edit, &settings.playback);
        match self {
            Slot::FileOpen => &file.open,
            Slot::FileSave => &file.save,
            Slot::FileSaveAs => &file.save_as,
            Slot::FileExport => &file.export_to_folder,
            Slot::EditDelete => &edit.delete,
            Slot::EditSelectAll => &edit.select_all,
            Slot::EditRenumber => &edit.renumber_cues,
            Slot::MoveUp => &edit.cuelist_move_up,
            Slot::ExtendUp => &edit.cuelist_extend_up,
            Slot::MoveDown => &edit.cuelist_move_down,
            Slot::ExtendDown => &edit.cuelist_extend_down,
            Slot::Execute => &playback.execute,
            Slot::Load => &playback.load,
            Slot::PauseAndResume => &playback.pause_and_resume,
            Slot::PauseAll => &playback.pause_all,
            Slot::ResumeAll => &playback.resume_all,
            Slot::Stop => &playback.stop,
            Slot::StopAll => &playback.stop_all,
            Slot::SeekForward => &playback.seek_forward,
            Slot::SeekBackward => &playback.seek_backward,
            Slot::ToggleRepeat => &settings.audio_action.toggle_repeat,
        }
    }

    pub fn set(self, settings: &mut HotkeySettings, binding: Option<String>) {
        let (file, edit, playback) = (&mut settings.file, &mut settings.edit, &mut settings.playback);
        let target = match self {
            Slot::FileOpen => &mut file.open,
            Slot::FileSave => &mut file.save,
            Slot::FileSaveAs => &mut file.save_as,
            Slot::FileExport => &mut file.export_to_folder,
            Slot::EditDelete => &mut edit.delete,
            Slot::EditSelectAll => &mut edit.select_all,
            Slot::EditRenumber => &mut edit.renumber_cues,
            Slot::MoveUp => &mut edit.cuelist_move_up,
            Slot::ExtendUp => &mut edit.cuelist_extend_up,
            Slot::MoveDown => &mut edit.cuelist_move_down,
            Slot::ExtendDown => &mut edit.cuelist_extend_down,
            Slot::Execute => &mut playback.execute,
            Slot::Load => &mut playback.load,
            Slot::PauseAndResume => &mut playback.pause_and_resume,
            Slot::PauseAll => &mut playback.pause_all,
            Slot::ResumeAll => &mut playback.resume_all,
            Slot::Stop => &mut playback.stop,
            Slot::StopAll => &mut playback.stop_all,
            Slot::SeekForward => &mut playback.seek_forward,
            Slot::SeekBackward => &mut playback.seek_backward,
            Slot::ToggleRepeat => &mut settings.audio_action.toggle_repeat,
        };
        *target = binding;
    }

    fn label(self) -> String {
        match self {
            Slot::FileOpen => fl!("dialog-settings-global-hotkey-file-open"),
            Slot::FileSave => fl!("dialog-settings-global-hotkey-file-save"),
            Slot::FileSaveAs => fl!("dialog-settings-global-hotkey-file-save-as"),
            Slot::FileExport => fl!("dialog-settings-global-hotkey-file-export-to-folder"),
            Slot::EditDelete => fl!("dialog-settings-global-hotkey-edit-delete"),
            Slot::EditSelectAll => fl!("dialog-settings-global-hotkey-edit-select-all"),
            Slot::EditRenumber => fl!("dialog-settings-global-hotkey-edit-renumber-cues"),
            Slot::MoveUp => fl!("dialog-settings-global-hotkey-edit-cuelist-move-up"),
            Slot::ExtendUp => fl!("dialog-settings-global-hotkey-edit-cuelist-extend-up"),
            Slot::MoveDown => fl!("dialog-settings-global-hotkey-edit-cuelist-move-down"),
            Slot::ExtendDown => fl!("dialog-settings-global-hotkey-edit-cuelist-extend-down"),
            Slot::Execute => fl!("dialog-settings-global-hotkey-playback-execute"),
            Slot::Load => fl!("dialog-settings-global-hotkey-playback-load"),
            Slot::PauseAndResume => fl!("dialog-settings-global-hotkey-playback-pause-and-resume"),
            Slot::PauseAll => fl!("dialog-settings-global-hotkey-playback-pause-all"),
            Slot::ResumeAll => fl!("dialog-settings-global-hotkey-playback-resume-all"),
            Slot::Stop => fl!("dialog-settings-global-hotkey-playback-stop"),
            Slot::StopAll => fl!("dialog-settings-global-hotkey-playback-stop-all"),
            Slot::SeekForward => fl!("dialog-settings-global-hotkey-playback-seek-forward"),
            Slot::SeekBackward => fl!("dialog-settings-global-hotkey-playback-seek-backward"),
            Slot::ToggleRepeat => fl!("dialog-settings-global-hotkey-audio-toggle-repeat"),
        }
    }
}

/// Every binding there is, in the canonical form that makes equal
/// shortcuts compare equal.
fn all_normalized(settings: &HotkeySettings) -> Vec<String> {
    [FILE.as_slice(), &EDIT, &PLAYBACK, &AUDIO]
        .into_iter()
        .flatten()
        .filter_map(|slot| slot.get(settings).as_deref())
        .map(normalize_hotkey)
        .collect()
}

/// Whether another binding has the same keys.
pub fn is_duplicate(settings: &HotkeySettings, slot: Slot) -> bool {
    let Some(binding) = slot.get(settings) else {
        return false;
    };
    let wanted = normalize_hotkey(binding);
    all_normalized(settings).iter().filter(|b| **b == wanted).count() > 1
}

/// A binding the way the input shows it.
fn shown(binding: &str) -> String {
    let mac = is_mac();
    match Hotkey::parse(binding, mac) {
        Some(hotkey) => hotkey.display(mac),
        None => binding.to_owned(),
    }
}

fn input<'a>(
    settings: &HotkeySettings,
    capturing: Option<Slot>,
    slot: Slot,
) -> Element<'a, Message> {
    let waiting = capturing == Some(slot);
    let current = match slot.get(settings) {
        Some(binding) => shown(binding),
        None => String::new(),
    };
    let content = if waiting {
        fl!("dialog-settings-global-hotkey-press-a-key")
    } else {
        current
    };

    let field = button(text(content).width(Length::Fill))
        .width(180)
        .padding([4, 8])
        .style(if waiting { button::primary } else { button::secondary })
        .on_press(Message::Capture(slot));

    let warning: Element<'a, Message> = if is_duplicate(settings, slot) {
        tooltip(
            icon::toned(Icon::Alert, 18.0, Tone::Warning),
            container(text(fl!("dialog-settings-global-hotkey-duplicate-warning")).size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Left,
        )
        .into()
    } else {
        Space::new().width(18).into()
    };

    let clear = button(icon::toned(Icon::Close, 16.0, Tone::Muted))
        .padding(4)
        .style(button::text)
        .on_press(Message::ClearHotkey(slot));

    row![
        text(slot.label()).width(Length::Fill),
        field,
        warning,
        clear,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn section<'a>(
    title: String,
    slots: &[Slot],
    settings: &HotkeySettings,
    capturing: Option<Slot>,
) -> Element<'a, Message> {
    let mut inputs = column![].spacing(6);
    for slot in slots {
        inputs = inputs.push(input(settings, capturing, *slot));
    }
    column![text(title).size(16), inputs].spacing(8).into()
}

pub fn view<'a>(settings: &HotkeySettings, capturing: Option<Slot>) -> Element<'a, Message> {
    column![
        section(fl!("dialog-settings-global-hotkey-file-title"), &FILE, settings, capturing),
        section(fl!("dialog-settings-global-hotkey-edit-title"), &EDIT, settings, capturing),
        section(
            fl!("dialog-settings-global-hotkey-playback-title"),
            &PLAYBACK,
            settings,
            capturing
        ),
        section(fl!("dialog-settings-global-hotkey-audio-title"), &AUDIO, settings, capturing),
    ]
    .spacing(20)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_reads_and_writes_its_own_binding() {
        let mut settings = HotkeySettings::default();
        for slot in FILE.iter().chain(&EDIT).chain(&PLAYBACK).chain(&AUDIO) {
            slot.set(&mut settings, Some(format!("F{}", *slot as u8)));
            assert_eq!(slot.get(&settings).as_deref(), Some(format!("F{}", *slot as u8).as_str()));
        }
        // Every slot got a different value, so none shares a field.
        assert!(!is_duplicate(&settings, Slot::Load));
    }

    #[test]
    fn equal_shortcuts_are_flagged_however_they_are_written() {
        let mut settings = HotkeySettings::default();
        assert!(!is_duplicate(&settings, Slot::Execute));

        Slot::Load.set(&mut settings, Some("Shift+Control+K".to_owned()));
        Slot::Stop.set(&mut settings, Some("Control+Shift+K".to_owned()));
        assert!(is_duplicate(&settings, Slot::Load));
        assert!(is_duplicate(&settings, Slot::Stop));

        Slot::Stop.set(&mut settings, None);
        assert!(!is_duplicate(&settings, Slot::Load), "an unbound slot counts for nothing");
    }
}
