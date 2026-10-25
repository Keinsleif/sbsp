// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use crate::AppState;
use sbsp_backend::controller::ControllerCommand;

#[tauri::command]
pub async fn send_controller_command(
    state: tauri::State<'_, AppState>,
    command: ControllerCommand,
) -> Result<(), String> {
    if let Some(handle) = state.get_handle().await {
        handle
            .controller_handle
            .send_command(command)
            .await
            .map_err(|e| e.to_string())
    } else {
        Err("Not connected.".into())
    }
}
