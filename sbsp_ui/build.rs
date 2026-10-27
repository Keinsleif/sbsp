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
    // The browser build connects to a host like the remote does; only on
    // wasm32, so `--features web` still builds and tests on the desktop
    // (without a connection) as before.
    let web_connection = std::env::var_os("CARGO_FEATURE_WEB").is_some()
        && std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|arch| arch == "wasm32");
    if std::env::var_os("CARGO_FEATURE_HOST").is_some()
        || std::env::var_os("CARGO_FEATURE_REMOTE").is_some()
        || web_connection
    {
        println!("cargo:rustc-cfg=backend");
    }

    // `client` is on when the app connects to a host over the network (the
    // remote; the browser build joins it with its own connection).
    println!("cargo:rustc-check-cfg=cfg(client)");
    if std::env::var_os("CARGO_FEATURE_REMOTE").is_some() || web_connection {
        println!("cargo:rustc-cfg=client");
    }

    // `installer` is on for the desktop apps, the ones that are installed
    // by a package and update themselves (Velopack).
    println!("cargo:rustc-check-cfg=cfg(installer)");
    if std::env::var_os("CARGO_FEATURE_HOST").is_some()
        || std::env::var_os("CARGO_FEATURE_REMOTE").is_some()
    {
        println!("cargo:rustc-cfg=installer");
    }

    let _ = std::fs::create_dir_all("web_dist");
    // Only the host embeds `web_dist/`. Watching it for the browser build
    // would rerun this on every trunk output, and with it the build.
    if std::env::var_os("CARGO_FEATURE_HOST").is_some() {
        println!("cargo:rerun-if-changed=web_dist");
    } else {
        println!("cargo:rerun-if-changed=build.rs");
    }
}
