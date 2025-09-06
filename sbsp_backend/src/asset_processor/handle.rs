use std::path::PathBuf;

use tokio::sync::{mpsc, oneshot};

use super::{AssetCommand, AssetData};

pub struct AssetProcessorHandle {
    pub(super) command_tx: mpsc::Sender<AssetCommand>,
}

impl AssetProcessorHandle {
    pub async fn request_file_asset_data(&self, target: PathBuf) -> Result<AssetData, String> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(AssetCommand::RequestFileAssetData {
                path: target,
                responder: result_tx,
            })
            .await
            .unwrap();

        result_rx
            .await
            .unwrap_or_else(|_| Err("AssetProcessor task may have panicked".to_string()))
    }
}
