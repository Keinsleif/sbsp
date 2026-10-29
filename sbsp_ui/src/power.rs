// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Keeping the machine awake while the show is being run.
//!
//! Only the native builds (host and remote) can do it; on the web build
//! [`KeepAwake::set`] does nothing (the browser's Wake Lock API needs a
//! secure context, which the host's plain-HTTP LAN address is not).

use crate::state::ui::KeepAwakeMode;

/// Whether the machine should be kept awake right now.
pub fn wanted(mode: KeepAwakeMode, window_focused: bool) -> bool {
    match mode {
        KeepAwakeMode::Never => false,
        KeepAwakeMode::WhileFocused => window_focused,
        KeepAwakeMode::Always => true,
    }
}

/// Holds the OS's "do not sleep" request while it is wanted.
#[derive(Default)]
pub struct KeepAwake {
    #[cfg(any(feature = "host", feature = "remote"))]
    guard: Option<keepawake::KeepAwake>,
}

impl KeepAwake {
    /// Starts or ends the request. Asking for what is already the case does
    /// nothing; a failed start is logged and tried again on the next call.
    #[cfg(any(feature = "host", feature = "remote"))]
    pub fn set(&mut self, on: bool) {
        match (on, self.guard.is_some()) {
            (true, false) => {
                let result = keepawake::Builder::default()
                    .display(true)
                    .idle(true)
                    .reason("A show is being run")
                    .app_name("SBSP")
                    .app_reverse_domain("com.keinsleif.sbsp")
                    .create();
                match result {
                    Ok(guard) => self.guard = Some(guard),
                    Err(error) => log::warn!("could not keep the machine awake: {error}"),
                }
            }
            (false, true) => self.guard = None,
            _ => {}
        }
    }

    #[cfg(not(any(feature = "host", feature = "remote")))]
    pub fn set(&mut self, _on: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_and_the_focus_decide() {
        for focused in [true, false] {
            assert!(!wanted(KeepAwakeMode::Never, focused));
            assert!(wanted(KeepAwakeMode::Always, focused));
        }
        assert!(wanted(KeepAwakeMode::WhileFocused, true));
        assert!(!wanted(KeepAwakeMode::WhileFocused, false));
    }
}
