use sbsp_backend::{apiserver::create_api_router, start_backend};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let (backend_handle, state_rx, event_tx) = start_backend();

    let app = create_api_router(
        backend_handle.controller_handle.clone(),
        state_rx,
        event_tx,
        backend_handle.model_handle.clone(),
    )
    .await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8888").await?;
    log::info!("ApiServer listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
