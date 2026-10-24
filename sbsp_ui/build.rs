// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! `web_dist/` is where the web build's output goes, and the host embeds
//! whatever is there (`src/host/web_assets.rs`). `rust-embed` refuses to
//! compile when the folder is missing, so make sure it exists: that way
//! `cargo check` works without building the web target first.

fn main() {
    let _ = std::fs::create_dir_all("web_dist");
    println!("cargo:rerun-if-changed=web_dist");
}
