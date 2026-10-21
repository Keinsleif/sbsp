// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Keyboard shortcuts: the settings' hotkey strings turned into a lookup
//! from a key press to an [`Action`].
//!
//! Ported from the `useHotkey` calls in `MainView.vue` / `CueList.vue`.
//! This module only decides *which* action a key press names; whether the
//! action is allowed right now (view mode, nothing selected, ...) and what
//! it does is the app's business, as in the original where each
//! `useHotkey` callback carried its own guards.

use iced::keyboard::{Key, Modifiers, key::Named};
use sbsp_frontend_settings::hotkey::HotkeySettings;

use crate::domain::hotkey::{Hotkey, Mods};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
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
    /// Candidate names for the pressed key (see [`key_names`]); a binding
    /// matches if it equals any of them.
    pub names: Vec<String>,
    pub mods: Mods,
    pub repeat: bool,
}

impl KeyPress {
    pub fn new(key: &Key, modified_key: &Key, modifiers: Modifiers, repeat: bool) -> Option<Self> {
        let mut names = key_names(key);
        for name in key_names(modified_key) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        if names.is_empty() {
            return None;
        }

        Some(Self {
            names,
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

/// Names a key goes by in binding strings: `"ArrowUp"`, `"Enter"`,
/// `"Space"` for named keys (the same spelling the settings use, which
/// comes from the browser's `KeyboardEvent.key`), the character itself
/// for character keys. Empty for keys that cannot be bound (`Unidentified`,
/// and bare modifier presses).
fn key_names(key: &Key) -> Vec<String> {
    match key {
        Key::Named(
            Named::Shift
            | Named::Control
            | Named::Alt
            | Named::Super
            | Named::Meta
            | Named::AltGraph,
        ) => Vec::new(),
        Key::Named(named) => vec![format!("{named:?}")],
        Key::Character(c) if c.as_str() == " " => vec!["Space".to_owned()],
        Key::Character(c) => vec![c.to_string()],
        Key::Unidentified => Vec::new(),
    }
}

pub struct Bindings(Vec<(Hotkey, Action)>);

impl Bindings {
    pub fn from_settings(settings: &HotkeySettings, is_mac: bool) -> Self {
        let playback = &settings.playback;
        let edit = &settings.edit;
        let file = &settings.file;

        let table: [(&Option<String>, Action); 21] = [
            (&playback.execute, Action::Execute),
            (&playback.load, Action::Load),
            (&playback.pause_and_resume, Action::PauseAndResume),
            (&playback.pause_all, Action::PauseAll),
            (&playback.resume_all, Action::ResumeAll),
            (&playback.stop, Action::Stop),
            (&playback.stop_all, Action::StopAll),
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

        // Not in the settings: the original left these to the browser's
        // own copy / cut / paste events.
        let fixed = [
            ("$mod+C", Action::Copy),
            ("$mod+X", Action::Cut),
            ("$mod+V", Action::Paste),
            ("$mod+D", Action::Duplicate),
        ];

        Self(
            table
                .into_iter()
                .filter_map(|(spec, action)| {
                    let hotkey = Hotkey::parse(spec.as_deref()?, is_mac);
                    if hotkey.is_none() {
                        log::warn!("ignoring unparsable hotkey {spec:?} for {action:?}");
                    }
                    Some((hotkey?, action))
                })
                .chain(
                    fixed
                        .into_iter()
                        .filter_map(|(spec, action)| Some((Hotkey::parse(spec, is_mac)?, action))),
                )
                .collect(),
        )
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
            .find(|(hotkey, _)| {
                press
                    .names
                    .iter()
                    .any(|name| hotkey.matches(name, press.mods))
            })
            .map(|(_, action)| *action)
            .filter(|action| !press.repeat || action.repeats())
    }
}

/// `is_mac` for the host platforms; the web build will need the browser's
/// answer instead (Phase 9).
pub fn is_mac() -> bool {
    cfg!(target_os = "macos")
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
        KeyPress::new(&key, &key, modifiers, false).unwrap()
    }

    fn named(n: Named) -> Key {
        Key::Named(n)
    }

    fn ch(c: &str) -> Key {
        Key::Character(c.into())
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
        assert_eq!(b.shortcut(Action::Export, false), None, "unbound by default");
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
        let held = KeyPress::new(
            &named(Named::Backspace),
            &named(Named::Backspace),
            Modifiers::CTRL,
            true,
        )
        .unwrap();
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
        let held = KeyPress::new(&ch("v"), &ch("v"), Modifiers::CTRL, true).unwrap();
        assert_eq!(b.find(&held), None);
    }

    #[test]
    fn unbound_keys_resolve_to_nothing() {
        let b = bindings();
        assert_eq!(b.find(&press(ch("q"), Modifiers::empty())), None);
    }

    #[test]
    fn held_keys_only_repeat_for_movement_and_seek() {
        let b = bindings();
        let held = |key: Key| KeyPress::new(&key, &key, Modifiers::empty(), true).unwrap();

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
            KeyPress::new(
                &named(Named::Shift),
                &named(Named::Shift),
                Modifiers::SHIFT,
                false
            )
            .is_none()
        );
    }
}
