// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod command;
pub mod state;
#[cfg(feature = "backend")]
mod handle;
#[cfg(feature = "backend")]
mod core;
#[cfg(all(feature = "backend", test))]
mod tests;

pub use command::ControllerCommand;

#[cfg(feature = "backend")]
pub use handle::CueControllerHandle;
#[cfg(feature = "backend")]
pub use core::*;
