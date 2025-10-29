use sbsp_backend::{api::server::start_apiserver, start_backend};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let (backend_handle, state_rx, event_tx) = start_backend();
    let shutdown_tx = start_apiserver(
        5800,
        backend_handle,
        state_rx,
        event_tx,
        Some("SBSP API Server".into()),
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
