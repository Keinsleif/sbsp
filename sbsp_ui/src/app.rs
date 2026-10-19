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

use iced::widget::{column, container, row};
use iced::{Element, Length, Subscription, Task, event, keyboard, mouse};

use crate::domain::{clipboard, cue_edit};
use crate::domain::new_cue::{self, CueKind};
#[cfg(feature = "host")]
use crate::domain::cursor::{CueEvent, CursorMove, advance};
use crate::features::{
    cue_list, debug_gallery, editor, header, hotkeys, renumber, side_bar, status_bar, viewer,
};
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
use crate::widgets::{divider, field, menu_bar, modal, toast};

#[cfg(feature = "host")]
use crate::host::backend::HostPort;
use crate::port::{Control, ModelEdit};
#[cfg(feature = "host")]
use crate::port::{FullState, PortEvent};

use sbsp_backend::model::cue::{Cue, CueParam, Uuid};
use sbsp_frontend_settings::{NameFormatSettings, TemplateSettings};

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
        .window_size((1280.0, 720.0))
        // `settings` replaces the whole `Settings`, so it comes before the
        // builder calls that fill in its fonts.
        .settings(iced::Settings {
            default_text_size: iced::Pixels(crate::theme::fonts::TEXT_SIZE),
            ..Default::default()
        })
        .font(crate::theme::fonts::UI_REGULAR)
        .font(crate::theme::fonts::MONO_REGULAR)
        .default_font(crate::theme::fonts::UI)
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
    /// Templates for new cues; the defaults until Phase 7 loads the
    /// settings.
    templates: TemplateSettings,
    /// Cues waiting for the user to confirm their removal.
    pending_remove: Option<Vec<Uuid>>,
    /// The renumber dialog's values and whether it is open.
    renumber: renumber::Dialog,
    renumber_open: bool,
    /// Files dropped on the window, waiting for the rest of the batch.
    dropped_files: Vec<std::path::PathBuf>,
    /// What copy and cut took; paste adds copies of these.
    clipboard: Vec<Cue>,
    /// The cue list row under the pointer (edit mode), which a right
    /// click acts on.
    hovered_row: Option<Uuid>,
    /// Cues being dragged to a new place in the list.
    cue_drag: Option<cue_list::Drag>,
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
    /// The one text field being edited, if any (shared by every field).
    fields: field::Drafts,
    editor: editor::Editor,
    gallery: debug_gallery::Gallery,
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
    Field(field::Message),
    Editor(editor::Message),
    Gallery(debug_gallery::Message),
    Renumber(renumber::Message),
    /// The files chosen in the add-audio-cue picker (empty: cancelled).
    AudioFilesPicked(Vec<std::path::PathBuf>),
    /// The file chosen for an audio cue's target (`None`: cancelled).
    AudioTargetPicked(Uuid, Option<std::path::PathBuf>),
    RemoveConfirmed,
    RemoveCancelled,
    #[cfg(feature = "host")]
    AssetRequestFailed(std::path::PathBuf, String),
    MeterTick(Instant),
    Header(header::Message),
    ClockTick,
    WindowFocused(bool),
    /// A file was dropped on the window (one event per file).
    FileDropped(std::path::PathBuf),
    /// No more drops arrived for a moment: handle the batch.
    DropsSettled,
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
            templates: TemplateSettings::default(),
            pending_remove: None,
            renumber: renumber::Dialog::default(),
            renumber_open: false,
            dropped_files: Vec::new(),
            clipboard: Vec::new(),
            hovered_row: None,
            cue_drag: None,
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
            fields: field::Drafts::default(),
            editor: editor::Editor::default(),
            gallery: debug_gallery::Gallery::default(),
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
        // The notes editor keeps its own copy of the selected cue's notes;
        // everything that can change either is followed by a check.
        let sync = !matches!(
            message,
            Message::Tick(_) | Message::MeterTick(_) | Message::ClockTick
        );
        let task = self.update_inner(message);
        if sync {
            self.sync_editor();
        }
        task
    }

    fn sync_editor(&mut self) {
        let cue = self
            .ui
            .selected
            .and_then(|id| self.model.cue_list.cues.get(&id));
        self.editor.file = self.file_info(cue);
        self.editor.sync(cue, &self.fields);
    }

    /// What the analysis of the audio cue's file says.
    fn file_info(&self, cue: Option<&Cue>) -> editor::FileInfo {
        let Some(cue) = cue else {
            return editor::FileInfo::default();
        };
        let data = match &cue.params {
            CueParam::Audio(audio) => self.assets.get(&audio.target),
            _ => None,
        };
        editor::FileInfo {
            duration: cue_list::asset_duration(cue, &self.assets),
            integrated_lufs: data.and_then(|data| data.integrated_lufs),
            peak: data.map(|data| f64::from(data.peak)),
            sound_start: data.and_then(|data| data.start_time),
            sound_end: data.and_then(|data| data.end_time),
        }
    }

    /// Sends an edited cue. A volume change of a cue that is playing goes
    /// to the playing cue as well, as the original's editor does.
    fn send_cue_edit(&self, before: &Cue, edited: Cue) -> Task<Message> {
        let volume = match (&before.params, &edited.params) {
            (CueParam::Audio(old), CueParam::Audio(new))
                if old.volume != new.volume && self.playback.active_cues.contains_key(&edited.id) =>
            {
                Some(Control::SetVolume(edited.id, f64::from(f32::from(new.volume))))
            }
            _ => None,
        };
        let live = self.send(volume.into_iter().collect());
        Task::batch([self.send_edit(ModelEdit::UpdateCue(edited)), live])
    }

    fn update_inner(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Menu(id) => self.handle_menu(id),
            Message::Header(message) => self.handle_header(message),
            Message::KeyPressed(press) => self.handle_key(press),
            Message::Field(message) => self.handle_field(message),
            Message::Editor(message) => self.handle_editor(message),
            Message::Gallery(message) => self.handle_gallery(message),
            Message::Renumber(message) => self.handle_renumber(message),
            Message::CueList(cue_list::Message::Context(action)) => {
                self.run_context_action(action)
            }
            Message::CueList(cue_list::Message::DragDropped) => self.finish_drag(),
            Message::CueList(cue_list::Message::DragTick) => self.scroll_while_dragging(),
            Message::AudioFilesPicked(files) => self.add_audio_cues(files),
            Message::AudioTargetPicked(id, file) => self.set_audio_target(id, file),
            Message::RemoveConfirmed => self.confirm_remove(),
            Message::FileDropped(path) => {
                self.dropped_files.push(path);
                Task::none()
            }
            Message::DropsSettled => self.add_dropped_files(),
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
            Message::StatusBar(status_bar::Message::ModeSelected(mode)) => {
                self.cue_drag = None;
                self.ui.set_mode(mode);
            }
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
            Message::Menu(_) | Message::KeyPressed(_) | Message::Header(_)
            | Message::Viewer(_)
            | Message::Field(_)
            | Message::Editor(_)
            | Message::Gallery(_)
            | Message::Renumber(_)
            | Message::FileDropped(_)
            | Message::DropsSettled
            | Message::AudioFilesPicked(_)
            | Message::AudioTargetPicked(..)
            | Message::RemoveConfirmed => {}
            Message::RemoveCancelled => self.pending_remove = None,
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
            cue_list::Message::Hover(id) => self.hovered_row = Some(id),
            cue_list::Message::HoverEnd(id) => {
                // Rows swap in either order as the pointer crosses them.
                if self.hovered_row == Some(id) {
                    self.hovered_row = None;
                }
            }
            cue_list::Message::RightClicked => {
                // The menu opens by itself; what it acts on is the row
                // under the pointer, unless that is in the selection
                // already (the original did the same).
                if let Some(id) = self.hovered_row
                    && self.ui.mode == Mode::Edit
                    && !self.ui.selected_rows.contains(&id)
                {
                    self.ui.set_selected(id, lock);
                }
            }
            cue_list::Message::DragStart(id) => {
                if self.ui.mode == Mode::Edit {
                    // Dragging a row outside the selection takes that row
                    // alone; dragging one of them takes them all.
                    if !self.ui.selected_rows.contains(&id) {
                        self.ui.set_selected(id, lock);
                    }
                    self.cue_drag = Some(cue_list::Drag {
                        ids: self.ui.selected_rows.clone(),
                        pointer: None,
                    });
                }
            }
            cue_list::Message::DragMoved(point) => {
                if let Some(drag) = &mut self.cue_drag {
                    drag.pointer = Some(point);
                }
            }
            cue_list::Message::DragLeft => {
                if let Some(drag) = &mut self.cue_drag {
                    drag.pointer = None;
                }
            }
            // These need to return a Task: see `update_inner`.
            cue_list::Message::Context(_)
            | cue_list::Message::DragDropped
            | cue_list::Message::DragTick => {}
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
        if let header::Message::AddCue(kind) = message {
            return self.add_cue(kind);
        }
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
        // Escape drops a drag on the floor (and is not Stop All, which it
        // is bound to).
        if self.cue_drag.is_some() && press.names.iter().any(|name| name == "Escape") {
            self.cue_drag = None;
            return Task::none();
        }
        // Behind the removal confirmation only its own keys work: GO on
        // Enter must not fire while the question is open.
        if self.pending_remove.is_some() {
            let is = |name: &str| press.names.iter().any(|n| n == name) && !press.repeat;
            return if is("Enter") {
                self.confirm_remove()
            } else if is("Escape") {
                self.pending_remove = None;
                Task::none()
            } else {
                Task::none()
            };
        }
        // Likewise the renumber dialog: Enter runs it, Escape closes it.
        if self.renumber_open {
            let is = |name: &str| press.names.iter().any(|n| n == name) && !press.repeat;
            return if is("Enter") {
                self.run_renumber()
            } else if is("Escape") {
                self.renumber_open = false;
                Task::none()
            } else {
                Task::none()
            };
        }
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
            A::Delete => {
                self.request_remove();
                return Task::none();
            }
            A::Copy => {
                self.copy_selection();
                return Task::none();
            }
            A::Cut => return self.cut_selection(),
            A::Paste => return self.paste_clipboard(),
            A::Duplicate => return self.duplicate_selection(),
            A::Renumber => {
                self.open_renumber();
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
            A::CursorUp
            | A::ExtendUp
            | A::CursorDown
            | A::ExtendDown
            | A::SelectAll
            | A::Delete
            | A::Copy
            | A::Cut
            | A::Paste
            | A::Duplicate
            | A::Renumber => Vec::new(),
        };

        self.send(controls)
    }

    /// A text field's typing, Enter, focus loss or step button. A finished
    /// edit goes to whichever part owns that field's value.
    fn handle_field(&mut self, message: field::Message) -> Task<Message> {
        // An editor field's edit belongs to the cue selected when it began.
        let begins_editor_edit = matches!(
            &message,
            field::Message::Input(id, _) if editor::owns(*id) && !self.fields.is_editing(*id)
        );

        // Enter in one of the renumber dialog's fields runs it, as in the
        // original; Escape in one closes it.
        let submits_renumber = self.renumber_open
            && matches!(&message, field::Message::Submit(id) if renumber::owns(*id));
        if self.renumber_open && message == field::Message::Escape {
            self.renumber_open = false;
        }

        let (outcome, task) = self.fields.update(message);
        let mut edit = Task::none();
        if let Some(outcome) = outcome {
            self.gallery.apply(&outcome);
            self.renumber.apply(&outcome);
            // A step button press has no draft behind it: it applies to
            // the cue on screen.
            let cue_id = match outcome {
                field::Outcome::Step(..) => self.ui.selected,
                field::Outcome::Commit(_) => self.editor.draft_cue,
            };
            let cue = cue_id.and_then(|id| self.model.cue_list.cues.get(&id));
            if let Some((before, edited)) = cue
                .and_then(|cue| self.editor.apply(&outcome, cue).map(|edited| (cue, edited)))
            {
                edit = self.send_cue_edit(before, edited);
            }
        }
        if begins_editor_edit {
            self.editor.draft_cue = self.ui.selected;
        }

        let run = if submits_renumber {
            self.run_renumber()
        } else {
            Task::none()
        };

        Task::batch([task.map(Message::Field), edit, run])
    }

    fn handle_renumber(&mut self, message: renumber::Message) -> Task<Message> {
        match message {
            renumber::Message::Field(message) => self.handle_field(message),
            renumber::Message::Run => self.run_renumber(),
            renumber::Message::Cancel => {
                self.renumber_open = false;
                Task::none()
            }
        }
    }

    /// Opens the renumber dialog for the selected cues. Edit mode only.
    fn open_renumber(&mut self) {
        if self.ui.mode == Mode::Edit && !self.ui.selected_rows.is_empty() {
            self.renumber = renumber::Dialog::default();
            self.renumber_open = true;
        }
    }

    /// The dialog's Run: renumbers the selected cues in list order and
    /// closes. Values that cannot be used leave the dialog open.
    fn run_renumber(&mut self) -> Task<Message> {
        // The field being edited counts: the button's press may arrive
        // before the field notices it lost the focus.
        if let Some(outcome) = self.fields.flush() {
            self.renumber.apply(&outcome);
        }
        if !self.renumber_open {
            return Task::none();
        }
        let Some(request) = self.renumber.request() else {
            return Task::none();
        };
        self.renumber_open = false;
        if self.ui.mode != Mode::Edit {
            return Task::none();
        }

        let ids: Vec<Uuid> = crate::domain::clipboard::selected_cues(
            &self.flat_cues(),
            &self.model.cue_list.cues,
            &self.ui.selected_rows,
        )
        .into_iter()
        .map(|cue| cue.id)
        .collect();
        if ids.is_empty() {
            return Task::none();
        }
        self.send_edit(ModelEdit::RenumberCues {
            ids,
            start_from: request.start,
            increment: request.increment,
            prefix: request.prefix,
            suffix: request.suffix,
        })
    }

    /// The bottom editor (edit mode only).
    fn handle_editor(&mut self, message: editor::Message) -> Task<Message> {
        if self.ui.mode != Mode::Edit {
            return Task::none();
        }
        let cue = self
            .ui
            .selected
            .and_then(|id| self.model.cue_list.cues.get(&id))
            .cloned();
        let now = Instant::now();
        let position = cue
            .as_ref()
            .and_then(|cue| self.playback.get_position(cue.id, now));

        match self.editor.update(message, cue.as_ref(), position, now) {
            editor::Update::None => Task::none(),
            editor::Update::Field(message) => self.handle_field(message),
            editor::Update::Edit(edited) => match cue {
                Some(before) => self.send_cue_edit(&before, *edited),
                None => Task::none(),
            },
            editor::Update::PickAudio(id) => self.pick_audio_target(id),
            editor::Update::Seek(id, seconds) => self.send(vec![Control::SeekTo(id, seconds)]),
            editor::Update::MatchVolume(kind) => {
                let Some(before) = cue else {
                    return Task::none();
                };
                let info = self.file_info(Some(&before));
                let volume = match kind {
                    editor::Match::Lufs => info.integrated_lufs.map(|lufs| {
                        cue_edit::volume_for_lufs(self.model.settings.audio.lufs_target, lufs)
                    }),
                    editor::Match::Peak => info.peak.map(cue_edit::volume_for_peak),
                };
                match volume.and_then(|volume| cue_edit::set_level(&before, volume)) {
                    Some(edited) => self.send_cue_edit(&before, edited),
                    None => Task::none(),
                }
            }
        }
    }

    #[cfg(feature = "host")]
    fn pick_audio_target(&self, id: Uuid) -> Task<Message> {
        let title = fl!("main-bottom-editor-audio-target-file");
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .add_filter("Audio", new_cue::AUDIO_EXTENSIONS)
                    .pick_file()
                    .await
                    .map(|file| file.path().to_path_buf())
            },
            move |file| Message::AudioTargetPicked(id, file),
        )
    }

    #[cfg(not(feature = "host"))]
    fn pick_audio_target(&self, _id: Uuid) -> Task<Message> {
        log::warn!("no file picker in this build");
        Task::none()
    }

    /// The picker answered: the cue as it is *now* gets the new target.
    fn set_audio_target(&mut self, id: Uuid, file: Option<std::path::PathBuf>) -> Task<Message> {
        let Some(file) = file else {
            return Task::none();
        };
        let Some(cue) = self.model.cue_list.cues.get(&id) else {
            return Task::none();
        };
        match crate::domain::cue_edit::set_audio_target(cue, file) {
            Some(edited) => self.send_edit(ModelEdit::UpdateCue(edited)),
            None => Task::none(),
        }
    }

    fn handle_gallery(&mut self, message: debug_gallery::Message) -> Task<Message> {
        use debug_gallery::Message as G;

        let field = match message {
            G::Field(field) => Some(field),
            G::Volume(message) => self.gallery.volume_message(message, Instant::now()),
            G::Panning(message) => self.gallery.panning_message(message, Instant::now()),
            G::Close => {
                self.gallery.open = false;
                None
            }
        };
        match field {
            Some(field) => self.handle_field(field),
            None => Task::none(),
        }
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

    #[cfg(feature = "host")]
    fn send_edit(&self, edit: ModelEdit) -> Task<Message> {
        let Some(backend) = &self.backend else {
            return Task::none();
        };
        Task::perform(backend.edit(edit), backend_result)
    }

    /// Remote/web have no backend connection yet (Phase 8/9).
    #[cfg(not(feature = "host"))]
    fn send_edit(&self, edit: ModelEdit) -> Task<Message> {
        log::warn!("no backend to send {edit:?} to");
        Task::none()
    }

    /// An add-cue button or menu item. Edit mode only. A new cue goes
    /// after the selection (see `domain::new_cue` for the exceptions); a
    /// group cue then takes the selected rows into itself and opens.
    fn add_cue(&mut self, kind: CueKind) -> Task<Message> {
        if self.ui.mode != Mode::Edit {
            return Task::none();
        }
        if kind == CueKind::Audio {
            return self.pick_audio_files();
        }

        let id = Uuid::new_v4();
        let Some(new) = new_cue::build(
            kind,
            id,
            &self.templates,
            self.ui.selected,
            &self.model.cue_list.cues,
        ) else {
            return Task::none();
        };

        if kind == CueKind::Group {
            let members = self.ui.selected_rows.clone();
            if !members.is_empty() {
                self.ui.expanded_rows.insert(id);
            }
            return self.send_edit(ModelEdit::AddGroup {
                group: new.cue,
                position: new.position,
                members,
            });
        }
        self.send_edit(ModelEdit::AddCues {
            cues: vec![new.cue],
            position: new.position,
        })
    }

    #[cfg(feature = "host")]
    fn pick_audio_files(&self) -> Task<Message> {
        let title = fl!("menu-cue-audio");
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .add_filter("Audio", new_cue::AUDIO_EXTENSIONS)
                    .pick_files()
                    .await
                    .map(|files| {
                        files
                            .into_iter()
                            .map(|f| f.path().to_path_buf())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            },
            Message::AudioFilesPicked,
        )
    }

    #[cfg(not(feature = "host"))]
    fn pick_audio_files(&self) -> Task<Message> {
        log::warn!("no file picker in this build");
        Task::none()
    }

    /// The picker answered: one audio cue per file, after the selection as
    /// it is *now* (the user may have moved it while the dialog was open).
    fn add_audio_cues(&mut self, files: Vec<std::path::PathBuf>) -> Task<Message> {
        if self.ui.mode != Mode::Edit {
            return Task::none();
        }
        let Some((cues, position)) = new_cue::audio_cues(
            files,
            std::iter::repeat_with(Uuid::new_v4),
            &self.templates,
            self.ui.selected,
        ) else {
            return Task::none();
        };
        self.send_edit(ModelEdit::AddCues { cues, position })
    }

    /// The batch of dropped files: audio files become cues after the
    /// selection (the drop position is not known, so not at the row it
    /// landed on); anything else is reported once. Edit mode only.
    fn add_dropped_files(&mut self) -> Task<Message> {
        let files = std::mem::take(&mut self.dropped_files);
        if self.ui.mode != Mode::Edit || files.is_empty() {
            return Task::none();
        }
        let (audio, other): (Vec<_>, Vec<_>) =
            files.into_iter().partition(|file| new_cue::is_audio_file(file));
        if !other.is_empty() {
            self.toasts
                .push(toast::Kind::Warning, fl!("notification-invalid-file-type"));
        }
        self.add_audio_cues(audio)
    }

    /// Delete: asks before removing the selected cues. Edit mode only.
    fn request_remove(&mut self) {
        if self.ui.mode == Mode::Edit && !self.ui.selected_rows.is_empty() {
            self.pending_remove = Some(self.ui.selected_rows.clone());
        }
    }

    /// The user said yes: removes the cues that are still there.
    fn confirm_remove(&mut self) -> Task<Message> {
        let Some(ids) = self.pending_remove.take() else {
            return Task::none();
        };
        if self.ui.mode != Mode::Edit {
            return Task::none();
        }
        let ids: Vec<Uuid> = ids
            .into_iter()
            .filter(|id| self.model.cue_list.cues.contains_key(id))
            .collect();
        if ids.is_empty() {
            return Task::none();
        }
        self.send_edit(ModelEdit::RemoveCues(ids))
    }

    /// The cue list's rows in display order, for the clipboard.
    fn flat_cues(&self) -> Vec<crate::domain::flat_list::FlatCueEntry> {
        crate::domain::flat_list::flatten(
            &self.model.cue_list.root_ids,
            &self.model.cue_list.cues,
            &self.ui.expanded_rows,
        )
    }

    /// Copy: the selected cues, top first, replace the clipboard's. Works
    /// in every mode; nothing selected leaves the clipboard alone.
    fn copy_selection(&mut self) {
        let cues = clipboard::selected_cues(
            &self.flat_cues(),
            &self.model.cue_list.cues,
            &self.ui.selected_rows,
        );
        if !cues.is_empty() {
            self.clipboard = cues;
        }
    }

    /// Cut: copy, then remove the originals without asking (edit mode;
    /// elsewhere it only copies, as in the original).
    fn cut_selection(&mut self) -> Task<Message> {
        self.copy_selection();
        if self.ui.mode != Mode::Edit {
            return Task::none();
        }
        let ids: Vec<Uuid> = self
            .ui
            .selected_rows
            .iter()
            .copied()
            .filter(|id| self.model.cue_list.cues.contains_key(id))
            .collect();
        if ids.is_empty() {
            return Task::none();
        }
        self.send_edit(ModelEdit::RemoveCues(ids))
    }

    /// Paste: copies of the clipboard's cues after the selected one (at the
    /// end of the list with none). Edit mode only.
    fn paste_clipboard(&mut self) -> Task<Message> {
        if self.ui.mode != Mode::Edit || self.clipboard.is_empty() {
            return Task::none();
        }
        let target = self
            .ui
            .selected
            .filter(|id| self.model.cue_list.cues.contains_key(id));
        self.send_edit(ModelEdit::AddCues {
            cues: clipboard::for_paste(&self.clipboard, Uuid::new_v4),
            position: new_cue::position_after(target),
        })
    }

    /// Duplicate: copies of the selected cues right after them, without
    /// touching the clipboard. Edit mode only.
    fn duplicate_selection(&mut self) -> Task<Message> {
        if self.ui.mode != Mode::Edit {
            return Task::none();
        }
        let flat = self.flat_cues();
        let cues = clipboard::selected_cues(
            &flat,
            &self.model.cue_list.cues,
            &self.ui.selected_rows,
        );
        let Some(position) = clipboard::duplicate_position(&flat, &self.ui.selected_rows) else {
            return Task::none();
        };
        if cues.is_empty() {
            return Task::none();
        }
        self.send_edit(ModelEdit::AddCues {
            cues: clipboard::for_paste(&cues, Uuid::new_v4),
            position,
        })
    }

    /// The mouse button went up: moves the dragged cues to where the
    /// indicator is, if it is anywhere.
    fn finish_drag(&mut self) -> Task<Message> {
        let Some(drag) = self.cue_drag.take() else {
            return Task::none();
        };
        if self.ui.mode != Mode::Edit {
            return Task::none();
        }
        let Some(position) =
            cue_list::drop_position(&self.model, &self.ui, self.cue_view, &drag)
        else {
            return Task::none();
        };
        let ids: Vec<Uuid> = drag
            .ids
            .into_iter()
            .filter(|id| self.model.cue_list.cues.contains_key(id))
            .collect();
        if ids.is_empty() {
            return Task::none();
        }
        self.send_edit(ModelEdit::MoveCues { ids, position })
    }

    /// A tick of a drag: scrolls the list if the pointer is at its edge.
    fn scroll_while_dragging(&mut self) -> Task<Message> {
        let Some(drag) = &self.cue_drag else {
            return Task::none();
        };
        let Some(y) = cue_list::drag_scroll_offset(&self.model, &self.ui, self.cue_view, drag)
        else {
            return Task::none();
        };
        // Remembered right away, as for the arrow keys.
        self.cue_view.offset_y = y;
        iced::widget::operation::scroll_to(
            cue_list::LIST_ID,
            iced::widget::operation::AbsoluteOffset { x: 0.0, y },
        )
    }

    fn run_context_action(&mut self, action: cue_list::ContextAction) -> Task<Message> {
        use cue_list::ContextAction as C;
        match action {
            C::Copy => self.run_action(hotkeys::Action::Copy),
            C::Cut => self.run_action(hotkeys::Action::Cut),
            C::Paste => self.run_action(hotkeys::Action::Paste),
            C::Duplicate => self.run_action(hotkeys::Action::Duplicate),
            C::Delete => self.run_action(hotkeys::Action::Delete),
        }
    }

    fn handle_menu(&mut self, id: crate::menu::MenuId) -> Task<Message> {
        match id.0 {
            "cue.add-audio" => self.add_cue(CueKind::Audio),
            "cue.add-wait" => self.add_cue(CueKind::Wait),
            "cue.add-fade" => self.add_cue(CueKind::Fade),
            "cue.add-group" => self.add_cue(CueKind::Group),
            "cue.add-start" => self.add_cue(CueKind::Start),
            "cue.add-stop" => self.add_cue(CueKind::Stop),
            "cue.add-pause" => self.add_cue(CueKind::Pause),
            "cue.add-load" => self.add_cue(CueKind::Load),
            "edit.delete" => self.run_action(hotkeys::Action::Delete),
            "edit.copy" => self.run_action(hotkeys::Action::Copy),
            "edit.cut" => self.run_action(hotkeys::Action::Cut),
            "edit.paste" => self.run_action(hotkeys::Action::Paste),
            "edit.duplicate" => self.run_action(hotkeys::Action::Duplicate),
            "edit.select-all" => self.run_action(hotkeys::Action::SelectAll),
            "tools.renumber" => self.run_action(hotkeys::Action::Renumber),
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
            #[cfg(feature = "host")]
            "debug.open-gallery" => {
                self.gallery.open = true;
                Task::none()
            }
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
            // What a text field being edited needs to know, whether or not
            // a widget consumed the event: a press or Tab may have taken
            // the focus away from it, and Escape that a focused field
            // consumed means "throw the edit away". (Escape nobody
            // consumed is a hotkey, below.)
            match &event {
                // The cue list's context menu consumes the right press;
                // the app then selects the row it was on.
                iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                    if status == event::Status::Captured =>
                {
                    return Some(Message::CueList(cue_list::Message::RightClicked));
                }
                iced::Event::Mouse(mouse::Event::ButtonPressed(_)) => {
                    return Some(Message::Field(field::Message::FocusMayHaveChanged));
                }
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Tab),
                    ..
                }) => return Some(Message::Field(field::Message::FocusMayHaveChanged)),
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                }) if status == event::Status::Captured => {
                    return Some(Message::Field(field::Message::Escape));
                }
                _ => {}
            }

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
                iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                    Some(Message::FileDropped(path))
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

        // While dragging cues: the release (wherever it happens) drops
        // them, and a timer scrolls the list when the pointer is at its
        // edge.
        let dragging = if self.cue_drag.is_some() {
            Subscription::batch([
                iced::time::every(Duration::from_millis(16))
                    .map(|_| Message::CueList(cue_list::Message::DragTick)),
                event::listen_with(|event, _status, _window| match event {
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                        Some(Message::CueList(cue_list::Message::DragDropped))
                    }
                    _ => None,
                }),
            ])
        } else {
            Subscription::none()
        };

        // Files dropped together arrive as separate events; the first tick
        // after the last of them closes the batch.
        let drops = if self.dropped_files.is_empty() {
            Subscription::none()
        } else {
            iced::time::every(Duration::from_millis(100)).map(|_| Message::DropsSettled)
        };

        Subscription::batch([
            toasts,
            tick,
            clock,
            meter,
            backend_events,
            modifiers,
            dragging,
            drops,
        ])
    }

    fn menu_spec(&self) -> MenuSpec {
        let editing = self.ui.mode == Mode::Edit;
        let add = |id, label: String| MenuNode::item(id, label).enabled(editing);
        let has_selection = !self.ui.selected_rows.is_empty();

        MenuSpec::new()
            .menu(
                fl!("menu-file"),
                vec![MenuNode::item("file.quit", fl!("menu-file-quit"))],
            )
            .menu(
                fl!("menu-edit-title"),
                vec![
                    MenuNode::item("edit.copy", fl!("menu-edit-copy")).enabled(has_selection),
                    MenuNode::item("edit.cut", fl!("menu-edit-cut")).enabled(has_selection),
                    MenuNode::item("edit.paste", fl!("menu-edit-paste"))
                        .enabled(editing && !self.clipboard.is_empty()),
                    MenuNode::item("edit.duplicate", fl!("menu-edit-duplicate"))
                        .enabled(editing && has_selection),
                    MenuNode::separator(),
                    MenuNode::item("edit.select-all", fl!("menu-edit-select-all-cues")),
                    add("edit.delete", fl!("menu-edit-delete-cue")),
                ],
            )
            .menu(
                fl!("menu-cue-title"),
                vec![
                    add("cue.add-audio", fl!("menu-cue-audio")),
                    add("cue.add-wait", fl!("menu-cue-wait")),
                    add("cue.add-fade", fl!("menu-cue-fade")),
                    add("cue.add-group", fl!("menu-cue-group")),
                    MenuNode::separator(),
                    add("cue.add-start", fl!("menu-cue-start")),
                    add("cue.add-stop", fl!("menu-cue-stop")),
                    add("cue.add-pause", fl!("menu-cue-pause")),
                    add("cue.add-load", fl!("menu-cue-load")),
                ],
            )
            .menu(
                fl!("menu-tools-title"),
                vec![
                    MenuNode::item("tools.renumber", fl!("menu-tools-renumber"))
                        .enabled(editing && has_selection),
                ],
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
                    MenuNode::item("debug.open-gallery", "Input Gallery"),
                ],
            )
            .menu(
                fl!("menu-help"),
                vec![MenuNode::item("help.about", fl!("menu-help-about"))],
            )
    }

    fn editor_view(&self) -> Element<'_, Message> {
        let cue = self
            .ui
            .selected
            .and_then(|id| self.model.cue_list.cues.get(&id));
        let flat = crate::domain::flat_list::flatten(
            &self.model.cue_list.root_ids,
            &self.model.cue_list.cues,
            &self.ui.expanded_rows,
        );
        let chain_override = cue.and_then(|cue| {
            flat.iter()
                .find(|entry| entry.cue_id == cue.id && entry.is_chain_overridden)
                .map(|entry| entry.chain)
        });

        editor::view(
            &self.editor,
            editor::Context {
                cue,
                model: &self.model,
                name_format: &self.name_format,
                drafts: &self.fields,
                duration: crate::domain::cue::get_duration(cue, |_| {
                    cue.and_then(|cue| cue_list::asset_duration(cue, &self.assets))
                }),
                is_active: cue.is_some_and(|cue| self.playback.active_cues.contains_key(&cue.id)),
                chain_override,
                file: self.file_info(cue),
                lufs_target: self.model.settings.audio.lufs_target,
                waveform: cue
                    .and_then(|cue| match &cue.params {
                        CueParam::Audio(audio) => self.assets.get(&audio.target),
                        _ => None,
                    })
                    .map(|data| data.waveform.as_slice())
                    .filter(|waveform| !waveform.is_empty()),
                playing: cue
                    .and_then(|cue| self.playback.active_cues.get(&cue.id))
                    .map(|active| editor::Playing {
                        ratio: viewer::position_ratio(active),
                        duration: active.duration,
                    }),
            },
            &flat,
        )
        .map(Message::Editor)
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
                    crate::domain::cue::display_name(
                        cue,
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
                self.cue_view,
                (self.ui.mode == Mode::Edit).then(|| cue_list::EditMode {
                    can_paste: !self.clipboard.is_empty(),
                    drag: self.cue_drag.as_ref(),
                }),
            )
                .map(Message::CueList),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let main_column: Element<'_, Message> = if self.ui.is_bottom_tab_open {
            let (bottom, height): (Element<'_, Message>, f32) = if self.ui.mode == Mode::Edit {
                (self.editor_view(), editor::HEIGHT)
            } else {
                (
                    viewer::view(
                        &self.viewer,
                        &self.model,
                        &self.ui,
                        &self.playback,
                        &self.assets,
                    )
                    .map(Message::Viewer),
                    viewer::HEIGHT,
                )
            };
            let bottom = container(bottom)
                .width(Length::Fill)
                .height(height + 2.0)
                .padding(1)
                .style(container::bordered_box);
            column![main, bottom].into()
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
            row![main_column, divider::vertical(), sidebar].into()
        } else {
            main_column
        };

        let footer = container(
            status_bar::view(&self.ui, self.model.cue_list.cues.len()).map(Message::StatusBar),
        )
        .width(Length::Fill);

        let content: Element<'_, Message> = column![
            header,
            transport,
            divider::horizontal(),
            body,
            divider::horizontal(),
            footer
        ]
        .into();

        let content = if self.gallery.open {
            modal::over(
                content,
                self.gallery.view(&self.fields).map(Message::Gallery),
                Message::Gallery(debug_gallery::Message::Close),
            )
        } else {
            content
        };

        let content = match &self.pending_remove {
            Some(ids) => modal::over_blocking(content, remove_dialog(ids.len())),
            None => content,
        };

        let content = if self.renumber_open {
            modal::over_blocking(
                content,
                renumber::view(&self.renumber, &self.fields).map(Message::Renumber),
            )
        } else {
            content
        };

        self.toasts.overlay(content, Message::Toast)
    }
}

/// "Are you sure you want to remove?" with Cancel / Confirm.
fn remove_dialog<'a>(count: usize) -> Element<'a, Message> {
    use iced::widget::{button, text};

    column![
        text(fl!("dialog-message-confirmation")).size(18),
        text(fl!("dialog-message-remove-cue")),
        text(fl!("main-foot-bar-cue-count-suffix", n = (count as i64))).size(12),
        row![
            button(text(fl!("general-cancel")))
                .style(button::secondary)
                .on_press(Message::RemoveCancelled),
            button(text(fl!("general-confirm")))
                .style(button::danger)
                .on_press(Message::RemoveConfirmed),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .into()
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

    #[test]
    #[serial]
    fn the_gallery_renders_only_when_open() {
        let mut state = new_state();
        let mut ui = iced_test::simulator(state.view());
        assert!(ui.find("Input Gallery (debug)").is_err());
        drop(ui);

        state.gallery.open = true;
        let mut ui = iced_test::simulator(state.view());
        assert!(ui.find("Input Gallery (debug)").is_ok());
    }

    #[test]
    #[serial]
    fn a_field_edit_is_applied_when_submitted() {
        let mut state = new_state();
        state.gallery.open = true;
        let id = field::FieldId("gallery.number");

        let _ = state.update(Message::Field(field::Message::Input(id, "99".to_string())));
        assert_eq!(state.gallery.number, Some(5.0), "typing alone changes nothing");

        let _ = state.update(Message::Field(field::Message::Submit(id)));
        assert_eq!(state.gallery.number, Some(10.0), "clamped to the field's max");
    }

    #[test]
    #[serial]
    fn escape_discards_a_field_edit() {
        let mut state = new_state();
        let id = field::FieldId("gallery.number");

        let _ = state.update(Message::Field(field::Message::Input(id, "9".to_string())));
        let _ = state.update(Message::Field(field::Message::Escape));
        let _ = state.update(Message::Field(field::Message::Submit(id)));

        assert_eq!(state.gallery.number, Some(5.0));
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
}
