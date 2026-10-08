// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Domain-specific TEA modules (see the migration plan, section 4.3):
//! each owns its own view logic and `Message` type, nested into the app's
//! via `.map()`, the same pattern used for `widgets::toast`.

pub mod cue_list;
