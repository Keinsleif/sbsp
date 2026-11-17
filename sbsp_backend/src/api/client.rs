// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(feature = "client")]
mod core;
#[cfg(feature = "client")]
mod file_list_handler;
mod service_entry;

#[cfg(feature = "client")]
pub use core::*;
#[cfg(feature = "client")]
pub use file_list_handler::FileListHandle;
pub use service_entry::ServiceEntry;
