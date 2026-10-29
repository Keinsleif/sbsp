// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! UI-independent types and logic (see the migration plan, section 4.3).
//!
//! Everything here is plain data in, plain data out -- no iced types, no
//! backend connection, no platform code -- so it's unit-tested directly
//! with `cargo test`, unlike the UI itself.
//!
//! Ported from the Vue frontend's `utils.ts` and `stores/showModel.ts`.
//! Not a 1:1 port of every function in those files -- see each submodule's
//! doc comment for specifics.

pub mod clipboard;
pub mod cue;
pub mod cue_edit;
pub mod cursor;
pub mod easing;
pub mod envelope;
pub mod fader;
pub mod field;
pub mod flat_list;
pub mod hotkey;
pub mod new_cue;
pub mod text;
pub mod time;
