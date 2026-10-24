use serial_test::serial;

use super::*;

// Every test here is #[serial]: all of them call fl!(), which reads
// the single process-wide i18n::LOADER static, and
// menu_toggle_language_switches_state calls i18n::select to mutate it.
// Without #[serial], cargo test's default parallel execution could
// interleave that mutation with any other test's fl!() calls here,
// which (depending on timing) would render the wrong language or just
// flake intermittently. Tag any new test added to this module the
// same way unless it provably never touches fl!()/i18n::LOADER.

/// `State::new`'s boot `Task` is dropped: `iced_test`'s `Simulator`
/// only drives a `view()` `Element` through simulated input, not a
/// running `Task`/`Subscription` executor, so there is nothing to poll
/// it with here regardless. Run with no features (`cargo test -p
/// sbsp_ui`, no `--features`), so this never attempts the `host`-only
/// backend boot in the first place.
///
/// The target is "test", which has no storage folder, so quitting in a
/// test never writes the developer's real UI state file.
fn new_state() -> State {
    State::new("test", crate::persist::Saved::default()).0
}

#[test]
#[serial]
fn menu_toggle_theme_cycles_light_dark_system() {
    let mut state = new_state();
    assert_eq!(state.theme_mode, ThemeMode::System);

    let mut ui = iced_test::simulator(state.view());
    ui.click("View").expect("the View menu should be clickable");
    ui.click("Toggle Theme")
        .expect("the Toggle Theme item should be clickable");

    for message in ui.into_messages() {
        let _ = state.update(message);
    }

    assert_eq!(state.theme_mode, ThemeMode::Dark);
}

/// Deliberately does not assert on any `fl!()`-rendered text after the
/// switch (e.g. that the item now reads "English", or that other
/// labels appear in Japanese): `i18n::LOADER` is one process-wide
/// `static`, and `cargo test` runs tests in parallel on separate
/// threads by default, all sharing it. Asserting on rendered text here
/// would be racing every other test in this module that calls `fl!()`
/// expecting English (`menu_about_shows_a_toast`,
/// `shell_skeleton_renders_its_placeholder_text`) -- this test could
/// flip the language out from under one of them mid-run, or one of
/// them could run while this test's own assertion expects Japanese.
/// `state.lang` itself is this test's own field, not shared, so that
/// part is safe to check same as `theme_mode` above.
#[test]
#[serial]
fn menu_toggle_language_switches_state() {
    let mut state = new_state();
    assert_eq!(state.lang, Language::En);

    let mut ui = iced_test::simulator(state.view());
    ui.click("View").expect("the View menu should be clickable");
    // The item shows the language it switches *to*; starting from
    // English that's its own name for Japanese, not "Japanese".
    ui.click("日本語")
        .expect("the language item should be clickable");

    for message in ui.into_messages() {
        let _ = state.update(message);
    }

    assert_eq!(state.lang, Language::Ja);

    // Leave the shared LOADER back in English: #[serial] only
    // serializes this module's own tests against each other, not
    // against fl!()-calling tests elsewhere in the crate that may be
    // added later and aren't tagged to know about this one.
    crate::i18n::select(&Language::En.langid());
}

#[test]
#[serial]
fn menu_about_shows_a_toast() {
    let mut state = new_state();

    let mut ui = iced_test::simulator(state.view());
    ui.click("Help").expect("the Help menu should be clickable");
    ui.click("About")
        .expect("the About item should be clickable");

    for message in ui.into_messages() {
        let _ = state.update(message);
    }

    let mut ui = iced_test::simulator(state.view());
    // `Selector`'s `&str` impl matches a widget's full text exactly,
    // not a substring (learned by running this: an earlier version
    // here searched for just "iced UI", which failed even though the
    // toast clearly contained it).
    assert!(
        ui.find(fl!("menu-help-about-message").as_str()).is_ok(),
        "the About toast's message should be visible after the click"
    );
}

/// Three root wait cues named "Wait 1s"/"Wait 2s"/"Wait 3s" (distinct
/// names, since `Simulator` selects rows by their exact text).
fn populated_state() -> (State, [sbsp_backend::model::cue::Uuid; 3]) {
    use sbsp_backend::model::cue::{
        Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid, WaitCueParam,
    };

    let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let mut state = new_state();

    for (i, id) in ids.iter().enumerate() {
        state.model.cue_list.cues.insert(
            *id,
            Cue {
                id: *id,
                number: String::new(),
                name: None,
                notes: String::new(),
                color: CueColor::default(),
                pre_wait: 0.0,
                chain: CueChain::default(),
                treat_stop_as_completed: false,
                cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::default(),
                parent_id: None,
                params: CueParam::Wait(WaitCueParam {
                    duration: (i + 1) as f64,
                }),
            },
        );
    }
    state.model.cue_list.root_ids = ids.to_vec();

    (state, ids)
}

#[test]
#[serial]
fn empty_cue_list_shows_its_placeholder() {
    let state = new_state();
    let mut ui = iced_test::simulator(state.view());

    assert!(ui.find(fl!("cue-list-empty").as_str()).is_ok());
}

#[test]
#[serial]
fn clicking_a_cue_row_selects_it() {
    let (mut state, ids) = populated_state();

    let mut ui = iced_test::simulator(state.view());
    ui.click("Wait 2s")
        .expect("the second row should be clickable");
    for message in ui.into_messages() {
        let _ = state.update(message);
    }

    assert_eq!(state.ui.selected, Some(ids[1]));
    assert_eq!(state.ui.selected_rows, vec![ids[1]]);
}

#[test]
#[serial]
fn shift_clicking_selects_the_range_from_the_current_selection() {
    let (mut state, ids) = populated_state();
    state.ui.set_selected(ids[0], false);
    state.modifiers = keyboard::Modifiers::SHIFT;

    let mut ui = iced_test::simulator(state.view());
    ui.click("Wait 3s")
        .expect("the third row should be clickable");
    for message in ui.into_messages() {
        let _ = state.update(message);
    }

    assert_eq!(state.ui.selected_rows, ids.to_vec());
    assert_eq!(state.ui.selected, Some(ids[2]));
}

fn press(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Message {
    let press = hotkeys::KeyPress::new(&key, &key, modifiers, false)
        .expect("test keys are all bindable");
    Message::KeyPressed(press)
}

fn arrow(named: keyboard::key::Named) -> keyboard::Key {
    keyboard::Key::Named(named)
}

#[test]
#[serial]
fn clicking_the_cursor_column_moves_the_playback_cursor() {
    let (mut state, ids) = populated_state();

    let _ = state.update(Message::CueList(cue_list::Message::SetCursor(ids[2])));

    assert_eq!(state.ui.playback_cursor, Some(ids[2]));
}

#[test]
#[serial]
fn the_header_toggles_flip_the_time_display_modes() {
    use crate::state::ui::DisplayMode;

    let mut state = new_state();
    assert_eq!(state.ui.pre_wait_display_mode, DisplayMode::Elapsed);

    let _ = state.update(Message::CueList(cue_list::Message::TogglePreWaitDisplay));
    let _ = state.update(Message::CueList(cue_list::Message::ToggleDurationDisplay));

    assert_eq!(state.ui.pre_wait_display_mode, DisplayMode::Remain);
    assert_eq!(state.ui.duration_display_mode, DisplayMode::Remain);
}

#[test]
#[serial]
fn transport_buttons_show_outside_view_mode_only() {
    let mut state = new_state();
    state.ui.set_permission(sbsp_backend::api::Permissions::all());
    assert!(state.show_controls(), "shown in edit mode");

    state.ui.set_mode(Mode::View);
    assert!(!state.show_controls(), "hidden in view mode");
}

#[test]
#[serial]
fn hide_controls_setting_hides_the_transport_buttons() {
    let mut state = new_state();
    state.ui.set_permission(sbsp_backend::api::Permissions::all());
    state.hide_controls = true;

    assert!(!state.show_controls());
}

#[test]
#[serial]
fn window_focus_events_are_tracked() {
    let mut state = new_state();
    assert!(state.window_focused);

    let _ = state.update(Message::WindowFocused(false));
    assert!(!state.window_focused);
}

#[test]
#[serial]
fn selecting_a_mode_switches_it_when_allowed() {
    let mut state = new_state();
    // Without the `host` feature a state starts with no permissions.
    state.ui.set_permission(sbsp_backend::api::Permissions::all());
    assert_eq!(state.ui.mode, Mode::Edit);

    let _ = state.update(Message::StatusBar(status_bar::Message::ModeSelected(Mode::View)));
    assert_eq!(state.ui.mode, Mode::View);
}

#[test]
#[serial]
fn hotkeys_that_move_the_selection_still_work_in_view_mode() {
    use keyboard::key::Named;

    let (mut state, ids) = populated_state();
    state.ui.set_permission(sbsp_backend::api::Permissions::all());
    state.ui.set_mode(Mode::View);
    state.ui.set_selected(ids[0], false);

    let _ = state.update(press(arrow(Named::ArrowDown), keyboard::Modifiers::empty()));
    assert_eq!(state.ui.selected, Some(ids[1]));
}

#[test]
#[serial]
fn arrow_down_moves_the_selection() {
    use keyboard::key::Named;

    let (mut state, ids) = populated_state();
    state.ui.set_selected(ids[0], false);

    let _ = state.update(press(arrow(Named::ArrowDown), keyboard::Modifiers::empty()));
    assert_eq!(state.ui.selected, Some(ids[1]));
    assert_eq!(state.ui.selected_rows, vec![ids[1]]);

    let _ = state.update(press(arrow(Named::ArrowUp), keyboard::Modifiers::empty()));
    assert_eq!(state.ui.selected, Some(ids[0]));
}

#[test]
#[serial]
fn shift_arrow_extends_the_selection() {
    use keyboard::key::Named;

    let (mut state, ids) = populated_state();
    state.ui.set_selected(ids[0], false);

    let _ = state.update(press(arrow(Named::ArrowDown), keyboard::Modifiers::SHIFT));
    let _ = state.update(press(arrow(Named::ArrowDown), keyboard::Modifiers::SHIFT));

    assert_eq!(state.ui.selected_rows, ids.to_vec());
    assert_eq!(state.ui.selected, Some(ids[2]));
}

#[test]
#[serial]
fn ctrl_a_selects_every_row() {
    let (mut state, ids) = populated_state();

    let _ = state.update(press(
        keyboard::Key::Character("a".into()),
        keyboard::Modifiers::CTRL,
    ));

    assert_eq!(state.ui.selected_rows, ids.to_vec());
}

#[test]
#[serial]
fn an_unbound_key_changes_nothing() {
    let (mut state, ids) = populated_state();
    state.ui.set_selected(ids[1], false);

    let _ = state.update(press(
        keyboard::Key::Character("q".into()),
        keyboard::Modifiers::empty(),
    ));

    assert_eq!(state.ui.selected, Some(ids[1]));
}

#[test]
#[serial]
fn shell_skeleton_renders_the_footer_count() {
    let state = new_state();
    let mut ui = iced_test::simulator(state.view());

    assert!(
        ui.find(fl!("main-foot-bar-cue-count-suffix", n = 0_i64).as_str())
            .is_ok(),
        "the footer's cue count should render"
    );
}

#[test]
#[serial]
fn sidebar_renders_its_tabs_and_can_be_toggled() {
    let mut state = new_state();

    let mut ui = iced_test::simulator(state.view());
    assert!(ui.find(fl!("main-side-bar-active-cues").as_str()).is_ok());
    assert!(ui.find(fl!("main-side-bar-meter-title").as_str()).is_ok());
    drop(ui);

    let _ = state.update(Message::StatusBar(status_bar::Message::ToggleSidebar));
    assert!(!state.ui.is_right_sidebar_open);
    let mut ui = iced_test::simulator(state.view());
    assert!(ui.find(fl!("main-side-bar-meter-title").as_str()).is_err());
}

#[test]
#[serial]
fn selecting_the_meter_tab_switches_and_resets() {
    let mut state = new_state();
    state.meter.push(1.0, 1.0);

    let _ = state.update(Message::SideBar(side_bar::Message::SelectTab(
        SideBarTab::Meter,
    )));

    assert_eq!(state.ui.side_bar_tab, SideBarTab::Meter);
    assert!(state.meter.left.level_db() <= -59.0);
}

#[test]
#[serial]
fn the_footer_buttons_toggle_the_bottom_tab() {
    let mut state = new_state();
    assert!(state.ui.is_bottom_tab_open);

    let _ = state.update(Message::StatusBar(status_bar::Message::ToggleBottomTab));
    assert!(!state.ui.is_bottom_tab_open);

    // Both layouts still build.
    let _ = iced_test::simulator(state.view());
    let _ = state.update(Message::StatusBar(status_bar::Message::ToggleBottomTab));
    let _ = iced_test::simulator(state.view());
}

#[test]
#[serial]
fn a_seek_from_the_viewer_is_ignored_in_view_mode() {
    let mut state = new_state();
    state.ui.set_permission(sbsp_backend::api::Permissions::all());
    let _ = state.update(Message::StatusBar(status_bar::Message::ModeSelected(Mode::View)));

    // Nothing to assert on the Task itself; this guards the match arm
    // (and that a seek with no backend cannot panic).
    let _ = state.update(Message::Viewer(viewer::Message::Seek(
        sbsp_backend::model::cue::Uuid::nil(),
        1.0,
    )));
}

#[test]
#[serial]
fn a_field_edit_is_applied_when_submitted() {
    let mut state = new_state();
    let id = field::FieldId("renumber.start");

    let _ = state.update(Message::Field(field::Message::Input(id, "99".to_string())));
    assert_eq!(
        state.renumber.request().map(|r| r.start),
        Some(1),
        "typing alone changes nothing"
    );

    let _ = state.update(Message::Field(field::Message::Submit(id)));
    assert_eq!(state.renumber.request().map(|r| r.start), Some(99));
}

#[test]
#[serial]
fn escape_discards_a_field_edit() {
    let mut state = new_state();
    let id = field::FieldId("renumber.start");

    let _ = state.update(Message::Field(field::Message::Input(id, "9".to_string())));
    let _ = state.update(Message::Field(field::Message::Escape));
    let _ = state.update(Message::Field(field::Message::Submit(id)));

    assert_eq!(state.renumber.request().map(|r| r.start), Some(1));
}

fn editing_state() -> (State, [Uuid; 3]) {
    let (mut state, ids) = populated_state();
    state.ui.set_permission(sbsp_backend::api::Permissions::all());
    state.ui.set_mode(Mode::Edit);
    (state, ids)
}

fn mod_key(c: &str) -> Message {
    press(
        keyboard::Key::Character(c.into()),
        keyboard::Modifiers::CTRL,
    )
}

fn clipboard_ids(state: &State) -> Vec<Uuid> {
    state.clipboard.iter().map(|cue| cue.id).collect()
}

#[test]
#[serial]
fn copy_takes_the_selected_cues_top_first() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[2], false);
    state.ui.add_selected(ids[0], false);

    let _ = state.update(mod_key("c"));
    assert_eq!(clipboard_ids(&state), vec![ids[0], ids[2]]);

    // Nothing selected: the clipboard keeps what it had.
    state.ui.clear_selected(false);
    let _ = state.update(mod_key("c"));
    assert_eq!(clipboard_ids(&state), vec![ids[0], ids[2]]);
}

#[test]
#[serial]
fn copy_works_outside_edit_mode_but_cut_keeps_the_cues_there() {
    let (mut state, ids) = editing_state();
    state.ui.set_mode(Mode::Run);
    state.ui.set_selected(ids[1], false);

    let _ = state.update(mod_key("x"));
    assert_eq!(clipboard_ids(&state), vec![ids[1]]);
    assert!(state.model.cue_list.cues.contains_key(&ids[1]));
}

#[test]
#[serial]
fn a_right_click_selects_its_row_unless_it_is_in_the_selection() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);
    state.ui.add_selected(ids[1], false);

    // On a selected row the selection stays.
    let _ = state.update(Message::CueList(cue_list::Message::Hover(ids[1])));
    let _ = state.update(Message::CueList(cue_list::Message::RightClicked));
    assert_eq!(state.ui.selected_rows.len(), 2);

    // On another row it moves there.
    let _ = state.update(Message::CueList(cue_list::Message::HoverEnd(ids[1])));
    let _ = state.update(Message::CueList(cue_list::Message::Hover(ids[2])));
    let _ = state.update(Message::CueList(cue_list::Message::RightClicked));
    assert_eq!(state.ui.selected_rows, vec![ids[2]]);

    // Leaving another row does not forget the one now under the pointer.
    let _ = state.update(Message::CueList(cue_list::Message::Hover(ids[0])));
    let _ = state.update(Message::CueList(cue_list::Message::HoverEnd(ids[2])));
    assert_eq!(state.hovered_row, Some(ids[0]));
}

#[test]
#[serial]
fn the_context_menu_exists_only_in_edit_mode() {
    let (mut state, _) = editing_state();
    state.ui.set_mode(Mode::Run);
    // A right click outside edit mode selects nothing.
    let _ = state.update(Message::CueList(cue_list::Message::RightClicked));
    assert!(state.ui.selected_rows.is_empty());
}

fn cue_list_message(message: cue_list::Message) -> Message {
    Message::CueList(message)
}

#[test]
#[serial]
fn dragging_a_selected_row_takes_the_whole_selection() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);
    state.ui.add_selected(ids[2], false);

    let _ = state.update(cue_list_message(cue_list::Message::DragStart(ids[2])));
    let drag = state.cue_drag.as_ref().expect("a drag started");
    assert_eq!(drag.ids, vec![ids[0], ids[2]]);
    assert_eq!(drag.pointer, None);
}

#[test]
#[serial]
fn dragging_a_row_outside_the_selection_selects_only_that_row() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);

    let _ = state.update(cue_list_message(cue_list::Message::DragStart(ids[1])));
    assert_eq!(state.ui.selected_rows, vec![ids[1]]);
    assert_eq!(state.cue_drag.as_ref().map(|drag| drag.ids.clone()), Some(vec![ids[1]]));
}

#[test]
#[serial]
fn the_pointer_is_followed_while_dragging_and_forgotten_outside_the_list() {
    let (mut state, ids) = editing_state();
    let _ = state.update(cue_list_message(cue_list::Message::DragStart(ids[0])));

    let at = iced::Point::new(12.0, 34.0);
    let _ = state.update(cue_list_message(cue_list::Message::DragMoved(at)));
    assert_eq!(state.cue_drag.as_ref().and_then(|drag| drag.pointer), Some(at));
    let _ = state.update(cue_list_message(cue_list::Message::DragLeft));
    assert_eq!(state.cue_drag.as_ref().and_then(|drag| drag.pointer), None);
}

#[test]
#[serial]
fn escape_cancels_a_drag_instead_of_stopping_everything() {
    let (mut state, ids) = editing_state();
    let _ = state.update(cue_list_message(cue_list::Message::DragStart(ids[0])));

    let _ = state.update(press(
        keyboard::Key::Named(keyboard::key::Named::Escape),
        keyboard::Modifiers::empty(),
    ));
    assert!(state.cue_drag.is_none());
}

#[test]
#[serial]
fn a_drop_ends_the_drag_and_leaving_edit_mode_cancels_it() {
    let (mut state, ids) = editing_state();
    let _ = state.update(cue_list_message(cue_list::Message::DragStart(ids[0])));
    let _ = state.update(cue_list_message(cue_list::Message::DragDropped));
    assert!(state.cue_drag.is_none());

    let _ = state.update(cue_list_message(cue_list::Message::DragStart(ids[0])));
    let _ = state.update(Message::StatusBar(status_bar::Message::ModeSelected(Mode::Run)));
    assert!(state.cue_drag.is_none());
}

#[test]
#[serial]
fn only_edit_mode_starts_a_drag() {
    let (mut state, ids) = editing_state();
    state.ui.set_mode(Mode::Run);

    let _ = state.update(cue_list_message(cue_list::Message::DragStart(ids[0])));
    assert!(state.cue_drag.is_none());
}

fn ctrl_backspace() -> Message {
    press(
        keyboard::Key::Named(keyboard::key::Named::Backspace),
        keyboard::Modifiers::CTRL,
    )
}

#[test]
#[serial]
fn delete_asks_before_removing_and_cancel_keeps_everything() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[1], false);

    let _ = state.update(ctrl_backspace());
    assert_eq!(state.pending_remove, Some(vec![ids[1]]));

    let mut ui = iced_test::simulator(state.view());
    assert!(ui.find(fl!("dialog-message-remove-cue").as_str()).is_ok());
    ui.click(fl!("general-cancel").as_str())
        .expect("the dialog has a cancel button");
    for message in ui.into_messages() {
        let _ = state.update(message);
    }

    assert_eq!(state.pending_remove, None);
    assert_eq!(state.model.cue_list.cues.len(), 3, "nothing is removed locally");
}

#[test]
#[serial]
fn confirming_closes_the_dialog() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);

    let _ = state.update(ctrl_backspace());
    let _ = state.update(Message::RemoveConfirmed);

    assert_eq!(state.pending_remove, None);
}

#[test]
#[serial]
fn delete_needs_edit_mode_and_a_selection() {
    let (mut state, ids) = editing_state();

    let _ = state.update(ctrl_backspace());
    assert_eq!(state.pending_remove, None, "nothing selected");

    state.ui.set_selected(ids[0], false);
    state.ui.set_mode(Mode::Run);
    let _ = state.update(ctrl_backspace());
    assert_eq!(state.pending_remove, None, "not in edit mode");
}

#[test]
#[serial]
fn while_the_question_is_open_only_enter_and_escape_answer_it() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);
    let key = |named| press(keyboard::Key::Named(named), keyboard::Modifiers::empty());

    let _ = state.update(ctrl_backspace());
    let _ = state.update(key(keyboard::key::Named::ArrowDown));
    assert_eq!(state.ui.selected, Some(ids[0]), "other keys do nothing");
    assert!(state.pending_remove.is_some());

    let _ = state.update(key(keyboard::key::Named::Escape));
    assert_eq!(state.pending_remove, None);

    let _ = state.update(ctrl_backspace());
    let _ = state.update(key(keyboard::key::Named::Enter));
    assert_eq!(state.pending_remove, None);
}

#[test]
#[serial]
fn dropped_files_wait_for_the_batch_then_become_audio_cues() {
    let (mut state, _ids) = editing_state();

    let _ = state.update(Message::FileDropped("a.wav".into()));
    let _ = state.update(Message::FileDropped("b.txt".into()));
    assert_eq!(state.dropped_files.len(), 2, "nothing happens until settled");

    let _ = state.update(Message::DropsSettled);
    assert!(state.dropped_files.is_empty());
}

#[test]
#[serial]
fn drops_are_ignored_outside_edit_mode() {
    let (mut state, _ids) = editing_state();
    state.ui.set_mode(Mode::Run);

    let _ = state.update(Message::FileDropped("a.wav".into()));
    let _ = state.update(Message::DropsSettled);
    assert!(state.dropped_files.is_empty(), "the batch is discarded");
}

#[test]
#[serial]
fn a_waiting_question_takes_enter_and_escape_and_nothing_else() {
    let mut state = new_state();
    let key = |named| press(keyboard::Key::Named(named), keyboard::Modifiers::empty());

    state.confirm = Some(file_ops::Confirm::StopActive(file_ops::After::New));
    let _ = state.update(key(keyboard::key::Named::ArrowDown));
    assert!(state.confirm.is_some(), "other keys do nothing");

    let _ = state.update(key(keyboard::key::Named::Escape));
    assert_eq!(state.confirm, None);

    state.confirm = Some(file_ops::Confirm::StopActive(file_ops::After::New));
    let _ = state.update(key(keyboard::key::Named::Enter));
    assert_eq!(state.confirm, None, "yes: nothing else to ask without changes");
}

#[test]
#[serial]
fn the_dialog_buttons_answer_the_question() {
    let mut state = new_state();
    state.confirm = Some(file_ops::Confirm::SaveChanges(file_ops::After::Open));

    let mut ui = iced_test::simulator(state.view());
    assert!(ui.find(fl!("dialog-save-confirm-content").as_str()).is_ok());
    ui.click(fl!("dialog-save-confirm-cancel").as_str())
        .expect("the dialog has a cancel button");
    for message in ui.into_messages() {
        let _ = state.update(message);
    }
    assert_eq!(state.confirm, None);
}

#[test]
#[serial]
fn close_requests_wait_while_a_question_is_open() {
    let mut state = new_state();
    state.confirm = Some(file_ops::Confirm::SaveChanges(file_ops::After::Quit));

    let _ = state.update(Message::CloseRequested);
    assert_eq!(
        state.confirm,
        Some(file_ops::Confirm::SaveChanges(file_ops::After::Quit)),
        "neither answered nor replaced"
    );
}

#[test]
#[serial]
fn a_failed_save_is_reported_and_continues_nothing() {
    let mut state = new_state();
    let _ = state.update(Message::FileSaved(
        Err("disk full".to_owned()),
        Some(file_ops::After::Quit),
    ));
    assert_eq!(state.toasts.len(), 1);
}

#[test]
#[serial]
fn backend_failures_get_their_own_titles() {
    use sbsp_backend::event::BackendError as E;
    let save = failure_message(&E::SaveToFile {
        path: "a.sbsp".into(),
        message: "denied".to_owned(),
    })
    .unwrap();
    assert!(save.starts_with(&fl!("notification-failed-to-save")));
    assert!(save.ends_with("denied"));
    let fallback = |device, config| failure_message(&E::AudioOutputFallback { device, config });
    assert!(
        fallback(true, false)
            .unwrap()
            .starts_with(&fl!("notification-audio-output-fallback-device-summary"))
    );
    assert!(
        fallback(false, true)
            .unwrap()
            .starts_with(&fl!("notification-audio-output-fallback-config-summary"))
    );
    assert!(
        fallback(true, true)
            .unwrap()
            .starts_with(&fl!("notification-audio-output-fallback-both-summary"))
    );
    assert_eq!(fallback(false, false), None);
    assert_eq!(failure_message(&E::Custom { id: 1, message: "x".to_owned() }), None);
}

fn ctrl_r() -> Message {
    press(
        keyboard::Key::Character("r".into()),
        keyboard::Modifiers::CTRL,
    )
}

#[test]
#[serial]
fn renumber_opens_with_a_selection_in_edit_mode() {
    let (mut state, ids) = editing_state();

    let _ = state.update(ctrl_r());
    assert!(!state.renumber_open, "nothing selected");

    state.ui.set_selected(ids[0], false);
    state.ui.set_mode(Mode::Run);
    let _ = state.update(ctrl_r());
    assert!(!state.renumber_open, "not in edit mode");

    state.ui.set_mode(Mode::Edit);
    let _ = state.update(ctrl_r());
    assert!(state.renumber_open);
}

#[test]
#[serial]
fn the_renumber_dialog_closes_on_escape_and_cancel() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);
    let key = |named| press(keyboard::Key::Named(named), keyboard::Modifiers::empty());

    let _ = state.update(ctrl_r());
    let _ = state.update(key(keyboard::key::Named::ArrowDown));
    assert!(state.renumber_open, "other keys do nothing");
    assert_eq!(state.ui.selected, Some(ids[0]));
    let _ = state.update(key(keyboard::key::Named::Escape));
    assert!(!state.renumber_open);

    let _ = state.update(ctrl_r());
    let _ = state.update(Message::Renumber(renumber::Message::Cancel));
    assert!(!state.renumber_open);
}

#[test]
#[serial]
fn running_closes_the_dialog_and_the_next_opening_starts_over() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);

    let id = field::FieldId("renumber.start");
    let _ = state.update(ctrl_r());
    let _ = state.update(Message::Field(field::Message::Input(id, "7".to_owned())));
    let _ = state.update(Message::Renumber(renumber::Message::Run));
    assert!(!state.renumber_open);

    let _ = state.update(ctrl_r());
    assert!(state.renumber_open);
    assert_eq!(state.renumber, renumber::Dialog::default());
}

#[test]
#[serial]
fn running_takes_in_the_field_still_being_edited() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);
    let id = field::FieldId("renumber.start");

    let _ = state.update(ctrl_r());
    let _ = state.update(Message::Field(field::Message::Input(id, "7".to_owned())));
    let _ = state.update(Message::Renumber(renumber::Message::Run));

    assert!(!state.renumber_open);
    assert_eq!(state.renumber.request().map(|r| r.start), Some(7));
}

#[test]
#[serial]
fn the_renumber_dialog_renders_only_when_open() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);
    let title = fl!("dialog-renumber-title");

    {
        let mut ui = iced_test::simulator(state.view());
        assert!(ui.find(title.as_str()).is_err());
    }
    let _ = state.update(ctrl_r());
    let mut ui = iced_test::simulator(state.view());
    assert!(ui.find(title.as_str()).is_ok());
}

#[test]
#[serial]
fn adding_a_group_opens_it_only_when_rows_move_into_it() {
    let (mut state, ids) = editing_state();

    let _ = state.update(Message::Header(header::Message::AddCue(CueKind::Group)));
    assert!(state.ui.expanded_rows.is_empty(), "nothing selected, nothing to open");

    state.ui.set_selected(ids[0], false);
    let _ = state.update(Message::Header(header::Message::AddCue(CueKind::Group)));
    assert_eq!(state.ui.expanded_rows.len(), 1);
}

#[test]
#[serial]
fn the_add_buttons_do_nothing_outside_edit_mode() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);
    state.ui.set_mode(Mode::Run);

    let _ = state.update(Message::Header(header::Message::AddCue(CueKind::Group)));
    assert!(state.ui.expanded_rows.is_empty());
}

#[test]
#[serial]
fn the_editor_replaces_the_viewer_in_edit_mode() {
    let (mut state, ids) = editing_state();
    state.ui.is_bottom_tab_open = true;

    {
        let mut ui = iced_test::simulator(state.view());
        assert!(
            ui.find(fl!("main-bottom-editor-no-selection").as_str()).is_ok(),
            "nothing is selected"
        );
    }

    state.ui.set_selected(ids[0], false);
    {
        let mut ui = iced_test::simulator(state.view());
        assert!(ui.find(fl!("main-bottom-editor-basics-title").as_str()).is_ok());
        assert!(ui.find(fl!("main-number").as_str()).is_ok());
    }

    state.ui.set_mode(Mode::Run);
    let mut ui = iced_test::simulator(state.view());
    assert!(ui.find(fl!("main-number").as_str()).is_err(), "no editor outside edit mode");
}

#[test]
#[serial]
fn the_trigger_tab_can_be_opened() {
    let (mut state, ids) = editing_state();
    state.ui.is_bottom_tab_open = true;
    state.ui.set_selected(ids[0], false);

    let mut ui = iced_test::simulator(state.view());
    ui.click(fl!("main-bottom-editor-trigger-title").as_str())
        .expect("the tab is a button");
    for message in ui.into_messages() {
        let _ = state.update(message);
    }

    let mut ui = iced_test::simulator(state.view());
    assert!(
        ui.find(fl!("main-bottom-editor-trigger-treat-stop-as-completed").as_str())
            .is_ok()
    );
}

#[test]
#[serial]
fn an_edit_belongs_to_the_cue_selected_when_it_began() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);

    let number = field::FieldId("editor.number");
    let _ = state.update(Message::Editor(editor::Message::Field(
        field::Message::Input(number, "9".to_string()),
    )));
    state.ui.set_selected(ids[1], false);

    assert_eq!(state.editor.draft_cue, Some(ids[0]));
}

#[test]
#[serial]
fn a_name_the_user_gave_is_what_the_list_shows() {
    let (mut state, ids) = populated_state();
    state.model.cue_list.cues.get_mut(&ids[0]).unwrap().name = Some("Opening".to_string());

    let mut ui = iced_test::simulator(state.view());
    assert!(ui.find("Opening").is_ok());
}
#[test]
#[serial]
fn loaded_settings_reach_the_ui() {
    use sbsp_frontend_settings::DarkMode;

    let mut state = new_state();
    let mut loaded = Preferences::default();
    loaded.general.lock_cursor_to_selection = !state.lock_cursor_to_selection;
    loaded.general.seek_amount = 7.5;
    loaded.appearance.dark_mode = DarkMode::Light;
    loaded.appearance.language = Some("ja".to_owned());
    loaded.appearance.hide_controls = !state.hide_controls;
    loaded.hotkey.file.open = Some("$mod+Shift+O".to_owned());
    loaded.name_format.audio = "custom".to_owned();

    state.apply_preferences(&loaded);

    assert_eq!(state.lock_cursor_to_selection, loaded.general.lock_cursor_to_selection);
    assert_eq!(state.seek_amount, 7.5);
    assert_eq!(state.theme_mode, ThemeMode::Light);
    assert_eq!(state.lang, Language::Ja);
    assert_eq!(state.hide_controls, loaded.appearance.hide_controls);
    assert_eq!(state.name_format.audio, "custom");
    assert_eq!(
        state
            .hotkeys
            .shortcut(hotkeys::Action::Open, hotkeys::is_mac())
            .as_deref(),
        Some(if hotkeys::is_mac() { "⇧⌘O" } else { "Ctrl+Shift+O" })
    );
    // The language took effect, not only the field.
    assert_eq!(fl!("menu-file-quit"), "終了");

    // Back to what the other tests expect.
    state.apply_preferences(&Preferences::default());
    crate::i18n::select(&Language::En.langid());
}

#[test]
#[serial]
fn menus_show_the_bindings() {
    let state = new_state();
    let spec = state.menu_spec();
    let (_, file) = &spec.menus[0];
    let shortcut_of = |wanted: &str| {
        file.iter().find_map(|node| match node {
            MenuNode::Item { id, shortcut, .. } if id.0 == wanted => Some(shortcut.clone()),
            _ => None,
        })
    };
    let save = if hotkeys::is_mac() { "⌘S" } else { "Ctrl+S" };
    assert_eq!(shortcut_of("file.save"), Some(Some(save.to_owned())));
    assert_eq!(shortcut_of("file.export"), Some(None), "unbound by default");
}

#[test]
#[serial]
fn the_cog_opens_the_settings_dialog_with_the_current_values() {
    let mut state = new_state();
    state.model.name = "Evening".to_owned();
    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));

    let dialog = state.settings_dialog.as_ref().expect("the dialog opens");
    assert_eq!(dialog.show_name, "Evening");
    assert_eq!(dialog.preferences.general, state.preferences.general);
}

#[test]
#[serial]
fn the_settings_dialog_closes_on_escape_and_cancel_without_applying() {
    let mut state = new_state();
    let before = state.lock_cursor_to_selection;
    let key = |named| press(keyboard::Key::Named(named), keyboard::Modifiers::empty());

    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    let _ = state.update(Message::Settings(settings_dialog::Message::LockCursor(!before)));
    let _ = state.update(key(keyboard::key::Named::ArrowDown));
    assert!(state.settings_dialog.is_some(), "other keys do nothing");
    let _ = state.update(key(keyboard::key::Named::Escape));
    assert!(state.settings_dialog.is_none());
    assert_eq!(state.lock_cursor_to_selection, before);

    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    let _ = state.update(Message::Settings(settings_dialog::Message::Cancel));
    assert!(state.settings_dialog.is_none());
    assert_eq!(state.lock_cursor_to_selection, before);
}

#[test]
#[serial]
fn done_applies_the_edited_settings() {
    let mut state = new_state();
    let before = state.lock_cursor_to_selection;

    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    let _ = state.update(Message::Settings(settings_dialog::Message::LockCursor(!before)));
    let _ = state.update(Message::Settings(settings_dialog::Message::DarkMode(
        sbsp_frontend_settings::DarkMode::Dark,
    )));
    let _ = state.update(Message::Settings(settings_dialog::Message::Done));

    assert!(state.settings_dialog.is_none());
    assert_eq!(state.lock_cursor_to_selection, !before);
    assert_eq!(state.theme_mode, ThemeMode::Dark);
    assert_eq!(state.preferences.general.lock_cursor_to_selection, !before);
}

#[test]
#[serial]
fn done_takes_the_field_being_edited() {
    let mut state = new_state();
    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    let _ = state.update(Message::Settings(settings_dialog::Message::SelectTab(
        settings_dialog::Tab::GlobalGeneral,
    )));
    let id = field::FieldId("settings.seek-amount");
    let _ = state.update(Message::Field(field::Message::Input(id, "7".to_owned())));
    let _ = state.update(Message::Settings(settings_dialog::Message::Done));

    assert_eq!(state.seek_amount, 7.0);
}

#[test]
#[serial]
fn the_settings_dialog_shows_every_tab() {
    let mut state = new_state();
    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    for tab in [
        settings_dialog::Tab::Preset,
        settings_dialog::Tab::ShowGeneral,
        settings_dialog::Tab::AudioLogic,
        settings_dialog::Tab::GlobalGeneral,
        settings_dialog::Tab::Appearance,
        settings_dialog::Tab::Hotkey,
        settings_dialog::Tab::Template,
        settings_dialog::Tab::NameFormat,
    ] {
        let _ = state.update(Message::Settings(settings_dialog::Message::SelectTab(tab)));
        let _ = iced_test::simulator(state.view());
    }
}

#[test]
#[serial]
fn the_edit_menu_has_the_settings_import_and_export() {
    let state = new_state();
    let spec = state.menu_spec();
    let (_, edit) = &spec.menus[1];
    for wanted in ["edit.import-settings", "edit.export-settings"] {
        assert!(
            edit.iter()
                .any(|node| matches!(node, MenuNode::Item { id, .. } if id.0 == wanted)),
            "{wanted} is in the Edit menu"
        );
    }
}

#[test]
#[serial]
fn a_waiting_hotkey_input_takes_the_next_press_including_escape() {
    use settings_dialog::Slot;

    let mut state = new_state();
    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    let _ = state.update(Message::Settings(settings_dialog::Message::SelectTab(
        settings_dialog::Tab::Hotkey,
    )));
    let _ = state.update(Message::Settings(settings_dialog::Message::Capture(Slot::Load)));

    let _ = state.update(press(
        keyboard::Key::Named(keyboard::key::Named::Escape),
        keyboard::Modifiers::empty(),
    ));
    let dialog = state.settings_dialog.as_ref().expect("Escape was taken as the binding");
    assert_eq!(dialog.preferences.hotkey.playback.load.as_deref(), Some("Escape"));
    assert_eq!(dialog.capturing, None);

    // Not waiting any more: Escape closes the dialog again.
    let _ = state.update(press(
        keyboard::Key::Named(keyboard::key::Named::Escape),
        keyboard::Modifiers::empty(),
    ));
    assert!(state.settings_dialog.is_none());
}

#[test]
#[serial]
fn the_template_editor_edits_the_template_not_the_selected_cue() {
    let (mut state, ids) = editing_state();
    state.ui.set_selected(ids[0], false);
    let number_before = state.model.cue_list.cues[&ids[0]].number.clone();

    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    let _ = state.update(Message::Settings(settings_dialog::Message::SelectTab(
        settings_dialog::Tab::Template,
    )));
    let _ = state.update(Message::Settings(settings_dialog::Message::SelectTemplate(
        settings_dialog::TemplateKind::Wait,
    )));
    let _ = iced_test::simulator(state.view());

    let id = field::FieldId("editor.number");
    let _ = state.update(Message::Field(field::Message::Input(id, "W9".to_owned())));
    let _ = state.update(Message::Settings(settings_dialog::Message::Done));

    assert_eq!(state.preferences.template.wait.number, "W9");
    assert_eq!(state.templates.wait.number, "W9", "new cues start from it");
    assert_eq!(state.model.cue_list.cues[&ids[0]].number, number_before);
}

#[test]
#[serial]
fn choosing_another_template_settles_the_edit_in_progress() {
    let mut state = new_state();
    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    let _ = state.update(Message::Settings(settings_dialog::Message::SelectTemplate(
        settings_dialog::TemplateKind::Audio,
    )));
    let id = field::FieldId("editor.number");
    let _ = state.update(Message::Field(field::Message::Input(id, "A1".to_owned())));
    let _ = state.update(Message::Settings(settings_dialog::Message::SelectTemplate(
        settings_dialog::TemplateKind::Fade,
    )));

    let dialog = state.settings_dialog.as_ref().unwrap();
    assert_eq!(dialog.preferences.template.audio.number, "A1");
    assert_ne!(dialog.preferences.template.fade.number, "A1");
}

#[test]
#[serial]
fn the_hardware_tab_exists_only_where_the_settings_have_audio() {
    // Tests run without the host feature: no audio hardware settings.
    let mut state = new_state();
    let _ = state.update(Message::StatusBar(status_bar::Message::OpenSettings));
    assert!(state.settings_dialog.as_ref().unwrap().audio.is_none());
    let _ = state.update(Message::Settings(settings_dialog::Message::Device(Some("x".to_owned()))));
    assert!(state.settings_dialog.as_ref().unwrap().audio.is_none(), "nothing to change");
}

#[test]
#[serial]
fn changing_the_hardware_while_audio_plays_asks_first() {
    let (mut state, _ids) = populated_state();
    // Active cues are only known from the backend's state; here only the
    // question and its answers are exercised.
    state.settings_dialog = Some(
        settings_dialog::SettingsDialog::new("Show", &state.model.settings, &state.preferences)
            .with_audio(AudioHardwareSettings::default()),
    );
    let key = |named| press(keyboard::Key::Named(named), keyboard::Modifiers::empty());

    state.audio_warning = true;
    let _ = state.update(key(keyboard::key::Named::Escape));
    assert!(!state.audio_warning);
    assert!(state.settings_dialog.is_some(), "the dialog stays open");

    state.audio_warning = true;
    let _ = state.update(Message::AudioWarning(false));
    assert!(!state.audio_warning);
    let _ = iced_test::simulator(state.view());
}

fn server_options() -> sbsp_backend::api::ApiServerOptions {
    sbsp_backend::api::ApiServerOptions {
        port: 5800,
        discoverry: None,
        auth_map: vec![sbsp_backend::api::PermissionInfo {
            password: "pw".to_owned(),
            permission: sbsp_backend::api::Permissions::READ,
        }],
    }
}

#[test]
#[serial]
fn the_server_panel_edits_its_options_and_shows_the_connect_information() {
    let mut state = new_state();
    state.server_panel = Some(server_panel::Panel::new(&server_options(), false));
    let _ = iced_test::simulator(state.view());

    let _ = state.update(Message::ServerPanel(server_panel::Message::Add));
    assert_eq!(state.server_panel.as_ref().unwrap().rows().len(), 2);

    let _ = state.update(Message::ServerPanel(server_panel::Message::ShowInfo(0)));
    assert!(state.server_panel.as_ref().unwrap().info.is_some());
    let _ = iced_test::simulator(state.view());
    let _ = state.update(Message::ServerPanel(server_panel::Message::CloseInfo));
    assert!(state.server_panel.as_ref().unwrap().info.is_none());
}

#[test]
#[serial]
fn the_server_panel_closes_on_escape_and_swallows_other_keys() {
    let mut state = new_state();
    let key = |named| press(keyboard::Key::Named(named), keyboard::Modifiers::empty());

    state.server_panel = Some(server_panel::Panel::new(&server_options(), false));
    let _ = state.update(key(keyboard::key::Named::ArrowDown));
    assert!(state.server_panel.is_some());

    // The connect information goes first, then the panel.
    let _ = state.update(Message::ServerPanel(server_panel::Message::ShowInfo(0)));
    let _ = state.update(key(keyboard::key::Named::Escape));
    assert!(state.server_panel.is_some());
    assert!(state.server_panel.as_ref().unwrap().info.is_none());
    let _ = state.update(key(keyboard::key::Named::Escape));
    assert!(state.server_panel.is_none());

    state.server_panel = Some(server_panel::Panel::new(&server_options(), false));
    let _ = state.update(Message::ServerPanel(server_panel::Message::Close));
    assert!(state.server_panel.is_none());
}

#[test]
#[serial]
fn the_credits_open_from_the_help_menu_and_close_on_escape() {
    let mut state = new_state();
    let _ = state.update(Message::Menu(crate::menu::MenuId("help.credits")));
    assert!(state.credits.is_some());
    let _ = iced_test::simulator(state.view());

    let _ = state.update(Message::Credits(credits::Message::Toggle(0)));
    assert_eq!(state.credits.as_ref().unwrap().open(), Some(0));
    let _ = iced_test::simulator(state.view());

    let _ = state.update(press(
        keyboard::Key::Named(keyboard::key::Named::ArrowDown),
        keyboard::Modifiers::empty(),
    ));
    assert!(state.credits.is_some(), "other keys do nothing");
    let _ = state.update(press(
        keyboard::Key::Named(keyboard::key::Named::Escape),
        keyboard::Modifiers::empty(),
    ));
    assert!(state.credits.is_none());

    let _ = state.update(Message::Menu(crate::menu::MenuId("help.credits")));
    let _ = state.update(Message::Credits(credits::Message::Close));
    assert!(state.credits.is_none());
}

#[test]
#[serial]
fn the_license_dialog_shows_the_edition_and_closes_in_two_steps() {
    let mut state = new_state();
    state.license_dialog = Some(license_dialog::Dialog::new(Some(license_dialog::Info {
        pro: true,
        owner: "Someone".to_owned(),
        issue_time: 1_700_000_000,
        id: "00000000-0000-0000-0000-000000000000".to_owned(),
    })));
    let _ = iced_test::simulator(state.view());

    state.license_dialog.as_mut().unwrap().activated = true;
    let _ = iced_test::simulator(state.view());

    let escape = || {
        press(
            keyboard::Key::Named(keyboard::key::Named::Escape),
            keyboard::Modifiers::empty(),
        )
    };
    let _ = state.update(escape());
    assert!(!state.license_dialog.as_ref().unwrap().activated, "the notice goes first");
    let _ = state.update(escape());
    assert!(state.license_dialog.is_none());
}

#[test]
fn the_window_and_ui_choices_are_what_the_next_run_starts_from() {
    use crate::persist::{Saved, WindowRect};

    let saved = Saved {
        window: Some(WindowRect { width: 1000.0, height: 600.0, x: Some(40.0), y: Some(30.0) }),
        mode: Some(Mode::Run),
        is_bottom_tab_open: Some(false),
        ..Saved::default()
    };
    let (mut state, _) = State::new("test", saved);
    assert_eq!(state.ui.mode, Mode::Run);
    assert!(!state.ui.is_bottom_tab_open);
    assert!(state.ui.is_right_sidebar_open);

    let _ = state.update(Message::WindowMoved(iced::Point::new(5.0, 6.0)));
    let _ = state.update(Message::WindowResized(iced::Size::new(900.0, 500.0)));
    assert_eq!(state.window_position, Some(iced::Point::new(5.0, 6.0)));
    assert_eq!(state.window_size, Some(iced::Size::new(900.0, 500.0)));
}
