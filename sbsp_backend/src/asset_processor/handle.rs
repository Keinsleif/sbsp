use std::{path::PathBuf};

use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::asset_processor::ProcessResult;

use super::{AssetProcessorCommand, AssetData};

#[derive(Clone)]
pub struct AssetProcessorHandle {
    pub(crate) result_rx_factory: broadcast::Sender<ProcessResult>,
    pub(crate) command_tx: mpsc::Sender<AssetProcessorCommand>,
}

impl AssetProcessorHandle {
    pub async fn request_file_asset_data(&self, target: PathBuf) -> Result<AssetData, String> {
        let id = Uuid::now_v7();
        self.command_tx
            .send(AssetProcessorCommand::RequestFileAssetData {
                id,
                path: target,
            })
            .await
            .unwrap();

        loop {
            if let Ok(result) = self.result_rx_factory.subscribe().recv().await
            && result.id == id {
                break result.data;
            }
        }
    }

    pub async fn request_process_all(&self) {
        self.command_tx.send(AssetProcessorCommand::ProcessAll).await.unwrap();
    }
}
