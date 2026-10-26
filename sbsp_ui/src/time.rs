// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The clock the app uses, which has to be the one `iced::time::every`
//! hands to its subscribers: std's on the desktop, and on wasm (where
//! std's `Instant::now()` panics) the browser-backed one iced's timers
//! use. `iced::time::Instant` is not that one on wasm, so the app names
//! this instead.

pub use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

#[cfg(target_arch = "wasm32")]
pub use wasmtimer::std::Instant;
