// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The size a cached drawing is kept at while the window is being resized.
//!
//! A `canvas::Cache` redraws whenever it is asked for another size, which
//! during a resize is every frame. Asking for the size it was last drawn at
//! hands back the drawing it already has.

use std::cell::Cell;

use iced::Size;

#[derive(Default)]
pub struct HeldSize(Cell<Option<Size>>);

impl HeldSize {
    /// The size to ask the cache for: `current`, or while `held` the size
    /// of the last drawing made without holding.
    pub fn pick(&self, current: Size, held: bool) -> Size {
        match (held, self.0.get()) {
            (true, Some(last)) => last,
            _ => {
                self.0.set(Some(current));
                current
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_size_is_kept_while_held() {
        let held = HeldSize::default();
        let first = Size::new(400.0, 80.0);

        assert_eq!(held.pick(first, true), first);
        assert_eq!(held.pick(Size::new(500.0, 80.0), true), first);
        assert_eq!(held.pick(Size::new(500.0, 80.0), false), Size::new(500.0, 80.0));
        assert_eq!(held.pick(Size::new(600.0, 80.0), true), Size::new(500.0, 80.0));
    }
}
