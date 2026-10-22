// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use sbsp_backend::controller::ControllerCommand;

use crate::AppState;

#[tauri::command]
pub async fn send_controller_command(
    state: tauri::State<'_, AppState>,
    command: ControllerCommand,
) -> Result<(), String> {
    let handle = state.get_handle();
    handle
        .controller_handle
        .send_command(command)
        .await
        .map_err(|e| e.to_string())
}
