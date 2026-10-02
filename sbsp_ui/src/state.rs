// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Application state (see the migration plan, section 4.3). Unlike
//! `domain/`, these types own mutable state and have `&mut self` methods;
//! `domain/` functions are called from inside them where the actual logic
//! is pure data transformation.

pub mod model;
pub mod ui;
