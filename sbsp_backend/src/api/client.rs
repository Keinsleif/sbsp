// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod service_entry;
#[cfg(feature = "client")]
mod file_list_handler;
#[cfg(feature = "client")]
mod core;

pub use service_entry::ServiceEntry;
#[cfg(feature = "client")]
pub use file_list_handler::FileListHandle;
#[cfg(feature = "client")]
pub use core::*;
