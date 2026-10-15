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
use crate::features::{cue_list, header, hotkeys, side_bar, status_bar, viewer};
use crate::i18n::Language;
use crate::menu::{MenuNode, MenuSpec};
use crate::state::{
    assets::AssetResults,
    level_meter::LevelMeterState,
    model::ShowModelState,
    playback::PlaybackState,
    ui::{Mode, SideBarTab, UiState},
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

/// Level meter refresh (about 30fps, same as the Tauri host's poll).
const METER_TICK: Duration = Duration::from_millis(33);

/// Longest step the meter decay takes at once, so a stalled frame (window
/// dragged, machine asleep) does not drop the bars in a single jump.
const METER_MAX_DT: f32 = 0.1;

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
    cue_view: cue_list::Viewport,
    // Same story as above: seek amount and the hotkey table come from
    // the settings defaults until Phase 7 loads real ones.
    seek_amount: f64,
    hotkeys: hotkeys::Bindings,
    modifiers: keyboard::Modifiers,
    clock: header::ClockTime,
    window_focused: bool,
    hide_controls: bool,
    model: ShowModelState,
    ui: UiState,
    playback: PlaybackState,
    assets: AssetResults,
    meter: LevelMeterState,
    last_meter_tick: Option<Instant>,
    viewer: viewer::Viewer,
    /// Set when the cue list or the cache was replaced, so the next
    /// `request_assets` pass looks for audio files nobody has analyzed yet.
    assets_stale: bool,
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
    StatusBar(status_bar::Message),
    SideBar(side_bar::Message),
    Viewer(viewer::Message),
    #[cfg(feature = "host")]
    AssetRequestFailed(std::path::PathBuf, String),
    MeterTick(Instant),
    Header(header::Message),
    ClockTick,
    WindowFocused(bool),
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
            cue_view: cue_list::Viewport::default(),
            seek_amount: sbsp_frontend_settings::GeneralSettings::default().seek_amount,
            hotkeys: hotkeys::Bindings::from_settings(
                &sbsp_frontend_settings::hotkey::HotkeySettings::default(),
                hotkeys::is_mac(),
            ),
            modifiers: keyboard::Modifiers::default(),
            clock: header::local_clock(),
            window_focused: true,
            hide_controls: sbsp_frontend_settings::AppearanceSettings::default().hide_controls,
            model: ShowModelState::default(),
            ui: UiState::new(is_host),
            playback: PlaybackState::new(),
            assets: AssetResults::new(),
            meter: LevelMeterState::default(),
            last_meter_tick: None,
            viewer: viewer::Viewer::default(),
            assets_stale: false,
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
            Message::Header(message) => self.handle_header(message),
            Message::KeyPressed(press) => self.handle_key(press),
            Message::Viewer(viewer::Message::Seek(id, seconds)) => {
                // Same guard as the original: View mode does not seek.
                if self.ui.mode == Mode::View {
                    Task::none()
                } else {
                    self.send(vec![Control::SeekTo(id, seconds)])
                }
            }
            #[cfg(feature = "host")]
            message @ (Message::BackendStarted(..) | Message::PortEvent(_)) => {
                self.apply(message);
                self.request_assets()
            }
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
            Message::StatusBar(status_bar::Message::ModeSelected(mode)) => self.ui.set_mode(mode),
            Message::StatusBar(status_bar::Message::ToggleBottomTab) => {
                self.ui.toggle_bottom_tab();
            }
            Message::StatusBar(status_bar::Message::ToggleSidebar) => {
                self.ui.toggle_right_sidebar();
                self.last_meter_tick = None;
            }
            Message::SideBar(side_bar::Message::SelectTab(tab)) => {
                self.ui.side_bar_tab = tab;
                // Starting from silence when the tab comes back, rather than
                // showing whatever the bars were left at.
                self.meter.reset();
                self.last_meter_tick = None;
            }
            Message::MeterTick(now) => self.tick_meter(now),
            // Handled in update(): they need to return a Task.
            Message::Menu(_) | Message::KeyPressed(_) | Message::Header(_) | Message::Viewer(_) => {}
            Message::ClockTick => self.clock = header::local_clock(),
            Message::WindowFocused(focused) => self.window_focused = focused,
            Message::Toast(message) => self.toasts.update(message),
            Message::Nothing => {}
            Message::Tick(now) => {
                let _ = self.playback.calculate_position(true, now);
            }
            #[cfg(feature = "host")]
            Message::BackendStarted(backend, state) => {
                log::info!(
                    "backend started, {} active cue(s) at connect",
                    state.show_state.active_cues.len()
                );
                self.playback.update(&state.show_state, Instant::now());
                self.model = state.show_model;
                self.ui.set_playback_cursor(
                    self.model.cue_list.root_ids.first().copied(),
                    self.lock_cursor_to_selection,
                    &self.model.cue_list.cues,
                );
                self.ui
                    .set_permission(sbsp_backend::api::Permissions::all());
                self.backend = Some(backend);
                self.assets_stale = true;
            }
            #[cfg(feature = "host")]
            Message::AssetRequestFailed(path, message) => {
                log::warn!("asset request for {} failed: {message}", path.display());
                self.assets.add_error(path);
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

    /// One meter frame: reads the engine's level (host only; a remote has
    /// no source for it until the protocol carries one), then decays.
    fn tick_meter(&mut self, now: Instant) {
        let dt = self
            .last_meter_tick
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(0.0)
            .min(METER_MAX_DT);
        self.last_meter_tick = Some(now);

        #[cfg(feature = "host")]
        if let Some((left, right)) = self.backend.as_ref().and_then(HostPort::level) {
            // The Tauri host only forwards non-silent frames; same here.
            if left > 0.001 || right > 0.001 {
                self.meter.push(left, right);
            }
        }

        self.meter.step(dt);
    }

    fn handle_cue_list(&mut self, message: cue_list::Message) {
        let lock = self.lock_cursor_to_selection;

        match message {
            cue_list::Message::ToggleExpand(id) => self.ui.toggle_expand(id),
            cue_list::Message::Scrolled(viewport) => self.cue_view = viewport,
            cue_list::Message::SetCursor(id) => {
                self.ui
                    .set_playback_cursor(Some(id), lock, &self.model.cue_list.cues)
            }
            cue_list::Message::TogglePreWaitDisplay => self.ui.toggle_pre_wait_display_mode(),
            cue_list::Message::ToggleDurationDisplay => self.ui.toggle_duration_display_mode(),
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

    /// Whether the header's transport buttons are shown.
    fn show_controls(&self) -> bool {
        !self.hide_controls && self.ui.mode != Mode::View
    }

    fn handle_header(&mut self, message: header::Message) -> Task<Message> {
        if self.ui.mode == Mode::View {
            return Task::none();
        }
        let cursor = self.ui.playback_cursor;
        let status = cursor
            .and_then(|id| self.playback.active_cues.get(&id))
            .map(|active| active.status);

        self.send(header::controls_for(message, cursor, status))
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
                let Some(id) = cue_list::move_cursor(&self.model, &mut self.ui, step, extend, lock)
                else {
                    return Task::none();
                };
                let Some(y) = cue_list::scroll_offset_for(&self.model, &self.ui, self.cue_view, id)
                else {
                    return Task::none();
                };
                // Remembered right away: a held arrow key can outrun the
                // scrollable's own report of where it ended up.
                self.cue_view.offset_y = y;
                return iced::widget::operation::scroll_to(
                    cue_list::LIST_ID,
                    iced::widget::operation::AbsoluteOffset { x: 0.0, y },
                );
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
        let selected_active = self.ui.selected.and_then(|id| {
            self.playback
                .active_cues
                .get(&id)
                .map(|cue| (id, cue.status))
        });

        let controls: Vec<Control> = match action {
            A::Execute => self
                .ui
                .playback_cursor
                .map(Control::Execute)
                .into_iter()
                .collect(),
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
            A::CursorUp | A::ExtendUp | A::CursorDown | A::ExtendDown | A::SelectAll => Vec::new(),
        };

        self.send(controls)
    }

    /// Asks the backend to analyze every audio file in the cue list that
    /// has no result yet (the viewer needs the waveform, the list needs
    /// the duration). The original did this lazily as rows rendered; the
    /// processor already bounds its own concurrency, so asking up front is
    /// fine and keeps `view()` free of side effects.
    #[cfg(feature = "host")]
    fn request_assets(&mut self) -> Task<Message> {
        use sbsp_backend::model::cue::CueParam;

        if !std::mem::take(&mut self.assets_stale) {
            return Task::none();
        }
        let Some(backend) = &self.backend else {
            return Task::none();
        };

        let mut paths: Vec<std::path::PathBuf> = self
            .model
            .cue_list
            .cues
            .values()
            .filter_map(|cue| match &cue.params {
                CueParam::Audio(audio) => Some(audio.target.clone()),
                _ => None,
            })
            .filter(|path| self.assets.get(path).is_none() && self.assets.should_request(path))
            .collect();
        paths.sort();
        paths.dedup();

        Task::batch(paths.into_iter().map(|path| {
            self.assets.mark_processing(path.clone());
            let failed_path = path.clone();
            Task::perform(backend.request_asset(path), move |result| match result {
                Ok(()) => Message::Nothing,
                Err(e) => Message::AssetRequestFailed(failed_path.clone(), e.to_string()),
            })
        }))
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
            #[cfg(feature = "host")]
            "debug.add-test-audio-cue" => self.add_test_audio_cue(),
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
            Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid, WaitCueParam,
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
            Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid, WaitCueParam,
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

    /// Debug aid: asks for an audio file and adds a cue that plays it, so
    /// the waveform viewer, durations and the sidebar can be exercised
    /// before the real editor and file operations exist (Phase 6/7).
    /// Removed together with the other Debug menu items.
    #[cfg(feature = "host")]
    fn add_test_audio_cue(&self) -> Task<Message> {
        use sbsp_backend::manager::InsertPosition;
        use sbsp_backend::model::cue::{
            Cue, CueChain, CueColor, CueCursorAdvanceTriggerOverride, CueParam, Uuid,
            audio::{AudioCueParam, Decibels, SoundType},
        };

        let Some(backend) = &self.backend else {
            return Task::none();
        };
        let handle = backend.handle();

        Task::perform(
            async move {
                let Some(file) = rfd::AsyncFileDialog::new()
                    .set_title("Add Test Audio Cue")
                    .add_filter("Audio", &["wav", "mp3", "flac", "ogg", "m4a", "aac"])
                    .pick_file()
                    .await
                else {
                    // Cancelled.
                    return Ok(());
                };

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
                    params: CueParam::Audio(AudioCueParam {
                        target: file.path().to_path_buf(),
                        start_time: None,
                        fade_in_param: None,
                        end_time: None,
                        fade_out_param: None,
                        volume: Decibels::default(),
                        pan: 0.0,
                        repeat: false,
                        sound_type: SoundType::default(),
                        envelope: Vec::new(),
                    }),
                };

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
    }

    /// Replaces the whole model (a load or a reset): selection starts
    /// over, cached asset results are dropped, and the playback cursor
    /// goes to the first cue (a load) or nowhere (a reset).
    #[cfg(feature = "host")]
    fn replace_model(&mut self, model: ShowModelState, cursor_to_first: bool) {
        self.model = model;
        self.assets.clear();
        self.assets_stale = true;
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
            BackendEvent::CueListUpdated { cue_list } => {
                self.model.cue_list = cue_list;
                self.assets_stale = true;
            }
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

        // Once a second is enough for the header clock to never show a
        // stale second for long; the half-second period keeps it from
        // beating against the second boundary.
        let clock = iced::time::every(Duration::from_millis(500)).map(|_| Message::ClockTick);

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
                iced::Event::Window(iced::window::Event::Focused) => {
                    Some(Message::WindowFocused(true))
                }
                iced::Event::Window(iced::window::Event::Unfocused) => {
                    Some(Message::WindowFocused(false))
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

        let meter = if self.ui.is_right_sidebar_open && self.ui.side_bar_tab == SideBarTab::Meter {
            iced::time::every(METER_TICK).map(Message::MeterTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([toasts, tick, clock, meter, backend_events, modifiers])
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
                    MenuNode::item("debug.add-test-audio-cue", "Add Test Audio Cue..."),
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

        let cursor_cue = self
            .ui
            .playback_cursor
            .and_then(|id| self.model.cue_list.cues.get(&id));
        let transport = header::view(header::View {
            clock: self.clock,
            window_focused: self.window_focused,
            show_controls: self.show_controls(),
            show_add_cue: self.ui.mode == Mode::Edit,
            cursor_cue,
            cursor_cue_name: cursor_cue
                .map(|cue| {
                    crate::domain::cue::build_cue_name(
                        Some(cue),
                        &self.model.cue_list.cues,
                        &self.name_format,
                    )
                })
                .unwrap_or_default(),
            cursor_status: self
                .ui
                .playback_cursor
                .and_then(|id| self.playback.active_cues.get(&id))
                .map(|active| active.status),
        })
        .map(Message::Header);

        let main = container(
            cue_list::view(
                &self.model,
                &self.ui,
                &self.playback,
                &self.assets,
                &self.name_format,
            )
                .map(Message::CueList),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12);

        let main_column: Element<'_, Message> = if self.ui.is_bottom_tab_open {
            let viewer = container(
                viewer::view(
                    &self.viewer,
                    &self.model,
                    &self.ui,
                    &self.playback,
                    &self.assets,
                )
                .map(Message::Viewer),
            )
            .width(Length::Fill)
            .height(viewer::HEIGHT + 2.0)
            .padding(1)
            .style(container::bordered_box);
            column![main, viewer].into()
        } else {
            main.into()
        };

        let body: Element<'_, Message> = if self.ui.is_right_sidebar_open {
            let sidebar = container(
                side_bar::view(
                    &self.model,
                    &self.ui,
                    &self.playback,
                    &self.meter,
                    &self.name_format,
                )
                .map(Message::SideBar),
            )
            .width(side_bar::WIDTH)
            .height(Length::Fill);
            row![main_column, sidebar].into()
        } else {
            main_column
        };

        let footer = container(
            status_bar::view(&self.ui, self.model.cue_list.cues.len()).map(Message::StatusBar),
        )
        .width(Length::Fill);

        let content: Element<'_, Message> = column![header, transport, body, footer].into();

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
}
