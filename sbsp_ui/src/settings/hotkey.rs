// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The keyboard shortcuts, as they are kept in the settings file.
//!
//! A binding is a string such as `"$mod+Shift+S"` or `"ArrowUp"`: modifiers
//! joined to a key with `+`, where `$mod` stands for Cmd on macOS and Ctrl
//! elsewhere (the string is parsed by `domain::hotkey`). `None` means the
//! action has no shortcut. Bindings are grouped by what they act on: files,
//! editing, playback, and audio cues.
//!
//! Field names are written in camelCase so that settings files stay
//! readable across versions. Every group and most fields fall back to their
//! defaults when missing, so a file written before a shortcut existed still
//! loads and gets that shortcut's default. `execute` also accepts the old
//! name `go`.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HotkeySettings {
    pub file: FileHotkey,
    pub edit: EditHotkey,
    pub playback: PlaybackHotkey,
    pub audio_action: AudioActionHotkey,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FileHotkey {
    pub open: Option<String>,
    pub save: Option<String>,
    pub save_as: Option<String>,
    pub export_to_folder: Option<String>,
}

impl Default for FileHotkey {
    fn default() -> Self {
        Self {
            open: Some("$mod+O".to_owned()),
            save: Some("$mod+S".to_owned()),
            save_as: Some("$mod+Shift+S".to_owned()),
            export_to_folder: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct EditHotkey {
    pub delete: Option<String>,
    pub select_all: Option<String>,
    pub renumber_cues: Option<String>,
    pub cuelist_move_up: Option<String>,
    pub cuelist_extend_up: Option<String>,
    pub cuelist_move_down: Option<String>,
    pub cuelist_extend_down: Option<String>,
}

impl Default for EditHotkey {
    fn default() -> Self {
        Self {
            delete: Some("$mod+Backspace".to_owned()),
            select_all: Some("$mod+A".to_owned()),
            renumber_cues: Some("$mod+R".to_owned()),
            cuelist_move_up: Some("ArrowUp".to_owned()),
            cuelist_extend_up: Some("Shift+ArrowUp".to_owned()),
            cuelist_move_down: Some("ArrowDown".to_owned()),
            cuelist_extend_down: Some("Shift+ArrowDown".to_owned()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaybackHotkey {
    #[serde(alias = "go")]
    pub execute: Option<String>,
    pub load: Option<String>,
    pub pause_and_resume: Option<String>,
    pub pause_all: Option<String>,
    pub resume_all: Option<String>,
    pub stop: Option<String>,
    #[serde(default)]
    pub fade_out: Option<String>,
    #[serde(default)]
    pub hard_stop: Option<String>,
    pub stop_all: Option<String>,
    #[serde(default)]
    pub fade_all_out: Option<String>,
    #[serde(default)]
    pub hard_stop_all: Option<String>,
    pub seek_forward: Option<String>,
    pub seek_backward: Option<String>,
}

impl Default for PlaybackHotkey {
    fn default() -> Self {
        Self {
            execute: Some("Enter".to_string()),
            load: Some("L".to_string()),
            pause_and_resume: Some("Space".to_string()),
            pause_all: Some("[".to_string()),
            resume_all: Some("]".to_string()),
            stop: Some("Backspace".to_string()),
            fade_out: None,
            hard_stop: None,
            stop_all: Some("Escape".to_string()),
            fade_all_out: None,
            hard_stop_all: None,
            seek_forward: Some("ArrowRight".to_string()),
            seek_backward: Some("ArrowLeft".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioActionHotkey {
    pub toggle_repeat: Option<String>,
}

impl Default for AudioActionHotkey {
    fn default() -> Self {
        Self {
            toggle_repeat: Some("R".to_string()),
        }
    }
}
