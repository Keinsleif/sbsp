// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The fonts the UI ships with: BIZ UDPGothic for text and BIZ UDGothic
//! where a fixed width is needed (clock, elapsed / remaining time).
//! Both are SIL Open Font License 1.1 (Morisawa, distributed at
//! googlefonts/morisawa-biz-ud-gothic); their license text goes with the
//! files (`assets/fonts/OFL.txt`) and into the credits.
//!
//! They are embedded so the look and the column widths do not depend on
//! what the OS has installed. iced's `Font::MONOSPACE` means "whatever
//! monospace font the OS has", which is why [`MONO`] is used instead.
//!
//! The font files are not in the repository's source patches: put
//! `BIZUDPGothic-Regular.ttf` and `BIZUDGothic-Regular.ttf` into
//! `sbsp_ui/assets/fonts/` (see the README there). Only the regular
//! weight is bundled; the UI does not use bold.

use iced::Font;

/// The family names are the fonts' own (their `name` table), which is what
/// iced looks them up by.
pub const UI: Font = Font::with_name("BIZ UDPGothic");
pub const MONO: Font = Font::with_name("BIZ UDGothic");

pub const UI_REGULAR: &[u8] = include_bytes!("../../assets/fonts/BIZUDPGothic-Regular.ttf");
pub const MONO_REGULAR: &[u8] = include_bytes!("../../assets/fonts/BIZUDGothic-Regular.ttf");

#[cfg(test)]
mod tests {
    use super::*;

    /// Catches the usual mistakes when adding the files by hand: an HTML
    /// page saved instead of the font, or a Git LFS pointer.
    #[test]
    fn the_embedded_files_are_fonts() {
        for bytes in [UI_REGULAR, MONO_REGULAR] {
            assert!(bytes.len() > 100_000, "suspiciously small: {} bytes", bytes.len());
            let tag = &bytes[..4];
            assert!(
                tag == [0, 1, 0, 0] || tag == b"OTTO" || tag == b"true" || tag == b"ttcf",
                "not a TrueType/OpenType file"
            );
        }
    }
}
