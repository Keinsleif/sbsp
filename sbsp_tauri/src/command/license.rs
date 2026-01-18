use tauri::{Manager as _, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

use sbsp_license::{LicenseManager, data::LicenseInformation};

#[tauri::command]
pub async fn activate_license(
    app_handle: tauri::AppHandle,
    license_manager: State<'_, LicenseManager>,
) -> Result<bool, String> {
    let (result_tx, result_rx) = oneshot::channel();
    app_handle
        .dialog()
        .file()
        .add_filter("License File", &["json"])
        .pick_file(|file_path_opt| {
            if let Some(file_path) = file_path_opt
                && let Some(path) = file_path.as_path()
            {
                result_tx.send(Some(path.to_path_buf())).unwrap();
            } else {
                result_tx.send(None).unwrap()
            }
        });
    if let Some(path) = result_rx.await.unwrap() {
        if let Err(e) = license_manager
            .activate_by_file(path.clone())
            .map_err(|e| e.to_string())
        {
            return Err(e);
        } else if let Ok(app_path) = app_handle.path().app_config_dir() {
            let license_path = app_path.join("license.json");
            let _ = tokio::fs::copy(path, license_path).await;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn get_license_info(license_manager: State<'_, LicenseManager>) -> Option<LicenseInformation> {
    license_manager.get_license_info()
}
