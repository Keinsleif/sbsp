// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Theme resolution: turns the user's theme preference (light / dark /
//! follow the OS) into a concrete `iced::Theme`.
//!
//! The actual color values live in [`palette`], kept separate so they can
//! be tuned without touching the resolution logic below.

pub mod palette;

/// The user-facing theme preference, as stored in settings.
///
/// This is intentionally a plain enum rather than `Option<iced::Theme>` so
/// it can be serialized directly into `sbsp_frontend_settings` without a
/// dependency on iced from that crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

/// Resolves a [`ThemeMode`] into a concrete [`iced::Theme`].
///
/// `System` asks the OS for its current light/dark setting. Detection can
/// fail (or return `Unspecified`, e.g. on a Linux desktop with no portal
/// support) in which case this falls back to dark, matching the rest of the
/// application's default.
pub fn resolve(mode: ThemeMode) -> iced::Theme {
    match mode {
        ThemeMode::Light => light(),
        ThemeMode::Dark => dark(),
        ThemeMode::System => match dark_light::detect() {
            Ok(dark_light::Mode::Light) => light(),
            Ok(dark_light::Mode::Dark) | Ok(dark_light::Mode::Unspecified) | Err(_) => dark(),
        },
    }
}

pub fn light() -> iced::Theme {
    iced::Theme::custom("SBSP Light".to_string(), palette::LIGHT)
}

pub fn dark() -> iced::Theme {
    iced::Theme::custom("SBSP Dark".to_string(), palette::DARK)
}

/// True if `theme` is one of the two themes returned by [`resolve`] for
/// [`ThemeMode::Dark`]. Used by widgets that need a light/dark branch
/// instead of pulling individual colors from the palette (e.g. choosing
/// between two pre-rendered icon variants).
pub fn is_dark(theme: &iced::Theme) -> bool {
    theme.extended_palette().is_dark
}
