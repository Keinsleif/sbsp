use std::path::PathBuf;

use tokio::sync::mpsc;

use super::AssetProcessorCommand;

#[derive(Clone)]
pub struct AssetProcessorHandle {
    pub(crate) command_tx: mpsc::Sender<AssetProcessorCommand>,
}

impl AssetProcessorHandle {
    pub async fn request_file_asset_data(&self, target: PathBuf) {
        self.command_tx
            .send(AssetProcessorCommand::RequestFileAssetData { path: target })
            .await
            .unwrap();
    }
}
