use sbsp_backend::{api::server::run, start_backend};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let (backend_handle, state_rx, event_tx) = start_backend();

    run(8888, backend_handle, state_rx, event_tx, true).await?;
    Ok(())
}
