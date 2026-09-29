// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Color values for the two SBSP themes.
//!
//! The original Vue frontend did not define its own palette; it relied on
//! PrimeVue's default preset (see the migration plan, section 6). These are
//! new starting values for the iced UI, chosen to read clearly on a
//! projector/booth laptop in a dim room (the primary use case), not a
//! reproduction of PrimeVue's colors. Treat them as a first draft: replace
//! `PRIMARY_LIGHT`/`PRIMARY_DARK` (and, if needed, the rest) once there is a
//! visual pass on the real screens in Phase 3.

use iced::Color;
use iced::color;
use iced::theme::Palette;

/// Accent color shared by both themes (a show-control amber, distinct from
/// the red/green used for danger/success so cue selection doesn't read as
/// an error or a "go").
const PRIMARY: Color = color!(0xE0A030);

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
