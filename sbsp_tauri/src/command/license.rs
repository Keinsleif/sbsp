use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

use sbsp_license::LicenseManager;


#[tauri::command]
pub async fn activate_license(app_handle: tauri::AppHandle, state: State<'_, LicenseManager>) -> Result<(), String> {
    let (result_tx, result_rx) = oneshot::channel();
    app_handle.dialog().file().add_filter("License File", &["json"]).pick_file(|file_path_opt| {
        if let Some(file_path) = file_path_opt && let Some(path) = file_path.as_path() {
            result_tx.send(Some(path.to_path_buf())).unwrap();
        } else {
            result_tx.send(None).unwrap()
        }
    });
    if let Some(path) = result_rx.await.unwrap() {
        state.activate_by_file(path).map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}