// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::path::PathBuf;

use clap::Parser;
use sbsp_backend::{
    BackendAudioSettings, BackendSettings, api::{ApiServerOptions, server::start_apiserver}, start_backend
};
use tokio::sync::watch;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    file: Option<PathBuf>,

    #[arg(long, default_value_t = 5800)]
    port: u16,

    #[arg(short, long, default_value = "SBS Player API Server")]
    discovery: Option<String>,

    #[arg(long, long_about = "")]
    auth: Option<String>,

    #[arg(long)]
    advance_cursor_when_go: bool,

    #[arg(long)]
    copy_assets_when_add: bool,

    #[arg(long)]
    get_hardware: bool,

    #[args(long)]
    device_id: Option<String>,

    #[args(long)]
    channel_count: Option<u16>,

    #[args(long)]
    sample_rate: Option<u32>,

    #[args(long)]
    buffer_size: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let args = Args::parse();
    let (_, settings_rx) = watch::channel(BackendSettings {
        advance_cursor_when_go: args.advance_cursor_when_go,
        copy_assets_when_add: args.copy_assets_when_add,
        audio: BackendAudioSettings {
            device_id: args.device_id,
            channel_count: args.channel_count,
            sample_rate: args.channel_count,
            buffer_size: args.buffer_size
        },
    });

    let (backend_handle, state_rx, event_tx) = match start_backend(settings_rx, false) {
        Ok(backends) => backends,
        Err(e) => {
            log::error!("{}", e);
            return Err(anyhow::anyhow!("{}", e));
        }
    };

    if let Some(path) = args.file {
        backend_handle.model_handle.load_from_file(path).await?;
    }

    let shutdown_tx = start_apiserver(
        backend_handle,
        state_rx,
        event_tx,
        ApiServerOptions {
            port: args.port,
            discoverry: args.discovery,
            password: args.password,
        },
    )
    .await?;

    shutdown_signal().await;

    shutdown_tx.send(())?;

    shutdown_tx.closed().await;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
