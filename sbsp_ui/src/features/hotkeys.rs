// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Keyboard shortcuts: the settings' hotkey strings turned into a lookup
//! from a key press to an [`Action`].
//!
//! This module only decides *which* action a key press names; whether the
//! action is allowed right now (view mode, nothing selected, ...) and what
//! it does is the app's business.

use crate::settings::hotkey::HotkeySettings;
use iced::keyboard::{Key, Modifiers, key::Named};

use crate::domain::hotkey::{Hotkey, Mods};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Execute,
    Load,
    PauseAndResume,
    PauseAll,
    ResumeAll,
    Stop,
    FadeOut,
    HardStop,
    StopAll,
    FadeAllOut,
    HardStopAll,
    SeekForward,
    SeekBackward,
    ToggleRepeat,
    CursorUp,
    ExtendUp,
    CursorDown,
    ExtendDown,
    SelectAll,
    /// Remove the selected cues (edit mode, after a confirmation).
    Delete,
    /// Copy the selected cues to the app's clipboard.
    Copy,
    /// Copy them and remove the originals (edit mode).
    Cut,
    /// Add the clipboard's cues after the selection (edit mode).
    Paste,
    /// Add copies of the selected cues after them (edit mode).
    Duplicate,
    /// Open the renumber dialog for the selected cues (edit mode).
    Renumber,
    /// File menu commands (host only).
    Open,
    Save,
    SaveAs,
    Export,
}

impl Action {
    /// Whether holding the key should keep firing the action. Moving the
    /// cursor and seeking are meant to be held; starting, stopping and
    /// the like must fire once per physical press -- a held Enter
    /// re-triggering GO is not something a show wants.
    pub fn repeats(self) -> bool {
        matches!(
            self,
            Action::CursorUp
                | Action::ExtendUp
                | Action::CursorDown
                | Action::ExtendDown
                | Action::SeekForward
                | Action::SeekBackward
        )
    }
}

/// One key press, reduced to what binding lookup needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPress {
    /// The pressed key's name (see [`key_name`]), as the key itself: with
    /// Shift held, `1` stays `"1"` rather than becoming `"!"`.
    pub name: String,
    pub mods: Mods,
    pub repeat: bool,
}

impl KeyPress {
    /// `key` is the key without the modifiers applied (iced's `key`, not
    /// its `modified_key`).
    pub fn new(key: &Key, modifiers: Modifiers, repeat: bool) -> Option<Self> {
        Some(Self {
            name: key_name(key)?,
            mods: Mods {
                ctrl: modifiers.control(),
                shift: modifiers.shift(),
                alt: modifiers.alt(),
                logo: modifiers.logo(),
            },
            repeat,
        })
    }
}

/// The name a key goes by in binding strings: `"ArrowUp"`, `"Enter"`,
/// `"Space"` for named keys (the same spelling the settings use, which
/// comes from the browser's `KeyboardEvent.key`), the character itself
/// for character keys. `None` for keys that cannot be bound
/// (`Unidentified`, and bare modifier presses).
fn key_name(key: &Key) -> Option<String> {
    match key {
        Key::Named(
            Named::Shift
            | Named::Control
            | Named::Alt
            | Named::Super
            | Named::Meta
            | Named::AltGraph,
        ) => None,
        Key::Named(named) => Some(format!("{named:?}")),
        Key::Character(c) if c.as_str() == " " => Some("Space".to_owned()),
        Key::Character(c) => Some(c.to_string()),
        Key::Unidentified => None,
    }
}

pub struct Bindings(Vec<(Hotkey, Action)>);

impl Bindings {
    pub fn from_settings(settings: &HotkeySettings, is_mac: bool) -> Self {
        let playback = &settings.playback;
        let edit = &settings.edit;
        let file = &settings.file;

        let table: [(&Option<String>, Action); 25] = [
            (&playback.execute, Action::Execute),
            (&playback.load, Action::Load),
            (&playback.pause_and_resume, Action::PauseAndResume),
            (&playback.pause_all, Action::PauseAll),
            (&playback.resume_all, Action::ResumeAll),
            (&playback.stop, Action::Stop),
            (&playback.fade_out, Action::FadeOut),
            (&playback.hard_stop, Action::HardStop),
            (&playback.stop_all, Action::StopAll),
            (&playback.fade_all_out, Action::FadeAllOut),
            (&playback.hard_stop_all, Action::HardStopAll),
            (&playback.seek_forward, Action::SeekForward),
            (&playback.seek_backward, Action::SeekBackward),
            (&settings.audio_action.toggle_repeat, Action::ToggleRepeat),
            (&edit.cuelist_move_up, Action::CursorUp),
            (&edit.cuelist_extend_up, Action::ExtendUp),
            (&edit.cuelist_move_down, Action::CursorDown),
            (&edit.cuelist_extend_down, Action::ExtendDown),
            (&edit.select_all, Action::SelectAll),
            (&edit.delete, Action::Delete),
            (&edit.renumber_cues, Action::Renumber),
            (&file.open, Action::Open),
            (&file.save, Action::Save),
            (&file.save_as, Action::SaveAs),
            (&file.export_to_folder, Action::Export),
        ];

        // Not in the settings; reserved first so a configured hotkey can
        // never shadow them.
        let fixed = [
            ("$mod+C", Action::Copy),
            ("$mod+X", Action::Cut),
            ("$mod+V", Action::Paste),
            ("$mod+D", Action::Duplicate),
        ];

        let mut bindings: Vec<(Hotkey, Action)> = Vec::new();
        for (spec, action) in fixed {
            if let Some(hotkey) = Hotkey::parse(spec, is_mac) {
                bindings.push((hotkey, action));
            }
        }
        for (spec, action) in table {
            let Some(spec) = spec.as_deref() else {
                continue;
            };
            let Some(hotkey) = Hotkey::parse(spec, is_mac) else {
                log::warn!("ignoring unparsable hotkey {spec:?} for {action:?}");
                continue;
            };
            // One key press must mean one action, and the shortcut shown
            // for an action must be the one that triggers it.
            if let Some((_, owner)) = bindings.iter().find(|(taken, _)| *taken == hotkey) {
                log::warn!(
                    "ignoring hotkey {spec:?} for {action:?}: already bound to {owner:?}"
                );
                continue;
            }
            bindings.push((hotkey, action));
        }
        Self(bindings)
    }

    /// How a menu shows the first binding of `action`, if it has one.
    pub fn shortcut(&self, action: Action, is_mac: bool) -> Option<String> {
        self.0
            .iter()
            .find(|(_, bound)| *bound == action)
            .map(|(hotkey, _)| hotkey.display(is_mac))
    }

    pub fn find(&self, press: &KeyPress) -> Option<Action> {
        self.0
            .iter()
            .find(|(hotkey, _)| hotkey.matches(&press.name, press.mods))
            .map(|(_, action)| *action)
            .filter(|action| !press.repeat || action.repeats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bindings() -> Bindings {
        Bindings::from_settings(&HotkeySettings::default(), false)
    }

    fn bindings_mac() -> Bindings {
        Bindings::from_settings(&HotkeySettings::default(), true)
    }

    fn press(key: Key, modifiers: Modifiers) -> KeyPress {
        KeyPress::new(&key, modifiers, false).unwrap()
    }

    fn named(n: Named) -> Key {
        Key::Named(n)
    }

    fn ch(c: &str) -> Key {
        Key::Character(c.into())
    }

    #[test]
    fn the_fade_out_and_hard_stop_keys_work_once_they_are_set() {
        let none = Modifiers::empty();
        // Not bound by default.
        assert_eq!(bindings().find(&press(named(Named::F1), none)), None);

        let mut settings = HotkeySettings::default();
        settings.playback.fade_out = Some("F1".to_owned());
        settings.playback.hard_stop = Some("F2".to_owned());
        settings.playback.fade_all_out = Some("F3".to_owned());
        settings.playback.hard_stop_all = Some("F4".to_owned());
        let b = Bindings::from_settings(&settings, false);
        assert_eq!(
            b.find(&press(named(Named::F1), none)),
            Some(Action::FadeOut)
        );
        assert_eq!(
            b.find(&press(named(Named::F2), none)),
            Some(Action::HardStop)
        );
        assert_eq!(
            b.find(&press(named(Named::F3), none)),
            Some(Action::FadeAllOut)
        );
        assert_eq!(
            b.find(&press(named(Named::F4), none)),
            Some(Action::HardStopAll)
        );
    }

    #[test]
    fn default_playback_keys_resolve() {
        let b = bindings();
        let none = Modifiers::empty();

        assert_eq!(
            b.find(&press(named(Named::Enter), none)),
            Some(Action::Execute)
        );
        assert_eq!(
            b.find(&press(named(Named::Space), none)),
            Some(Action::PauseAndResume)
        );
        assert_eq!(
            b.find(&press(named(Named::Escape), none)),
            Some(Action::StopAll)
        );
        assert_eq!(
            b.find(&press(named(Named::Backspace), none)),
            Some(Action::Stop)
        );
        assert_eq!(b.find(&press(ch("["), none)), Some(Action::PauseAll));
        assert_eq!(b.find(&press(ch("]"), none)), Some(Action::ResumeAll));
        assert_eq!(b.find(&press(ch("l"), none)), Some(Action::Load));
        assert_eq!(b.find(&press(ch("L"), none)), Some(Action::Load));
        assert_eq!(b.find(&press(ch("r"), none)), Some(Action::ToggleRepeat));
    }

    #[test]
    fn character_space_is_also_space() {
        let b = bindings();
        assert_eq!(
            b.find(&press(ch(" "), Modifiers::empty())),
            Some(Action::PauseAndResume)
        );
    }

    #[test]
    fn shift_arrow_extends_and_plain_arrow_moves() {
        let b = bindings();

        assert_eq!(
            b.find(&press(named(Named::ArrowUp), Modifiers::empty())),
            Some(Action::CursorUp)
        );
        assert_eq!(
            b.find(&press(named(Named::ArrowUp), Modifiers::SHIFT)),
            Some(Action::ExtendUp)
        );
        assert_eq!(
            b.find(&press(named(Named::ArrowDown), Modifiers::SHIFT)),
            Some(Action::ExtendDown)
        );
    }

    #[test]
    fn mod_a_selects_all_with_ctrl_on_pc() {
        let b = bindings();
        assert_eq!(
            b.find(&press(ch("a"), Modifiers::CTRL)),
            Some(Action::SelectAll)
        );
        assert_eq!(b.find(&press(ch("a"), Modifiers::empty())), None);
    }

    #[test]
    fn the_file_commands_follow_the_settings() {
        let b = bindings();
        for (key, shift, action) in [
            ("o", false, Action::Open),
            ("s", false, Action::Save),
            ("s", true, Action::SaveAs),
        ] {
            let mods = if shift {
                Modifiers::CTRL | Modifiers::SHIFT
            } else {
                Modifiers::CTRL
            };
            assert_eq!(b.find(&press(ch(key), mods)), Some(action), "{key}");
        }
    }

    #[test]
    fn menus_can_show_a_binding() {
        let b = bindings();
        let b_mac = bindings_mac();
        assert_eq!(b.shortcut(Action::Save, false).as_deref(), Some("Ctrl+S"));
        assert_eq!(b_mac.shortcut(Action::Copy, true).as_deref(), Some("⌘C"));
        assert_eq!(
            b.shortcut(Action::Export, false),
            None,
            "unbound by default"
        );
    }

    #[test]
    fn mod_r_renumbers() {
        let b = bindings();
        assert_eq!(
            b.find(&press(ch("r"), Modifiers::CTRL)),
            Some(Action::Renumber)
        );
    }

    #[test]
    fn mod_backspace_deletes_and_plain_backspace_still_stops() {
        let b = bindings();
        assert_eq!(
            b.find(&press(named(Named::Backspace), Modifiers::CTRL)),
            Some(Action::Delete)
        );
        assert_eq!(
            b.find(&press(named(Named::Backspace), Modifiers::empty())),
            Some(Action::Stop)
        );
    }

    #[test]
    fn deleting_does_not_repeat_when_held() {
        let b = bindings();
        let held = KeyPress::new(&named(Named::Backspace), Modifiers::CTRL, true).unwrap();
        assert_eq!(b.find(&held), None);
    }

    #[test]
    fn copy_cut_paste_and_duplicate_are_fixed_mod_keys() {
        let b = bindings();
        for (key, action) in [
            ("c", Action::Copy),
            ("x", Action::Cut),
            ("v", Action::Paste),
            ("d", Action::Duplicate),
        ] {
            assert_eq!(b.find(&press(ch(key), Modifiers::CTRL)), Some(action));
            assert_eq!(b.find(&press(ch(key), Modifiers::empty())), None);
        }
        // With the command key on a Mac.
        let mac = Bindings::from_settings(&HotkeySettings::default(), true);
        assert_eq!(
            mac.find(&press(ch("c"), Modifiers::LOGO)),
            Some(Action::Copy)
        );
        // Holding the key does not paste over and over.
        let held = KeyPress::new(&ch("v"), Modifiers::CTRL, true).unwrap();
        assert_eq!(b.find(&held), None);
    }

    #[test]
    fn a_configured_hotkey_cannot_shadow_a_fixed_one() {
        let mut settings = HotkeySettings::default();
        settings.playback.execute = Some("$mod+C".to_owned());
        let b = Bindings::from_settings(&settings, false);
        assert_eq!(
            b.find(&press(ch("c"), Modifiers::CTRL)),
            Some(Action::Copy)
        );
        assert_eq!(b.shortcut(Action::Execute, false), None);
    }

    #[test]
    fn conflicts_are_resolved_by_the_resolved_key_not_the_spelling() {
        let mut settings = HotkeySettings::default();
        settings.playback.execute = Some("Control+c".to_owned());
        let win = Bindings::from_settings(&settings, false);
        assert_eq!(win.shortcut(Action::Execute, false), None);
        // On a Mac `$mod` is the command key, so Control+C is free.
        let mac = Bindings::from_settings(&settings, true);
        assert_eq!(
            mac.find(&press(ch("c"), Modifiers::CTRL)),
            Some(Action::Execute)
        );
        assert_eq!(
            mac.find(&press(ch("c"), Modifiers::LOGO)),
            Some(Action::Copy)
        );
    }

    #[test]
    fn the_first_of_two_configured_duplicates_wins() {
        let mut settings = HotkeySettings::default();
        settings.playback.execute = Some("F5".to_owned());
        settings.playback.load = Some("f5".to_owned());
        let b = Bindings::from_settings(&settings, false);
        assert_eq!(
            b.find(&press(named(Named::F5), Modifiers::empty())),
            Some(Action::Execute)
        );
        assert_eq!(b.shortcut(Action::Load, false), None);
    }

    #[test]
    fn a_shifted_key_is_looked_up_as_the_key_itself() {
        let mut settings = HotkeySettings::default();
        settings.playback.execute = Some("Shift+1".to_owned());
        settings.playback.load = Some("Shift+!".to_owned());
        let b = Bindings::from_settings(&settings, false);
        // Shift+1 is the key "1" with Shift held, never "!".
        assert_eq!(
            b.find(&press(ch("1"), Modifiers::SHIFT)),
            Some(Action::Execute)
        );
        // `Shift+!` only fires on a keyboard that has "!" as a key of its own.
        assert_eq!(
            b.find(&press(ch("!"), Modifiers::SHIFT)),
            Some(Action::Load)
        );
    }

    #[test]
    fn unbound_keys_resolve_to_nothing() {
        let b = bindings();
        assert_eq!(b.find(&press(ch("q"), Modifiers::empty())), None);
    }

    #[test]
    fn held_keys_only_repeat_for_movement_and_seek() {
        let b = bindings();
        let held = |key: Key| KeyPress::new(&key, Modifiers::empty(), true).unwrap();

        assert_eq!(b.find(&held(named(Named::Enter))), None);
        assert_eq!(b.find(&held(named(Named::Space))), None);
        assert_eq!(
            b.find(&held(named(Named::ArrowDown))),
            Some(Action::CursorDown)
        );
        assert_eq!(
            b.find(&held(named(Named::ArrowRight))),
            Some(Action::SeekForward)
        );
    }

    #[test]
    fn a_disabled_binding_is_skipped() {
        let mut settings = HotkeySettings::default();
        settings.playback.execute = None;
        let b = Bindings::from_settings(&settings, false);

        assert_eq!(
            b.find(&press(named(Named::Enter), Modifiers::empty())),
            None
        );
    }

    #[test]
    fn bare_modifier_presses_are_not_bindable() {
        assert!(
            KeyPress::new(&named(Named::Shift), Modifiers::SHIFT, false).is_none()
        );
    }
}
