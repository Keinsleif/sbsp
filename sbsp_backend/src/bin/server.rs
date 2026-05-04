use std::path::PathBuf;

use clap::Parser;
use sbsp_backend::{
    BackendSettings,
    api::{ApiServerOptions, server::start_apiserver},
    start_backend,
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

    #[arg(short, long)]
    password: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let args = Args::parse();
    let (_, settings_rx) = watch::channel(BackendSettings::default());

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
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
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
