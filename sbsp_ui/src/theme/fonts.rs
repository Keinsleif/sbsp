// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The fonts the UI ships with: BIZ UDPGothic for text and BIZ UDGothic
//! where a fixed width is needed (clock, elapsed / remaining time).
//! Both are SIL Open Font License 1.1 (Morisawa, distributed at
//! googlefonts/morisawa-biz-ud-gothic); their license text goes with the
//! files (`assets/fonts/OFL.txt`) and into the credits.
//!
//! On the desktop they are embedded so the look and the column widths do not depend on
//! what the OS has installed. iced's `Font::MONOSPACE` means "whatever
//! monospace font the OS has", which is why [`MONO`] is used instead.
//!
//! The browser build does not embed them (4.6 MB each would double the
//! wasm): it fetches cut-down copies from the host that serves the page
//! (`scripts/subset_fonts.py`, `web::fetch_fonts`) and registers them with
//! the same family names.
//!
//! The font files are not in the repository's source patches: put
//! `BIZUDPGothic-Regular.ttf` and `BIZUDGothic-Regular.ttf` into
//! `sbsp_ui/assets/fonts/` (see the README there). Only the regular
//! weight is bundled; the UI does not use bold.

use iced::Font;

/// The family names are the fonts' own (their `name` table), which is what
/// iced looks them up by.
/// The size of text that does not set one (iced's own default is 16, which
/// is large for this UI: the sidebar's 260px runs out). BIZ UD glyphs fill
/// their em box more than iced's default font, so 14 reads about the same
/// as the original's text.
pub const TEXT_SIZE: f32 = 14.0;

pub const UI: Font = Font::with_name("BIZ UDPGothic");
pub const MONO: Font = Font::with_name("BIZ UDGothic");

#[cfg(not(target_arch = "wasm32"))]
pub const UI_REGULAR: &[u8] = include_bytes!("../../assets/fonts/BIZUDPGothic-Regular.ttf");
#[cfg(not(target_arch = "wasm32"))]
pub const MONO_REGULAR: &[u8] = include_bytes!("../../assets/fonts/BIZUDGothic-Regular.ttf");

#[cfg(all(test, not(target_arch = "wasm32")))]
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
