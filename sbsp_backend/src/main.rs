// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::path::PathBuf;

use clap::Parser;
use sbsp_backend::{
    BackendAudioSettings, BackendSettings,
    api::{ApiServerOptions, PermissionInfo, server::start_apiserver},
    helper::get_supported_hardware,
    start_backend,
};
use termtree::Tree;
use tokio::sync::watch;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    file: Option<PathBuf>,

    #[arg(long, default_value_t = 5800)]
    port: u16,

    #[arg(short, long, default_value = "SBS Player API Server")]
    discovery: Option<String>,

    #[arg(
        long,
        long_help = "List of PermissionInfo string in '<password>:<permission>' format."
    )]
    auth: Vec<PermissionInfo>,

    #[arg(long)]
    copy_assets_when_add: bool,

    #[arg(long)]
    get_hardware: bool,

    #[arg(long)]
    device_id: Option<String>,

    #[arg(long)]
    channel_count: Option<u16>,

    #[arg(long)]
    sample_rate: Option<u32>,

    #[arg(long)]
    buffer_size: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let args = Args::parse();

    if args.get_hardware {
        let hardware = get_supported_hardware()?;
        let mut root = Tree::new("Supported Hardware".to_owned());
        for (id, device) in &hardware.devices {
            let is_default = if id == &hardware.default {
                " [Default]"
            } else {
                ""
            };
            let mut dev_node = Tree::new(format!("{} (ID: {}){is_default}", device.name, id));
            dev_node.push(format!(
                "Defaults: {} ch @ {} Hz",
                device.default_channel_count, device.default_sample_rate
            ));
            let mut configs_node = Tree::new("Frame Configs".to_string());
            for (i, config) in device.supported_configs.iter().enumerate() {
                let mut cfg_node = Tree::new(format!("Config #{}", i + 1));
                cfg_node.push(format!("Channels: {}", config.channel_count));

                let rates: Vec<_> = config.sample_rates.iter().map(|r| r.to_string()).collect();
                cfg_node.push(format!("Sample Rates: [{}] Hz", rates.join(", ")));

                let buffers: Vec<_> = config.buffer_sizes.iter().map(|b| b.to_string()).collect();
                cfg_node.push(format!("Buffer Sizes: [{}]", buffers.join(", ")));

                configs_node.push(cfg_node);
            }
            dev_node.push(configs_node);
            root.push(dev_node);
        }
        println!("{}", root);
        return Ok(());
    }

    let (_, settings_rx) = watch::channel(BackendSettings {
        copy_assets_when_add: args.copy_assets_when_add,
        audio: BackendAudioSettings {
            device_id: args.device_id,
            channel_count: args.channel_count,
            sample_rate: args.sample_rate,
            buffer_size: args.buffer_size,
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
            auth_map: args.auth,
        },
    )
    .await?;

    shutdown_signal().await;

    log::info!("Shutting down server...");

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
