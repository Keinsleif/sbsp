// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

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
