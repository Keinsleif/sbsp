// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(feature = "backend")]
mod cache;
mod command;
mod data;
#[cfg(feature = "backend")]
mod handle;
#[cfg(feature = "backend")]
mod processor;

pub use command::AssetProcessorCommand;
pub use data::{AssetData, AssetMetadata};

#[cfg(feature = "backend")]
pub use handle::AssetProcessorHandle;
#[cfg(feature = "backend")]
pub use processor::*;
