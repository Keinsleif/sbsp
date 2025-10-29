use serde::Serialize;
use tauri::{AppHandle, State, ipc::Channel};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Updater(#[from] tauri_plugin_updater::Error),
    #[error("there is no pending update")]
    NoPendingUpdate,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(
    tag = "event",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DownloadEvent {
    Started { content_length: Option<u64> },
    Progress { chunk_length: usize },
    Finished,
}

#[derive(Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadata {
    version: String,
    current_version: String,
}

#[tauri::command]
pub async fn fetch_update(
    app: AppHandle,
    pending_update: State<'_, PendingUpdate>,
) -> Result<Option<UpdateMetadata>> {
    let update = app.updater()?.check().await?;

    let update_metadata = update.as_ref().map(|update| UpdateMetadata {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
    });

    *pending_update.0.lock().await = update;

    Ok(update_metadata)
}

#[tauri::command]
pub async fn install_update(
    pending_update: State<'_, PendingUpdate>,
    on_event: Channel<DownloadEvent>,
) -> Result<()> {
    let Some(update) = pending_update.0.lock().await.take() else {
        return Err(Error::NoPendingUpdate);
    };

    let mut started = false;

    update
        .download_and_install(
            |chunk_length, content_length| {
                if !started {
                    let _ = on_event.send(DownloadEvent::Started { content_length });
                    started = true;
                }

                let _ = on_event.send(DownloadEvent::Progress { chunk_length });
            },
            || {
                let _ = on_event.send(DownloadEvent::Finished);
            },
        )
        .await?;

    Ok(())
}

#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);
