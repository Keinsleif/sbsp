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
//!
//! File/delete/renumber bindings are not included yet: their actions
//! (open/save dialogs, cue removal, the renumber dialog) arrive with
//! Phase 6/7, and a binding with nothing behind it would just swallow
//! the key.

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

        let table: [(&Option<String>, Action); 15] = [
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
                .collect(),
        )
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
