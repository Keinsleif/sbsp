// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! `web_dist/` is where the web build's output goes, and the host embeds
//! whatever is there (`src/host/web_assets.rs`). `rust-embed` refuses to
//! compile when the folder is missing, so make sure it exists: that way
//! `cargo check` works without building the web target first.

fn main() {
    // `backend` is on when the app has a backend to talk to: the host's own,
    // or a remote's connection to one. Most of the app's backend handling is
    // the same for both, and is gated on this instead of on `host`.
    println!("cargo:rustc-check-cfg=cfg(backend)");
    // `client` is on when the app connects to a host over the network (the
    // remote; the browser build joins it with its own connection).
    println!("cargo:rustc-check-cfg=cfg(client)");
    if std::env::var_os("CARGO_FEATURE_REMOTE").is_some() {
        println!("cargo:rustc-cfg=client");
    }
    if std::env::var_os("CARGO_FEATURE_HOST").is_some()
        || std::env::var_os("CARGO_FEATURE_REMOTE").is_some()
    {
        println!("cargo:rustc-cfg=backend");
    }

    let _ = std::fs::create_dir_all("web_dist");
    println!("cargo:rerun-if-changed=web_dist");
}
