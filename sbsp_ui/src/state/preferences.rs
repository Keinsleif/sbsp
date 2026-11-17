// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The part of the settings that both the host and the remote keep, and
//! what the app does with it. The host's settings also hold the audio
//! hardware; the remote's do not.

use crate::settings::hotkey::HotkeySettings;
use crate::settings::{
    AppearanceSettings, DarkMode, GeneralSettings, GlobalHostSettings, GlobalRemoteSettings,
    NameFormatSettings, TemplateSettings,
};

use crate::theme::ThemeMode;

#[derive(Debug, Clone, PartialEq)]
pub struct Preferences {
    pub general: GeneralSettings,
    pub appearance: AppearanceSettings,
    pub hotkey: HotkeySettings,
    pub template: TemplateSettings,
    pub name_format: NameFormatSettings,
}

impl Default for Preferences {
    fn default() -> Self {
        Self::from(&GlobalHostSettings::default())
    }
}

impl From<&GlobalHostSettings> for Preferences {
    fn from(settings: &GlobalHostSettings) -> Self {
        Self {
            general: settings.general.clone(),
            appearance: settings.appearance.clone(),
            hotkey: settings.hotkey.clone(),
            template: settings.template.clone(),
            name_format: settings.name_format.clone(),
        }
    }
}

impl From<&GlobalRemoteSettings> for Preferences {
    fn from(settings: &GlobalRemoteSettings) -> Self {
        Self {
            general: settings.general.clone(),
            appearance: settings.appearance.clone(),
            hotkey: settings.hotkey.clone(),
            template: settings.template.clone(),
            name_format: settings.name_format.clone(),
        }
    }
}

pub fn theme_mode(mode: DarkMode) -> ThemeMode {
    match mode {
        DarkMode::Dark => ThemeMode::Dark,
        DarkMode::Light => ThemeMode::Light,
        DarkMode::System => ThemeMode::System,
    }
}

pub fn dark_mode(mode: ThemeMode) -> DarkMode {
    match mode {
        ThemeMode::Dark => DarkMode::Dark,
        ThemeMode::Light => DarkMode::Light,
        ThemeMode::System => DarkMode::System,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_modes_round_trip() {
        for mode in [DarkMode::Dark, DarkMode::Light, DarkMode::System] {
            assert_eq!(dark_mode(theme_mode(mode)), mode);
        }
    }
}
