// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod command;
pub mod project;
#[cfg(feature = "backend")]
mod guard;
#[cfg(feature = "backend")]
mod handle;
#[cfg(feature = "backend")]
mod core;
#[cfg(all(feature = "backend", test))]
mod tests;

pub use command::{InsertPosition, ModelCommand};

#[cfg(feature = "backend")]
pub use handle::ShowModelHandle;
#[cfg(feature = "backend")]
pub use core::*;
