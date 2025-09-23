mod command;

use sbsp_backend::{controller::state::ShowState, event::UiEvent, start_backend};
use tauri::{
    AppHandle, Emitter, Manager as _,
    menu::{MenuBuilder, MenuId, MenuItem, SubmenuBuilder},
};
use tokio::sync::{broadcast, watch};

use crate::command::{
    controller::{
        go, load, pause, pause_all, resume, resume_all, seek_by, seek_to, set_playback_cursor,
        stop, stop_all, toggle_repeat,
    },
    file_open, file_save, file_save_as,
    model_manager::{
        add_cue, add_cues, get_show_model, move_cue, remove_cue, renumber_cues, update_cue,
        update_settings,
    },
    process_asset,
};

async fn forward_backend_state_and_event(
    app_handle: AppHandle,
    mut state_rx: watch::Receiver<ShowState>,
    mut event_rx: broadcast::Receiver<UiEvent>,
) {
    loop {
        tokio::select! {
            Ok(_) = state_rx.changed() => {
                let state = state_rx.borrow().clone();
                app_handle.emit("backend-state-update", state).ok();
            },
            Ok(event) = event_rx.recv() => {
                app_handle.emit("backend-event", event).ok();
            }
            else => break,
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_denylist(&["settings"])
                .build(),
        )
        .setup(|app| {
            let app_handle = app.handle();

            let file_menu = SubmenuBuilder::new(app, "File")
                .items(&[
                    &MenuItem::with_id(
                        app_handle,
                        MenuId::new("id_open"),
                        "Open",
                        true,
                        Some("Ctrl+O"),
                    )?,
                    &MenuItem::with_id(
                        app_handle,
                        MenuId::new("id_save"),
                        "Save",
                        true,
                        Some("Ctrl+S"),
                    )?,
                    &MenuItem::with_id(
                        app_handle,
                        MenuId::new("id_save_as"),
                        "Save As...",
                        true,
                        Some("Ctrl+Shift+S"),
                    )?,
                ])
                .separator()
                .text(MenuId::new("id_quit"), "Quit")
                .build()?;
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .cut()
                .copy()
                .paste()
                .item(&MenuItem::with_id(
                    app,
                    MenuId::new("id_delete"),
                    "Delete",
                    true,
                    Some("Ctrl+Backspace"),
                )?)
                .select_all()
                .build()?;
            let tools_menu = SubmenuBuilder::new(app, "Tools")
                .item(&MenuItem::with_id(
                    app,
                    MenuId::new("id_renumber"),
                    "Renumber selected cues",
                    true,
                    Some("Ctrl+R"),
                )?)
                .build()?;
            let menu = MenuBuilder::new(app)
                .items(&[&file_menu, &edit_menu, &tools_menu])
                .build()?;
            app.set_menu(menu)?;

            let (backend_handle, state_rx, event_tx) = start_backend();

            tauri::async_runtime::spawn(forward_backend_state_and_event(
                app_handle.clone(),
                state_rx,
                event_tx.subscribe(),
            ));

            app.manage(backend_handle);

            Ok(())
        })
        .on_menu_event(|handle, event| match event.id().as_ref() {
            "id_open" => {
                file_open(handle.clone());
            }
            "id_save" => {
                file_save(handle.clone());
            }
            "id_save_as" => {
                file_save_as(handle.clone());
            }
            "id_quit" => {
                handle.cleanup_before_exit();
                std::process::exit(0);
            }
            "id_delete" | "id_renumber" => {
                let _ = handle.emit("menu_clicked", event.id());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            go,
            pause,
            resume,
            stop,
            pause_all,
            resume_all,
            stop_all,
            load,
            seek_to,
            seek_by,
            get_show_model,
            set_playback_cursor,
            toggle_repeat,
            update_cue,
            add_cue,
            add_cues,
            remove_cue,
            move_cue,
            renumber_cues,
            update_settings,
            process_asset,
            file_open,
            file_save,
            file_save_as,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
