// SPDX-License-Identifier: Elastic-2.0
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

pub mod cue;
pub mod easing;
pub mod fader;
pub mod flat_list;
pub mod hotkey;
pub mod text;
pub mod time;
