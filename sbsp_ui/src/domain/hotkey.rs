// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `utils.ts`: `normalizeHotkey` (and its `MODIFIER_KEYS`
//! constant, from `composables/useHotkey.ts`).
//!
//! This only normalizes the stored string representation (e.g.
//! `"ctrl+Shift+s"` -> `"Control+Shift+S"` is not quite right -- case of
//! the non-modifier key is left as-is, matching the original, which never
//! touched casing beyond the literal `"Ctrl"` -> `"Control"` substitution).
//! [`Hotkey`] parses a binding into modifiers + key name and matches it
//! against a key press; turning an iced key event into that key name is
//! `features::hotkeys`' job, so this module stays free of iced types.

const MODIFIER_KEYS: [&str; 6] = ["Control", "Meta", "OS", "Alt", "AltGraph", "Shift"];

/// Normalizes a hotkey string to a canonical `Modifier+...+Key` form:
/// modifiers sorted alphabetically and listed first, the non-modifier
/// key(s) after. Empty segments (from stray `+`s) are dropped. An empty
/// input returns an empty string.
pub fn normalize_hotkey(hotkey: &str) -> String {
    if hotkey.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = hotkey
        .split('+')
        .map(|p| p.trim().replace("Ctrl", "Control"))
        .filter(|p| !p.is_empty())
        .collect();

    let mut modifiers: Vec<&String> = parts.iter().filter(|p| is_modifier(p)).collect();
    modifiers.sort();

    let keys: Vec<&String> = parts.iter().filter(|p| !is_modifier(p)).collect();

    modifiers
        .into_iter()
        .chain(keys)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("+")
}

fn is_modifier(part: &str) -> bool {
    MODIFIER_KEYS.contains(&part)
}

/// Modifier state of a key press or a binding; compared exactly, as the
/// original's `tinykeys` does (`Shift+ArrowUp` does not fire for a plain
/// `ArrowUp` binding, and vice versa).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// Cmd on macOS, the Windows/Super key elsewhere.
    pub logo: bool,
}

/// A parsed binding such as `"$mod+Shift+S"` or `"ArrowUp"`.
///
/// The key is kept as a lowercased name (`"arrowup"`, `"space"`, `"["`);
/// [`Hotkey::matches`] compares case-insensitively against whatever name
/// the UI layer derives from its own key event. `$mod` resolves to Cmd on
/// macOS and Ctrl elsewhere, like `tinykeys`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    mods: Mods,
    key: String,
}

impl Hotkey {
    /// `None` for an empty spec, one with no non-modifier key, or one with
    /// more than one (a chord sequence is not supported).
    pub fn parse(spec: &str, is_mac: bool) -> Option<Self> {
        let mut mods = Mods::default();
        let mut key: Option<String> = None;

        for part in spec.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            let lower = part.to_lowercase();
            match lower.as_str() {
                "$mod" if is_mac => mods.logo = true,
                "$mod" => mods.ctrl = true,
                "control" | "ctrl" => mods.ctrl = true,
                "meta" | "cmd" | "command" | "os" => mods.logo = true,
                "alt" | "option" => mods.alt = true,
                "shift" => mods.shift = true,
                _ => {
                    if key.is_some() {
                        return None;
                    }
                    key = Some(lower);
                }
            }
        }

        Some(Self { mods, key: key? })
    }

    /// How a menu shows this binding: `Ctrl+Shift+S`, or `⇧⌘S` on a Mac.
    pub fn display(&self, is_mac: bool) -> String {
        let key = display_key(&self.key);
        let Mods {
            ctrl,
            shift,
            alt,
            logo,
        } = self.mods;

        if is_mac {
            let mut text = String::new();
            for (on, symbol) in [(ctrl, '⌃'), (alt, '⌥'), (shift, '⇧'), (logo, '⌘')] {
                if on {
                    text.push(symbol);
                }
            }
            text + &key
        } else {
            let mut parts = Vec::new();
            for (on, name) in [(ctrl, "Ctrl"), (alt, "Alt"), (shift, "Shift"), (logo, "Win")] {
                if on {
                    parts.push(name.to_owned());
                }
            }
            parts.push(key);
            parts.join("+")
        }
    }

    pub fn matches(&self, key: &str, mods: Mods) -> bool {
        self.mods == mods && self.key == key.to_lowercase()
    }
}

/// A key name (lowercased, as [`Hotkey`] keeps it) the way a menu shows it.
fn display_key(key: &str) -> String {
    match key {
        "arrowup" => "↑".to_owned(),
        "arrowdown" => "↓".to_owned(),
        "arrowleft" => "←".to_owned(),
        "arrowright" => "→".to_owned(),
        "escape" => "Esc".to_owned(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_display_the_way_menus_show_them() {
        let save_as = Hotkey::parse("$mod+Shift+S", false).unwrap();
        assert_eq!(save_as.display(false), "Ctrl+Shift+S");
        assert_eq!(Hotkey::parse("$mod+Shift+S", true).unwrap().display(true), "⇧⌘S");
        assert_eq!(Hotkey::parse("Escape", false).unwrap().display(false), "Esc");
        assert_eq!(
            Hotkey::parse("$mod+Backspace", false).unwrap().display(false),
            "Ctrl+Backspace"
        );
        assert_eq!(Hotkey::parse("Shift+ArrowUp", false).unwrap().display(false), "Shift+↑");
    }

    #[test]
    fn empty_is_empty() {
        assert_eq!(normalize_hotkey(""), "");
    }

    #[test]
    fn replaces_ctrl_with_control() {
        assert_eq!(normalize_hotkey("Ctrl+S"), "Control+S");
    }

    #[test]
    fn sorts_modifiers_before_key() {
        assert_eq!(normalize_hotkey("S+Shift+Control"), "Control+Shift+S");
    }

    #[test]
    fn drops_empty_segments() {
        assert_eq!(normalize_hotkey("Control++S"), "Control+S");
    }

    #[test]
    fn leaves_key_casing_alone() {
        assert_eq!(normalize_hotkey("Control+s"), "Control+s");
    }

    const NONE: Mods = Mods {
        ctrl: false,
        shift: false,
        alt: false,
        logo: false,
    };

    #[test]
    fn parses_a_bare_key() {
        let hotkey = Hotkey::parse("ArrowUp", false).unwrap();
        assert!(hotkey.matches("ArrowUp", NONE));
        assert!(hotkey.matches("arrowup", NONE));
        assert!(!hotkey.matches("ArrowDown", NONE));
    }

    #[test]
    fn mod_is_ctrl_off_mac_and_cmd_on_mac() {
        let ctrl = Mods { ctrl: true, ..NONE };
        let logo = Mods { logo: true, ..NONE };

        let pc = Hotkey::parse("$mod+S", false).unwrap();
        assert!(pc.matches("s", ctrl));
        assert!(!pc.matches("s", logo));

        let mac = Hotkey::parse("$mod+S", true).unwrap();
        assert!(mac.matches("s", logo));
        assert!(!mac.matches("s", ctrl));
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let plain = Hotkey::parse("ArrowUp", false).unwrap();
        let shift = Hotkey::parse("Shift+ArrowUp", false).unwrap();
        let shifted = Mods {
            shift: true,
            ..NONE
        };

        assert!(plain.matches("ArrowUp", NONE));
        assert!(!plain.matches("ArrowUp", shifted));
        assert!(shift.matches("ArrowUp", shifted));
        assert!(!shift.matches("ArrowUp", NONE));
    }

    #[test]
    fn old_ctrl_spelling_still_parses() {
        let hotkey = Hotkey::parse("Ctrl+O", false).unwrap();
        assert!(hotkey.matches("o", Mods { ctrl: true, ..NONE }));
    }

    #[test]
    fn rejects_specs_without_exactly_one_key() {
        assert!(Hotkey::parse("", false).is_none());
        assert!(Hotkey::parse("Shift", false).is_none());
        assert!(Hotkey::parse("A+B", false).is_none());
    }
}
