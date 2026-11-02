// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Color values for the two SBSP themes.

use iced::Color;
use iced::color;
use iced::theme::Palette;

/// Accent color shared by both themes (a show-control amber, distinct from
/// the red/green used for danger/success so cue selection doesn't read as
/// an error or a "go").
const PRIMARY: Color = color!(0x1867C0);

pub const DARK: Palette = Palette {
    background: color!(0x1B1D21),
    text: color!(0xE8E8E8),
    primary: PRIMARY,
    success: color!(0x3FB27F),
    warning: color!(0xE0A030),
    danger: color!(0xE5484D),
};

pub const LIGHT: Palette = Palette {
    background: color!(0xF7F7F8),
    text: color!(0x1B1D21),
    primary: PRIMARY,
    success: color!(0x1F9D62),
    warning: color!(0xB57200),
    danger: color!(0xC9312A),
};

pub fn get_text_color(bg: Color) -> Color {
    if bg.relative_contrast(Color::WHITE) >= bg.relative_contrast(Color::BLACK) {
        Color::WHITE
    } else {
        Color::BLACK
    }
}
