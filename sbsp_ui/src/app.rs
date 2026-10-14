// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The real root `State`/`Message`/`update`/`view`/`subscription`, per the
//! migration plan's Phase 3. Replaces `prototype.rs` (Phase 0's throwaway
//! verification scaffold, now deleted -- it did its job: IME, the 2000-row
//! list, theme/language switching, the menu bar, toast/modal, and the
//! backend connection were all exercised through it and are now folded in
//! here for real).
//!
//! Scope for this patch: a working shell (menu bar, header/sidebar/footer
//! skeleton, theme, toasts) wired to the real `state::*` modules and the
//! host backend connection, with the backend's periodic position tick
//! actually driving `PlaybackState::calculate_position`. The sidebar and
//! main content are placeholders -- the cue list itself is Phase 4.

use std::time::{Duration, Instant};

use iced::widget::{column, container, row, text};
use iced::{Element, Length, Subscription, Task, event, keyboard};

#[cfg(feature = "host")]
use crate::domain::cursor::{CueEvent, CursorMove, advance};
use crate::features::{cue_list, hotkeys};
use crate::i18n::Language;
use crate::menu::{MenuNode, MenuSpec};
use crate::state::{
    assets::AssetResults,
    model::ShowModelState,
    playback::PlaybackState,
    ui::{Mode, UiState},
};
use crate::theme::ThemeMode;
use crate::widgets::{menu_bar, toast};

#[cfg(feature = "host")]
use crate::host::backend::HostPort;
use crate::port::Control;
#[cfg(feature = "host")]
use crate::port::{FullState, PortEvent};

use sbsp_frontend_settings::NameFormatSettings;

use crate::fl;

/// How often `calculate_position` re-derives interpolated positions while
/// something is active. 50ms is a placeholder, not a measured choice --
/// revisit once there's an actual level meter / progress bar reading it
/// (Phase 5) to judge smoothness against.
const POSITION_TICK: Duration = Duration::from_millis(50);

pub fn run(target: &'static str) -> iced::Result {
    iced::application(move || State::new(target), State::update, State::view)
        .title(|_: &State| fl!("app-title"))
        .theme(|state: &State| crate::theme::resolve(state.theme_mode))
        .subscription(State::subscription)
        .window_size((1100.0, 780.0))
        .run()
}

pub struct State {
    theme_mode: ThemeMode,
    lang: Language,
    // Not yet synced from the loaded settings (they live behind an async
    // RwLock inside HostPort, unreadable from the synchronous view()). The
    // defaults match what a first run loads anyway; wiring real values in
    // is a Phase 7 job (a settings dialog is the first thing that can
    // change them).
    name_format: NameFormatSettings,
    lock_cursor_to_selection: bool,
    // Same story as above: seek amount and the hotkey table come from
    // the settings defaults until Phase 7 loads real ones.
    seek_amount: f64,
    hotkeys: hotkeys::Bindings,
    modifiers: keyboard::Modifiers,
    model: ShowModelState,
    ui: UiState,
    playback: PlaybackState,
    assets: AssetResults,
    toasts: toast::Toasts,
    #[cfg(feature = "host")]
    backend: Option<HostPort>,
}

#[derive(Clone)]
pub enum Message {
    ToggleTheme,
    ToggleLanguage,
    ModifiersChanged(keyboard::Modifiers),
    KeyPressed(hotkeys::KeyPress),
    CueList(cue_list::Message),
    Menu(crate::menu::MenuId),
    Toast(toast::Message),
    Tick(Instant),
    /// Result of a fire-and-forget backend call that succeeded.
    Nothing,
    #[cfg(feature = "host")]
    BackendStarted(HostPort, FullState),
    #[cfg(feature = "host")]
    PortEvent(PortEvent),
    #[cfg(feature = "host")]
    BackendError(String),
}

/// Turns the outcome of a fire-and-forget backend call into a `Message`
/// (`Task::perform` needs one value either way).
#[cfg(feature = "host")]
fn backend_result(result: anyhow::Result<()>) -> Message {
    match result {
        Ok(()) => Message::Nothing,
        Err(e) => Message::BackendError(e.to_string()),
    }
}

impl State {
    fn new(target: &'static str) -> (Self, Task<Message>) {
        log::info!("starting sbsp_ui ({target})");

        #[cfg(feature = "host")]
        let boot_task = Task::perform(HostPort::start(), |result| match result {
            Ok((backend, state)) => Message::BackendStarted(backend, state),
            Err(e) => Message::BackendError(e.to_string()),
        });
        #[cfg(not(feature = "host"))]
        let boot_task = Task::none();

        let is_host = cfg!(feature = "host");
        let lang = Language::default();
        // Redundant with LOADER's own lazy initialization today (it loads
        // the fallback language, which is also Language::default()), but
        // explicit rather than relying on that coincidence -- if the two
        // ever diverge this still does the right thing instead of silently
        // booting in the wrong language.
        crate::i18n::select(&lang.langid());

        let state = Self {
            theme_mode: ThemeMode::default(),
            lang,
            name_format: NameFormatSettings::default(),
            lock_cursor_to_selection: sbsp_frontend_settings::GeneralSettings::default()
                .lock_cursor_to_selection,
            seek_amount: sbsp_frontend_settings::GeneralSettings::default().seek_amount,
            hotkeys: hotkeys::Bindings::from_settings(
                &sbsp_frontend_settings::hotkey::HotkeySettings::default(),
                hotkeys::is_mac(),
            ),
            modifiers: keyboard::Modifiers::default(),
            model: ShowModelState::default(),
            ui: UiState::new(is_host),
            playback: PlaybackState::new(),
            assets: AssetResults::new(),
            toasts: toast::Toasts::new(),
            #[cfg(feature = "host")]
            backend: None,
        };

        (state, boot_task)
    }

    /// Messages that need to start something asynchronous (a menu item
    /// that closes the window or talks to the backend) return their
    /// `Task` from here; everything else is a plain state change in
    /// [`State::apply`].
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Menu(id) => self.handle_menu(id),
            Message::KeyPressed(press) => self.handle_key(press),
            other => {
                self.apply(other);
                Task::none()
            }
        }
    }

    fn apply(&mut self, message: Message) {
        match message {
            Message::ToggleTheme => {
                self.theme_mode = match self.theme_mode {
                    ThemeMode::System => ThemeMode::Dark,
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::System,
                };
            }
            Message::ToggleLanguage => {
                self.lang = self.lang.toggle();
                crate::i18n::select(&self.lang.langid());
            }
            Message::ModifiersChanged(modifiers) => self.modifiers = modifiers,
            Message::CueList(message) => self.handle_cue_list(message),
            // Handled in update(): they need to return a Task.
            Message::Menu(_) | Message::KeyPressed(_) => {}
            Message::Toast(message) => self.toasts.update(message),
            Message::Nothing => {}
            Message::Tick(now) => {
                let _ = self.playback.calculate_position(true, now);
            }
            #[cfg(feature = "host")]
            Message::BackendStarted(backend, state) => {
                log::info!("backend started, {} active cue(s) at connect", state.show_state.active_cues.len());
                self.playback.update(&state.show_state, Instant::now());
                self.model = state.show_model;
                self.ui.set_playback_cursor(
                    self.model.cue_list.root_ids.first().copied(),
                    self.lock_cursor_to_selection,
                    &self.model.cue_list.cues,
                );
                self.ui.set_permission(sbsp_backend::api::Permissions::all());
                self.backend = Some(backend);
            }
            #[cfg(feature = "host")]
            Message::PortEvent(event) => self.handle_port_event(event),
            #[cfg(feature = "host")]
            Message::BackendError(message) => {
                log::error!("backend error: {message}");
                self.toasts.push(toast::Kind::Danger, message);
            }
        }
    }

    fn handle_cue_list(&mut self, message: cue_list::Message) {
        let lock = self.lock_cursor_to_selection;

        match message {
            cue_list::Message::ToggleExpand(id) => self.ui.toggle_expand(id),
            cue_list::Message::Select(id) => {
                if self.modifiers.shift() {
                    // Anchor on the current selection; with none yet,
                    // the clicked row anchors itself (a one-row range).
                    let anchor = self.ui.selected.unwrap_or(id);
                    let order = cue_list::visible_order(&self.model, &self.ui);
                    self.ui.select_range(anchor, id, &order, lock);
                } else if self.modifiers.command() {
                    if self.ui.selected_rows.contains(&id) {
                        self.ui.remove_from_selected(&[id], lock);
                    } else {
                        self.ui.add_selected(id, lock);
                    }
                } else {
                    self.ui.set_selected(id, lock);
                }
            }
        }
    }

    fn handle_key(&mut self, press: hotkeys::KeyPress) -> Task<Message> {
        match self.hotkeys.find(&press) {
            Some(action) => self.run_action(action),
            None => Task::none(),
        }
    }

    /// What each hotkey does. The guards mirror the original's per-hotkey
    /// callbacks: selection movement works in every mode, everything that
    /// plays or stops something is off in view mode.
    fn run_action(&mut self, action: hotkeys::Action) -> Task<Message> {
        use cue_list::Step;
        use hotkeys::Action as A;
        use sbsp_backend::controller::state::PlaybackStatus as S;

        let lock = self.lock_cursor_to_selection;

        match action {
            A::CursorUp | A::ExtendUp | A::CursorDown | A::ExtendDown => {
                let step = if matches!(action, A::CursorUp | A::ExtendUp) {
                    Step::Up
                } else {
                    Step::Down
                };
                let extend = matches!(action, A::ExtendUp | A::ExtendDown);
                // Scrolling the new row into view is a separate piece.
                let _ = cue_list::move_cursor(&self.model, &mut self.ui, step, extend, lock);
                return Task::none();
            }
            A::SelectAll => {
                cue_list::select_all(&self.model, &mut self.ui);
                return Task::none();
            }
            _ => {}
        }

        if self.ui.mode == Mode::View {
            return Task::none();
        }

        let selected_rows = || self.ui.selected_rows.iter().copied();
        let selected_active = self
            .ui
            .selected
            .and_then(|id| self.playback.active_cues.get(&id).map(|cue| (id, cue.status)));

        let controls: Vec<Control> = match action {
            A::Execute => self.ui.playback_cursor.map(Control::Execute).into_iter().collect(),
            A::Load => selected_rows().map(Control::Load).collect(),
            A::Stop => selected_rows().map(Control::Stop).collect(),
            A::ToggleRepeat => selected_rows().map(Control::ToggleRepeat).collect(),
            A::PauseAndResume => match selected_active {
                Some((id, S::PreWaiting | S::Playing)) => vec![Control::Pause(id)],
                Some((id, S::PreWaitPaused | S::Paused)) => vec![Control::Resume(id)],
                _ => Vec::new(),
            },
            A::PauseAll => vec![Control::PauseAll],
            A::ResumeAll => vec![Control::ResumeAll],
            A::StopAll => vec![Control::StopAll],
            A::SeekForward => selected_active
                .map(|(id, _)| Control::SeekBy(id, self.seek_amount))
                .into_iter()
                .collect(),
            A::SeekBackward => selected_active
                .map(|(id, _)| Control::SeekBy(id, -self.seek_amount))
                .into_iter()
                .collect(),
            // Handled above.
            A::CursorUp | A::ExtendUp | A::CursorDown | A::ExtendDown | A::SelectAll => {
                Vec::new()
            }
        };

        self.send(controls)
    }

    #[cfg(feature = "host")]
    fn send(&self, controls: Vec<Control>) -> Task<Message> {
        let Some(backend) = &self.backend else {
            return Task::none();
        };
        if controls.is_empty() {
            return Task::none();
        }

        Task::perform(backend.send(controls), backend_result)
    }

    /// Remote/web have no backend connection yet (Phase 8/9).
    #[cfg(not(feature = "host"))]
    fn send(&self, controls: Vec<Control>) -> Task<Message> {
        if !controls.is_empty() {
            log::warn!("no backend to send {controls:?} to");
        }
        Task::none()
    }

    fn handle_menu(&mut self, id: crate::menu::MenuId) -> Task<Message> {
        match id.0 {
            "view.toggle-theme" => self.update(Message::ToggleTheme),
            "view.toggle-language" => self.update(Message::ToggleLanguage),
            "help.about" => {
                self.toasts
                    .push(toast::Kind::Info, fl!("menu-help-about-message"));
                Task::none()
            }
            "file.quit" => iced::window::latest().and_then(iced::window::close),
            #[cfg(feature = "host")]
            "debug.add-test-cue" => self.add_test_cue(),
            #[cfg(feature = "host")]
            "debug.add-test-group" => self.add_test_group(),
            other => {
                log::warn!("unhandled menu id: {other}");
                Task::none()
            }
        }
    }

    /// TEMPORARY debug aid (removed together with `add_test_cue`): appends
    /// a playlist group holding two wait cues at the end of the root list,
    /// so expand/collapse of the cue list can be exercised by hand.
    #[cfg(feature = "host")]
    fn add_test_group(&self) -> Task<Message> {
        use sbsp_backend::manager::InsertPosition;
        use sbsp_backend::model::cue::{
            Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid,
            WaitCueParam,
            group::{GroupCueParamBase, GroupMode},
        };

        let Some(backend) = &self.backend else {
            return Task::none();
        };
        let handle = backend.handle();

        let make = |name: &str, parent_id: Option<Uuid>, params: CueParam| Cue {
            id: Uuid::new_v4(),
            number: String::new(),
            name: Some(name.to_string()),
            notes: String::new(),
            color: CueColor::default(),
            pre_wait: 0.0,
            chain: CueChain::default(),
            treat_stop_as_completed: false,
            cursor_advance_trigger_override: CueCursorAdvanceTriggerOverride::default(),
            parent_id,
            params,
        };

        let group = make(
            "Test Group",
            None,
            CueParam::Group {
                base: GroupCueParamBase {
                    mode: GroupMode::Playlist { repeat: false },
                },
                children: Vec::new(),
            },
        );
        let group_id = group.id;
        let child_a = make(
            "Child A",
            Some(group_id),
            CueParam::Wait(WaitCueParam { duration: 1.0 }),
        );
        let child_b = make(
            "Child B",
            Some(group_id),
            CueParam::Wait(WaitCueParam { duration: 2.0 }),
        );

        Task::perform(
            async move {
                let model = &handle.model_handle;
                model
                    .add_cue(
                        group,
                        InsertPosition::Inside {
                            target: None,
                            index: None,
                        },
                    )
                    .await?;
                for child in [child_a, child_b] {
                    model
                        .add_cue(
                            child,
                            InsertPosition::Inside {
                                target: Some(group_id),
                                index: None,
                            },
                        )
                        .await?;
                }
                Ok::<(), anyhow::Error>(())
            },
            backend_result,
        )
    }

    /// TEMPORARY debug aid, removed once Phase 6 has a real way to add
    /// cues: appends a wait cue ("Wait 1s", "Wait 2s", ...) at the end of
    /// the root list through the backend, so the cue list can be exercised
    /// by hand before anything else creates cues. The result comes back
    /// through `CueListUpdated`, exactly as a real add would.
    #[cfg(feature = "host")]
    fn add_test_cue(&self) -> Task<Message> {
        use sbsp_backend::manager::InsertPosition;
        use sbsp_backend::model::cue::{
            Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid,
            WaitCueParam,
        };

        let Some(backend) = &self.backend else {
            return Task::none();
        };
        let handle = backend.handle();

        let cue = Cue {
            id: Uuid::new_v4(),
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
                duration: (self.model.cue_list.cues.len() + 1) as f64,
            }),
        };

        Task::perform(
            async move {
                handle
                    .model_handle
                    .add_cue(
                        cue,
                        InsertPosition::Inside {
                            target: None,
                            index: None,
                        },
                    )
                    .await
            },
            backend_result,
        )
        .then(Task::done)
    }

    /// Replaces the whole model (a load or a reset): selection starts
    /// over, cached asset results are dropped, and the playback cursor
    /// goes to the first cue (a load) or nowhere (a reset).
    #[cfg(feature = "host")]
    fn replace_model(&mut self, model: ShowModelState, cursor_to_first: bool) {
        self.model = model;
        self.assets.clear();
        self.ui.reset_selected();
        let first = cursor_to_first
            .then(|| self.model.cue_list.root_ids.first().copied())
            .flatten();
        self.ui.set_playback_cursor(
            first,
            self.lock_cursor_to_selection,
            &self.model.cue_list.cues,
        );
    }

    /// Per-cue status events the UI itself reacts to: errors become
    /// toasts, and a trigger/completion of the cue under the playback
    /// cursor may advance the cursor, per the cue's override or else the
    /// show-wide `cursor_advance_trigger`.
    #[cfg(feature = "host")]
    fn on_cue_status(&mut self, param: &sbsp_backend::event::CueStatusEventParam) {
        use sbsp_backend::event::CueStatusEventParam as E;

        let (cue_id, event) = match param {
            E::Error { error, .. } => {
                log::error!("cue error: {error}");
                self.toasts.push(toast::Kind::Danger, error.clone());
                return;
            }
            E::Triggered { cue_id } => (*cue_id, CueEvent::Triggered),
            E::Completed { cue_id } => (*cue_id, CueEvent::Completed),
            _ => return,
        };

        if self.ui.playback_cursor != Some(cue_id) {
            return;
        }
        let Some(cue) = self.model.cue_list.cues.get(&cue_id) else {
            return;
        };

        let moved = advance(
            cue,
            &self.model.cue_list.cues,
            &self.model.cue_list.root_ids,
            self.model.settings.general.cursor_advance_trigger,
            event,
        );
        if let CursorMove::To(next) = moved {
            let lock = self.lock_cursor_to_selection;
            self.ui
                .set_playback_cursor(next, lock, &self.model.cue_list.cues);
        }
    }

    #[cfg(feature = "host")]
    fn handle_port_event(&mut self, event: PortEvent) {
        use sbsp_backend::event::BackendEvent;

        match event {
            BackendEvent::CueStatus(param) => {
                self.on_cue_status(&param);
                self.playback.handle_cue_state_event(&param, Instant::now());
            }
            BackendEvent::SyncState(data) => self.playback.handle_sync_event(&data, Instant::now()),
            BackendEvent::ShowModelLoaded { model, .. } => {
                self.replace_model(model, true);
            }
            BackendEvent::ShowModelReset { model } => {
                self.replace_model(model, false);
            }
            BackendEvent::CueRemoved { cue_ids } => {
                let lock = self.lock_cursor_to_selection;
                let ids: Vec<_> = cue_ids.iter().copied().collect();
                self.ui.remove_from_selected(&ids, lock);
                if self
                    .ui
                    .playback_cursor
                    .is_some_and(|cursor| cue_ids.contains(&cursor))
                {
                    self.ui
                        .set_playback_cursor(None, lock, &self.model.cue_list.cues);
                }
            }
            // The authoritative post-mutation list; add_cue/remove_cue/
            // move_cue and friends (Phase 4's cue list operations) all
            // produce this rather than a per-operation delta, so handling
            // just this one event keeps the model in sync with all of them.
            BackendEvent::CueListUpdated { cue_list } => self.model.cue_list = cue_list,
            BackendEvent::SettingsUpdated { new_settings } => self.model.settings = *new_settings,
            BackendEvent::ModelNameUpdated { new_name } => self.model.name = new_name,
            BackendEvent::AssetMetadata { path, data } => self.assets.add_metadata(path, data),
            BackendEvent::AssetResult { path, result } => match result {
                Ok(data) => self.assets.add(path, data),
                Err(_) => self.assets.add_error(path),
            },
            BackendEvent::OperationFailed { error } => {
                log::warn!("backend operation failed: {error:?}");
            }
            other => log::debug!("unhandled backend event: {other:?}"),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let toasts = self.toasts.subscription().map(Message::Toast);

        let tick = if self.playback.active_cues.is_empty() {
            Subscription::none()
        } else {
            iced::time::every(POSITION_TICK).map(Message::Tick)
        };

        #[cfg(feature = "host")]
        let backend_events = self
            .backend
            .as_ref()
            .map(|backend| backend.events().map(Message::PortEvent))
            .unwrap_or(Subscription::none());
        #[cfg(not(feature = "host"))]
        let backend_events = Subscription::none();

        // Modifier state (to tell a plain click from shift-/ctrl-click in
        // the cue list) and key presses (hotkeys). Events a focused widget
        // already consumed are skipped, which is also what keeps typing
        // into a text field from firing playback hotkeys.
        let modifiers = event::listen_with(|event, status, _window| {
            if status == event::Status::Captured {
                return None;
            }
            match event {
                iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                    Some(Message::ModifiersChanged(modifiers))
                }
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    modified_key,
                    modifiers,
                    repeat,
                    ..
                }) => hotkeys::KeyPress::new(&key, &modified_key, modifiers, repeat)
                    .map(Message::KeyPressed),
                _ => None,
            }
        });

        Subscription::batch([toasts, tick, backend_events, modifiers])
    }

    fn menu_spec(&self) -> MenuSpec {
        MenuSpec::new()
            .menu(
                fl!("menu-file"),
                vec![MenuNode::item("file.quit", fl!("menu-file-quit"))],
            )
            .menu(
                fl!("menu-view"),
                vec![
                    MenuNode::item("view.toggle-theme", fl!("menu-view-toggle-theme")),
                    // Shows the language it would switch *to*, in that
                    // language's own name (not fl!()'d through the
                    // currently active one): "English" should stay legible
                    // as a target even to someone who can't read whatever
                    // is presently selected, and vice versa.
                    MenuNode::item("view.toggle-language", self.lang.toggle().label()),
                ],
            )
            .menu_if(
                cfg!(feature = "host"),
                "Debug",
                vec![
                    MenuNode::item("debug.add-test-cue", "Add Test Cue"),
                    MenuNode::item("debug.add-test-group", "Add Test Group"),
                ],
            )
            .menu(
                fl!("menu-help"),
                vec![MenuNode::item("help.about", fl!("menu-help-about"))],
            )
    }

    fn view(&self) -> Element<'_, Message> {
        let header = container(menu_bar::menu_bar(&self.menu_spec(), Message::Menu))
            .width(Length::Fill)
            .padding(4);

        let sidebar = container(text(fl!("shell-sidebar-placeholder")))
            .width(240)
            .height(Length::Fill)
            .padding(12);

        let main = container(
            cue_list::view(&self.model, &self.ui, &self.playback, &self.name_format)
                .map(Message::CueList),
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12);

        let body = row![sidebar, main];

        let footer = container(text(fl!("shell-footer-ready")))
            .width(Length::Fill)
            .padding(6);

        let content: Element<'_, Message> = column![header, body, footer].into();

        self.toasts.overlay(content, Message::Toast)
    }
}

#[cfg(test)]
mod tests {
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
    fn new_state() -> State {
        State::new("host").0
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
        ui.click("日本語").expect("the language item should be clickable");

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
        ui.click("About").expect("the About item should be clickable");

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
            Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid,
            WaitCueParam,
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
        ui.click("Wait 2s").expect("the second row should be clickable");
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
        ui.click("Wait 3s").expect("the third row should be clickable");
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
    fn shell_skeleton_renders_its_placeholder_text() {
        let state = new_state();
        let mut ui = iced_test::simulator(state.view());

        assert!(
            ui.find(fl!("shell-footer-ready").as_str()).is_ok(),
            "the footer placeholder should render"
        );
        // Exact match, not substring (see the note in menu_about_shows_a_toast):
        // the widget's full text is "Sidebar (Phase 6)", so searching for
        // just "Phase 6" does not match it.
        assert!(
            ui.find(fl!("shell-sidebar-placeholder").as_str()).is_ok(),
            "the sidebar placeholder should render"
        );
    }
}
