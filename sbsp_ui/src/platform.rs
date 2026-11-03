// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(not(target_arch = "wasm32"))]
pub mod atomic_write;
#[cfg(target_arch = "wasm32")]
pub mod browser;
#[cfg(any(feature = "host", feature = "remote"))]
pub mod files;
#[cfg(not(target_arch = "wasm32"))]
pub mod logging;
pub mod power;
pub mod time;
#[cfg(installer)]
pub mod update;
