// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod command;
#[cfg(feature = "backend")]
mod core;
#[cfg(feature = "backend")]
mod handle;
pub mod state;
#[cfg(all(feature = "backend", test))]
mod tests;

pub use command::ControllerCommand;

#[cfg(feature = "backend")]
pub use self::core::*;
#[cfg(feature = "backend")]
pub use handle::CueControllerHandle;
