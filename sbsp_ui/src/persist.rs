// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! What the app remembers between runs: the window's place and size, and
//! the UI choices the Tauri build kept with `pinia-plugin-persistedstate`
//! (mode, the pre-wait / duration display modes, the sidebar tab and the
//! open/closed state of the sidebar and bottom viewer) and when to keep the
//! machine awake.
//!
//! The file is `ui_state.json` next to the settings (`config_dir()/<identifier>`).
//! Everything is optional, so a missing field, a missing file and a file
//! from an older version all fall back to the defaults. It is written when
//! the app quits. The web build keeps this in `localStorage` (Phase 9);
//! until then `load`/`save` do nothing there.

use serde::{Deserialize, Serialize};

use crate::state::ui::{DisplayMode, KeepAwakeMode, Mode, SideBarTab, UiState};

/// The smallest window a saved size may restore.
const MIN_WIDTH: f32 = 480.0;
const MIN_HEIGHT: f32 = 270.0;
/// Positions below this are the OS's "minimized" placeholder (Windows
/// reports -32000), not a place a window can be restored to.
const MIN_POSITION: f32 = -10_000.0;
const MAX_SIZE: f32 = 16_384.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowRect {
    pub width: f32,
    pub height: f32,
    /// Missing on platforms that do not report the window position
    /// (Wayland).
    pub x: Option<f32>,
    pub y: Option<f32>,
    /// The size and position above are from before the window was
    /// maximized, so that un-maximizing returns to them.
    #[serde(default)]
    pub maximized: bool,
}

impl WindowRect {
    /// Whether the rectangle can be restored as it is. A saved size from a
    /// setup that no longer exists (a huge or tiny window) is not trusted.
    pub fn is_usable(&self) -> bool {
        let size_ok = |value: f32, min: f32| value.is_finite() && (min..=MAX_SIZE).contains(&value);
        size_ok(self.width, MIN_WIDTH) && size_ok(self.height, MIN_HEIGHT)
    }

    /// The saved position, if both coordinates are there and plausible.
    pub fn position(&self) -> Option<(f32, f32)> {
        let (x, y) = (self.x?, self.y?);
        let ok = |value: f32| value.is_finite() && (MIN_POSITION..=MAX_SIZE).contains(&value);
        (ok(x) && ok(y)).then_some((x, y))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub window: Option<WindowRect>,
    pub mode: Option<Mode>,
    pub pre_wait_display_mode: Option<DisplayMode>,
    pub duration_display_mode: Option<DisplayMode>,
    pub side_bar_tab: Option<SideBarTab>,
    pub is_right_sidebar_open: Option<bool>,
    pub is_bottom_tab_open: Option<bool>,
    pub keep_awake: Option<KeepAwakeMode>,
}

impl Saved {
    pub fn capture(ui: &UiState, window: Option<WindowRect>) -> Self {
        Self {
            window,
            mode: Some(ui.mode),
            pre_wait_display_mode: Some(ui.pre_wait_display_mode),
            duration_display_mode: Some(ui.duration_display_mode),
            side_bar_tab: Some(ui.side_bar_tab),
            is_right_sidebar_open: Some(ui.is_right_sidebar_open),
            is_bottom_tab_open: Some(ui.is_bottom_tab_open),
            keep_awake: Some(ui.keep_awake),
        }
    }

    /// Puts the remembered choices into a fresh `UiState`. The mode goes in
    /// as it was; `set_permission` drops it later if the permissions held
    /// (a remote's, once connected) do not allow it.
    pub fn apply(&self, ui: &mut UiState) {
        if let Some(mode) = self.mode {
            ui.mode = mode;
        }
        if let Some(value) = self.pre_wait_display_mode {
            ui.pre_wait_display_mode = value;
        }
        if let Some(value) = self.duration_display_mode {
            ui.duration_display_mode = value;
        }
        if let Some(value) = self.side_bar_tab {
            ui.side_bar_tab = value;
        }
        if let Some(value) = self.is_right_sidebar_open {
            ui.is_right_sidebar_open = value;
        }
        if let Some(value) = self.is_bottom_tab_open {
            ui.is_bottom_tab_open = value;
        }
        if let Some(value) = self.keep_awake {
            ui.keep_awake = value;
        }
    }

    /// A file that does not parse (hand-edited, cut short) is the same as
    /// no file.
    pub fn parse(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_else(|error| {
            log::warn!("ignoring the saved UI state: {error}");
            Self::default()
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("the saved state serializes")
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod storage {
    use std::path::PathBuf;

    use super::Saved;

    /// The same identifiers as the Tauri builds, so the file sits next to
    /// the settings.
    fn path(target: &str) -> Option<PathBuf> {
        let identifier = match target {
            "host" => "com.keinsleif.sbsp",
            "remote" => "com.keinsleif.sbsp.remote",
            _ => return None,
        };
        dirs::config_dir().map(|dir| dir.join(identifier).join("ui_state.json"))
    }

    pub fn load(target: &str) -> Saved {
        let Some(path) = path(target) else {
            return Saved::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(json) => Saved::parse(&json),
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("could not read {}: {error}", path.display());
                }
                Saved::default()
            }
        }
    }

    pub fn save(target: &str, saved: &Saved) {
        let Some(path) = path(target) else {
            return;
        };
        // Written beside the file and renamed over it, so an interrupted
        // write never leaves half a file behind.
        let result = (|| {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, saved.to_json())?;
            std::fs::rename(&temporary, &path)
        })();
        if let Err(error) = result {
            log::warn!("could not save {}: {error}", path.display());
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod storage {
    use super::Saved;

    fn key(target: &str) -> String {
        format!("sbsp.{target}.ui_state")
    }

    pub fn load(target: &str) -> Saved {
        crate::browser::get(&key(target)).map_or_else(Saved::default, |json| Saved::parse(&json))
    }

    pub fn save(target: &str, saved: &Saved) {
        crate::browser::set(&key(target), &saved.to_json());
    }
}

pub use storage::{load, save};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choices_survive_a_round_trip() {
        let mut ui = UiState::new(true);
        ui.mode = Mode::Run;
        ui.side_bar_tab = SideBarTab::Meter;
        ui.is_right_sidebar_open = false;
        ui.duration_display_mode = DisplayMode::Remain;
        ui.keep_awake = KeepAwakeMode::Always;
        let window = WindowRect { width: 1000.0, height: 600.0, x: Some(10.0), y: None, maximized: true };

        let saved = Saved::parse(&Saved::capture(&ui, Some(window)).to_json());

        let mut restored = UiState::new(true);
        saved.apply(&mut restored);
        assert_eq!(restored.mode, Mode::Run);
        assert_eq!(restored.side_bar_tab, SideBarTab::Meter);
        assert!(!restored.is_right_sidebar_open);
        assert!(restored.is_bottom_tab_open);
        assert_eq!(restored.duration_display_mode, DisplayMode::Remain);
        assert_eq!(restored.keep_awake, KeepAwakeMode::Always);
        assert_eq!(saved.window, Some(window));
    }

    #[test]
    fn missing_fields_and_broken_files_fall_back_to_the_defaults() {
        assert_eq!(Saved::parse("{}"), Saved::default());
        assert_eq!(Saved::parse("{ \"mode\": \"sideways\" }"), Saved::default());
        assert_eq!(Saved::parse("not json"), Saved::default());
        // An unknown field from a later version is ignored.
        assert_eq!(Saved::parse("{ \"mode\": \"run\", \"later\": 1 }").mode, Some(Mode::Run));
    }

    #[test]
    fn applying_nothing_keeps_the_defaults() {
        let mut ui = UiState::new(true);
        Saved::default().apply(&mut ui);
        let fresh = UiState::new(true);
        assert_eq!(ui.mode, fresh.mode);
        assert_eq!(ui.is_right_sidebar_open, fresh.is_right_sidebar_open);
    }

    #[test]
    fn implausible_windows_are_not_restored() {
        let rect = |width, height, x, y| WindowRect { width, height, x, y, maximized: false };
        assert!(rect(1280.0, 720.0, None, None).is_usable());
        assert!(!rect(100.0, 720.0, None, None).is_usable());
        assert!(!rect(1280.0, 100_000.0, None, None).is_usable());
        assert!(!rect(f32::NAN, 720.0, None, None).is_usable());
        assert_eq!(rect(1280.0, 720.0, Some(-5.0), Some(30.0)).position(), Some((-5.0, 30.0)));
        // The Windows "minimized" placeholder, and a half-known position.
        assert_eq!(rect(1280.0, 720.0, Some(-32_000.0), Some(-32_000.0)).position(), None);
        assert_eq!(rect(1280.0, 720.0, Some(10.0), None).position(), None);
    }
}
