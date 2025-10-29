use tokio::sync::{mpsc, watch};

use crate::api::FileList;

#[derive(Clone, Debug)]
pub struct FileListHandle {
    file_list_rx: watch::Receiver<Vec<FileList>>,
    command_tx: mpsc::Sender<()>,
}

impl FileListHandle {
    pub fn new(file_list_rx: watch::Receiver<Vec<FileList>>, command_tx: mpsc::Sender<()>) -> Self {
        Self {
            file_list_rx,
            command_tx,
        }
    }

    pub async fn request_file_list(&self) -> anyhow::Result<()> {
        log::debug!("File Listing reqested.");
        self.command_tx.send(()).await?;
        Ok(())
    }

    pub async fn recv_file_list(&mut self) -> anyhow::Result<Vec<FileList>> {
        self.file_list_rx.changed().await?;
        Ok(self.file_list_rx.borrow().clone())
    }
}
