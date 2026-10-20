// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The file dialogs and backend calls behind New / Open / Save / Save As /
//! Export. Ported from the file commands of `sbsp_app`.

use std::time::Duration;

use anyhow::anyhow;
use rfd::AsyncFileDialog;
use sbsp_backend::BackendHandle;

/// How long a save may take before it is reported as failed. The backend
/// takes the command from a queue and writes the file afterwards; the
/// modified flag going down is the only completion signal.
const SAVE_TIMEOUT: Duration = Duration::from_secs(10);
const SAVE_POLL: Duration = Duration::from_millis(20);

fn show_filter(dialog: AsyncFileDialog) -> AsyncFileDialog {
    dialog.add_filter("Show Model", &["sbsp"])
}

/// Asks for a show file and loads it. Cancelling does nothing.
pub async fn open(handle: BackendHandle) -> anyhow::Result<()> {
    let Some(file) = show_filter(AsyncFileDialog::new()).pick_file().await else {
        return Ok(());
    };
    handle
        .model_handle
        .load_from_file(file.path().to_path_buf())
        .await
}

/// Saves the show: over its file when it has one (and `ask_for_path` is
/// off), else to a file chosen now. `Ok(false)`: the dialog was cancelled.
/// Returns once the file is written.
pub async fn save(handle: BackendHandle, ask_for_path: bool) -> anyhow::Result<bool> {
    let model = &handle.model_handle;
    let current = model.get_current_file_path().await;

    match (&current, ask_for_path) {
        (Some(_), false) => model.save().await?,
        _ => {
            let mut dialog = show_filter(AsyncFileDialog::new());
            if let Some(path) = &current {
                if let Some(directory) = path.parent() {
                    dialog = dialog.set_directory(directory);
                }
                if let Some(name) = path.file_name() {
                    dialog = dialog.set_file_name(name.to_string_lossy());
                }
            }
            let Some(file) = dialog.save_file().await else {
                return Ok(false);
            };
            model.save_as(file.path().to_path_buf()).await?;
        }
    }

    let waited = tokio::time::timeout(SAVE_TIMEOUT, async {
        while model.is_modified() {
            tokio::time::sleep(SAVE_POLL).await;
        }
    })
    .await;
    waited.map(|()| true).map_err(|_| anyhow!("saving timed out"))
}

/// Asks for a folder and exports the show with its assets there.
/// `Ok(false)`: cancelled.
pub async fn export(handle: BackendHandle) -> anyhow::Result<bool> {
    let model = &handle.model_handle;
    let mut dialog = AsyncFileDialog::new();
    if let Some(directory) = model
        .get_current_file_path()
        .await
        .as_deref()
        .and_then(|path| path.parent())
    {
        dialog = dialog.set_directory(directory);
    }
    let Some(folder) = dialog.pick_folder().await else {
        return Ok(false);
    };
    model.export_to_folder(folder.path().to_path_buf()).await?;
    Ok(true)
}
