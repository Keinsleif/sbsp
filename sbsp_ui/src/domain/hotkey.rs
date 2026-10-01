// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `utils.ts`: `normalizeHotkey` (and its `MODIFIER_KEYS`
//! constant, from `composables/useHotkey.ts`).
//!
//! This only normalizes the stored string representation (e.g.
//! `"ctrl+Shift+s"` -> `"Control+Shift+S"` is not quite right -- case of
//! the non-modifier key is left as-is, matching the original, which never
//! touched casing beyond the literal `"Ctrl"` -> `"Control"` substitution).
//! It does not parse the string into a structured key representation or
//! match it against iced key events; that's separate work for whichever
//! phase wires up the actual keyboard subscription (Phase 4), once there's
//! a concrete `iced::keyboard::Key` to match against to design it around.

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
