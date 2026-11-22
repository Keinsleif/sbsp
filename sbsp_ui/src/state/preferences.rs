// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The part of the settings that both the host and the remote keep, and
//! what the app does with it. The host's settings also hold the audio
//! hardware; the remote's do not.

use crate::i18n::Language;
use crate::settings::hotkey::HotkeySettings;
use crate::settings::{
    AppearanceSettings, GeneralSettings, GlobalHostSettings, GlobalRemoteSettings,
    NameFormatSettings, TemplateSettings,
};
use crate::theme::ThemeMode;

#[derive(Debug, Clone, PartialEq)]
pub struct Preferences {
    pub general: GeneralSettings,
    pub appearance: AppearancePreferences,
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
            appearance: (&settings.appearance).into(),
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
            appearance: (&settings.appearance).into(),
            hotkey: settings.hotkey.clone(),
            template: settings.template.clone(),
            name_format: settings.name_format.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppearancePreferences {
    pub dark_mode: ThemeMode,
    pub lang: Option<Language>,
    pub hide_controls: bool,
}

impl From<&AppearanceSettings> for AppearancePreferences {
    fn from(value: &AppearanceSettings) -> Self {
        Self {
            dark_mode: value.dark_mode,
            lang: Language::from_setting(value.language.as_ref()),
            hide_controls: value.hide_controls,
        }
    }
}

impl From<&AppearancePreferences> for AppearanceSettings {
    fn from(value: &AppearancePreferences) -> Self {
        Self {
            language: value.lang.map(|l| l.langid()),
            dark_mode: value.dark_mode,
            hide_controls: value.hide_controls,
        }
    }
}