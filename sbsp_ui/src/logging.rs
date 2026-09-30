// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A minimal `log` + `fern` logger, shared by the host and remote binaries.
//!
//! `log::info!`/`log::error!`/etc. are no-ops until some logger
//! implementation calls `log::set_logger` -- this is that implementation.
//! Currently stderr-only; file output (see the migration plan, section 5.2:
//! `dirs::data_local_dir()/<identifier>/logs`, or `~/Library/Logs/<identifier>`
//! on macOS) is Phase 7 work, once there's an identifier/paths module to
//! hang it off of.
//!
//! Not used by the web target: stdio isn't connected to the browser console
//! on wasm32, so Phase 9 will need a separate `console_log`-based setup
//! there instead of this module.

/// Installs the logger. Should be called once, at the very top of `main()`,
/// before anything that might log (in particular, before starting the
/// backend).
pub fn init() {
    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    let result = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stderr())
        .apply();

    if let Err(e) = result {
        // Only fails if a logger was already installed; fall back to
        // stderr directly so this isn't silently swallowed.
        eprintln!("failed to initialize logger: {e}");
    }
}
