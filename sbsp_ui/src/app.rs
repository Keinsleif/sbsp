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

#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use iced::widget::{column, container, row};
use iced::{Element, Length, Subscription, Task, event, keyboard, mouse};

use crate::domain::{clipboard, cue_edit};
use crate::domain::new_cue::{self, CueKind};
#[cfg(feature = "host")]
use crate::domain::cursor::{CueEvent, CursorMove, advance};
use crate::features::{
    credits, cue_list, editor, file_ops, header, hotkeys, license_dialog, renumber, server_panel,
    settings_dialog, side_bar, status_bar, viewer,
};
use crate::i18n::Language;
use crate::menu::{MenuNode, MenuSpec};
use crate::state::preferences::{self, Preferences};
use crate::state::{
    assets::AssetResults,
    level_meter::LevelMeterState,
    model::ShowModelState,
    playback::PlaybackState,
    ui::{Mode, SideBarTab, UiState},
};
use crate::persist::{Saved, WindowRect};
use crate::theme::ThemeMode;
use crate::widgets::{divider, field, menu_bar, modal, toast};

#[cfg(feature = "host")]
use crate::host::backend::HostPort;
use crate::port::{Control, ModelEdit};
#[cfg(feature = "host")]
use crate::port::{FullState, PortEvent};

use sbsp_backend::helper::SupportedHardware;
use sbsp_backend::model::cue::{Cue, CueParam, Uuid};
use sbsp_frontend_settings::{AudioHardwareSettings, NameFormatSettings, TemplateSettings};
#[cfg(feature = "host")]
use sbsp_frontend_settings::GlobalHostSettings;

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
    let saved = crate::persist::load(target);
    // The window opens where and how big it was last time, if that still
    // makes sense.
    let rect = saved.window.filter(WindowRect::is_usable);
    let size = rect.map_or(iced::Size::new(1280.0, 720.0), |rect| {
        iced::Size::new(rect.width, rect.height)
    });
    let position = rect
        .and_then(|rect| rect.position())
        .map_or(iced::window::Position::Default, |(x, y)| {
            iced::window::Position::Specific(iced::Point::new(x, y))
        });

    iced::application(
        move || State::new(target, saved.clone()),
        State::update,
        State::view,
    )
        .title(|state: &State| format!("{} - {}", fl!("app-title"), state.model.name))
        .theme(|state: &State| crate::theme::resolve(state.theme_mode))
        .subscription(State::subscription)
        // Closing the window goes through `Message::CloseRequested`, which
        // may ask about unsaved changes first.
        .window(iced::window::Settings {
            size,
            position,
            maximized: rect.is_some_and(|rect| rect.maximized),
            exit_on_close_request: false,
            ..Default::default()
        })
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
    /// The settings as loaded (and as last saved).
    #[cfg(feature = "host")]
    host_settings: GlobalHostSettings,
    /// A question about replacing or closing the show, waiting for an
    /// answer.
    confirm: Option<file_ops::Confirm>,
    /// Cues waiting for the user to confirm their removal.
    pending_remove: Option<Vec<Uuid>>,
    /// The settings as last applied, which the settings dialog starts from.
    preferences: Preferences,
    /// What the audio hardware tab lists, once the system has said.
    hardware: Option<Result<SupportedHardware, String>>,
    /// Done was pressed with audio hardware changes while audio cues play;
    /// the question about stopping them is open.
    audio_warning: bool,
    /// The credits, while they are open.
    credits: Option<credits::Dialog>,
    /// The license dialog, while it is open.
    license_dialog: Option<license_dialog::Dialog>,
    /// The server panel, while it is open.
    server_panel: Option<server_panel::Panel>,
    /// The settings dialog, while it is open.
    settings_dialog: Option<settings_dialog::SettingsDialog>,
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
    /// Which app this is ("host", "remote" or "web"); picks where the
    /// remembered UI state is kept.
    target: &'static str,
    /// Where the window is and how big, as the OS last reported it.
    window_size: Option<iced::Size>,
    window_position: Option<iced::Point>,
    /// Whether the window is maximized (the size and position above are
    /// from before that).
    window_maximized: bool,
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
    /// The editor under the template list of the settings dialog.
    template_editor: editor::Editor,
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
    Renumber(renumber::Message),
    Settings(settings_dialog::Message),
    ServerPanel(server_panel::Message),
    Credits(credits::Message),
    License(license_dialog::Message),
    /// A license file was chosen and checked (`false`: none chosen).
    #[cfg(feature = "host")]
    LicenseActivated(Result<bool, String>),
    /// The server's options and whether it runs, as the panel opens.
    #[cfg(feature = "host")]
    ServerOpened(Box<sbsp_backend::api::ApiServerOptions>, bool),
    /// The server started (`true`) or stopped (`false`), or could not.
    #[cfg(feature = "host")]
    ServerToggled(Result<bool, String>),
    /// The answer to the audio hardware question.
    AudioWarning(bool),
    /// The devices the system offers.
    #[cfg(feature = "host")]
    HardwareLoaded(Result<Box<SupportedHardware>, String>),
    /// The files chosen in the add-audio-cue picker (empty: cancelled).
    AudioFilesPicked(Vec<std::path::PathBuf>),
    /// The file chosen for an audio cue's target (`None`: cancelled).
    AudioTargetPicked(Uuid, Option<std::path::PathBuf>),
    RemoveConfirmed,
    RemoveCancelled,
    File(file_ops::Command),
    Confirm(file_ops::Answer),
    /// The window's close button (or the Quit menu item).
    CloseRequested,
    /// A save finished: whether it happened (`false`: the dialog was
    /// cancelled), then what was waiting for it.
    FileSaved(Result<bool, String>, Option<file_ops::After>),
    #[cfg(feature = "host")]
    AssetRequestFailed(std::path::PathBuf, String),
    MeterTick(Instant),
    Header(header::Message),
    ClockTick,
    WindowFocused(bool),
    /// The OS moved or resized the window. Whether the window is maximized
    /// is asked before the new geometry is remembered (see `WindowSettled`).
    WindowMoved(iced::window::Id, iced::Point),
    WindowResized(iced::window::Id, iced::Size),
    /// The answer: the geometry to remember, unless the window is maximized
    /// (then the size and place from before it was maximized stay).
    WindowSettled {
        size: Option<iced::Size>,
        position: Option<iced::Point>,
        maximized: bool,
    },
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
    BackendStarted(HostPort, FullState, Box<GlobalHostSettings>),
    #[cfg(feature = "host")]
    PortEvent(PortEvent),
    #[cfg(feature = "host")]
    BackendError(String),
    /// The settings read from a file the user chose (`None`: cancelled).
    #[cfg(feature = "host")]
    SettingsImported(Result<Option<Box<GlobalHostSettings>>, String>),
}

/// What to tell the user about a failed backend operation. A custom error
/// has no message here.
#[cfg(any(test, feature = "host"))]
fn failure_message(error: &sbsp_backend::event::BackendError) -> Option<String> {
    use sbsp_backend::event::BackendError as E;
    let (title, detail) = match error {
        E::SaveToFile { message, .. } => (fl!("notification-failed-to-save"), message),
        E::LoadFromFile { message, .. } => (fl!("notification-failed-to-load"), message),
        E::ExportToFolder { message, .. } => (fl!("notification-failed-to-export"), message),
        E::CueExecute { message, .. } => (fl!("notification-failed-to-execute-cue"), message),
        E::CueEdit { message } => (fl!("notification-failed-to-edit-cue"), message),
        E::AudioOutputFallback { device, config } => {
            let (summary, detail) = match (device, config) {
                (true, true) => (
                    fl!("notification-audio-output-fallback-both-summary"),
                    fl!("notification-audio-output-fallback-both-detail"),
                ),
                (true, false) => (
                    fl!("notification-audio-output-fallback-device-summary"),
                    fl!("notification-audio-output-fallback-device-detail"),
                ),
                (false, true) => (
                    fl!("notification-audio-output-fallback-config-summary"),
                    fl!("notification-audio-output-fallback-config-detail"),
                ),
                (false, false) => return None,
            };
            return Some(format!("{summary}: {detail}"));
        }
        E::Custom { .. } => return None,
    };
    Some(format!("{title}: {detail}"))
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
    fn new(target: &'static str, saved: Saved) -> (Self, Task<Message>) {
        log::info!("starting sbsp_ui ({target})");

        #[cfg(feature = "host")]
        let boot_task = Task::perform(HostPort::start(), |result| match result {
            Ok((backend, state, settings)) => {
                Message::BackendStarted(backend, state, Box::new(settings))
            }
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
            #[cfg(feature = "host")]
            host_settings: GlobalHostSettings::default(),
            confirm: None,
            pending_remove: None,
            preferences: Preferences::default(),
            credits: None,
            license_dialog: None,
            server_panel: None,
            settings_dialog: None,
            hardware: None,
            audio_warning: false,
            renumber: renumber::Dialog::default(),
            renumber_open: false,
            dropped_files: Vec::new(),
            clipboard: Vec::new(),
            hovered_row: None,
            cue_drag: None,
            modifiers: keyboard::Modifiers::default(),
            clock: header::local_clock(),
            window_focused: true,
            target,
            window_size: saved
                .window
                .filter(WindowRect::is_usable)
                .map(|rect| iced::Size::new(rect.width, rect.height)),
            window_position: saved
                .window
                .and_then(|rect| rect.position())
                .map(|(x, y)| iced::Point::new(x, y)),
            window_maximized: saved.window.is_some_and(|rect| rect.maximized),
            hide_controls: sbsp_frontend_settings::AppearanceSettings::default().hide_controls,
            model: ShowModelState::default(),
            ui: {
                let mut ui = UiState::new(is_host);
                saved.apply(&mut ui);
                ui
            },
            playback: PlaybackState::new(),
            assets: AssetResults::new(),
            meter: LevelMeterState::default(),
            last_meter_tick: None,
            viewer: viewer::Viewer::default(),
            fields: field::Drafts::default(),
            editor: editor::Editor::default(),
            template_editor: editor::Editor::default(),
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
        self.template_editor.sync(
            self.settings_dialog.as_ref().and_then(|d| d.template_cue()),
            &self.fields,
        );
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
            Message::Renumber(message) => self.handle_renumber(message),
            Message::Settings(message) => self.handle_settings(message),
            Message::StatusBar(status_bar::Message::OpenSettings) => self.open_settings(),
            Message::StatusBar(status_bar::Message::OpenServerPanel) => self.open_server_panel(),
            Message::ServerPanel(message) => self.handle_server_panel(message),
            Message::Credits(message) => {
                match message {
                    credits::Message::Close => self.credits = None,
                    other => {
                        if let Some(dialog) = &mut self.credits {
                            dialog.update(&other);
                        }
                    }
                }
                Task::none()
            }
            Message::License(message) => self.handle_license(message),
            Message::AudioWarning(true) => {
                self.audio_warning = false;
                self.finish_settings(true)
            }
            Message::AudioWarning(false) => {
                self.audio_warning = false;
                Task::none()
            }
            Message::CueList(cue_list::Message::Context(action)) => {
                self.run_context_action(action)
            }
            Message::CueList(cue_list::Message::DragDropped) => self.finish_drag(),
            Message::CueList(cue_list::Message::DragTick) => self.scroll_while_dragging(),
            Message::AudioFilesPicked(files) => self.add_audio_cues(files),
            Message::AudioTargetPicked(id, file) => self.set_audio_target(id, file),
            Message::RemoveConfirmed => self.confirm_remove(),
            Message::File(command) => self.file_command(command),
            Message::Confirm(answer) => self.answer_confirm(answer),
            Message::CloseRequested => self.request_close(),
            Message::WindowMoved(id, position) => {
                iced::window::is_maximized(id).map(move |maximized| Message::WindowSettled {
                    size: None,
                    position: Some(position),
                    maximized,
                })
            }
            Message::WindowResized(id, size) => {
                iced::window::is_maximized(id).map(move |maximized| Message::WindowSettled {
                    size: Some(size),
                    position: None,
                    maximized,
                })
            }
            Message::FileSaved(result, after) => self.file_saved(result, after),
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
            message @ (Message::ToggleTheme | Message::ToggleLanguage) => {
                self.apply(message);
                self.save_appearance()
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

    /// Takes in loaded (or edited) settings: everything the UI reads from
    /// them without asking the settings again.
    #[cfg_attr(not(any(feature = "host", test)), allow(dead_code))]
    fn apply_preferences(&mut self, preferences: &Preferences) {
        self.preferences = preferences.clone();
        self.name_format = preferences.name_format.clone();
        self.lock_cursor_to_selection = preferences.general.lock_cursor_to_selection;
        self.seek_amount = preferences.general.seek_amount;
        self.hotkeys = hotkeys::Bindings::from_settings(&preferences.hotkey, hotkeys::is_mac());
        self.hide_controls = preferences.appearance.hide_controls;
        self.templates = preferences.template.clone();
        self.theme_mode = preferences::theme_mode(preferences.appearance.dark_mode);
        self.lang = Language::from_setting(
            preferences.appearance.language.as_deref(),
            crate::i18n::system_language().as_deref(),
        );
        crate::i18n::select(&self.lang.langid());
    }

    /// The View menu's theme and language switches are kept in the
    /// settings.
    #[cfg(feature = "host")]
    fn save_appearance(&mut self) -> Task<Message> {
        let appearance = &mut self.host_settings.appearance;
        appearance.dark_mode = preferences::dark_mode(self.theme_mode);
        appearance.language = Some(self.lang.setting().to_owned());
        match &self.backend {
            Some(backend) => Task::perform(
                backend.save_settings(self.host_settings.clone()),
                backend_result,
            ),
            None => Task::none(),
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
            Message::StatusBar(status_bar::Message::OpenSettings)
            | Message::StatusBar(status_bar::Message::OpenServerPanel)
            | Message::ServerPanel(_)
            | Message::Credits(_)
            | Message::License(_)
            | Message::AudioWarning(_) => {}
            #[cfg(feature = "host")]
            Message::LicenseActivated(result) => match result {
                Ok(true) => {
                    let info = self.license_info();
                    if let Some(dialog) = &mut self.license_dialog {
                        dialog.info = info;
                        dialog.activated = true;
                    }
                }
                Ok(false) => {}
                Err(message) => {
                    log::error!("license: {message}");
                    self.toasts.push(
                        toast::Kind::Danger,
                        format!("{}: {message}", fl!("dialog-message-license-pro-title")),
                    );
                }
            },
            #[cfg(feature = "host")]
            Message::ServerOpened(options, running) => {
                self.server_panel = Some(server_panel::Panel::new(&options, running));
            }
            #[cfg(feature = "host")]
            Message::ServerToggled(result) => match result {
                Ok(running) => {
                    if let Some(panel) = &mut self.server_panel {
                        panel.running = running;
                    }
                }
                Err(message) => {
                    log::error!("server: {message}");
                    self.toasts.push(
                        toast::Kind::Danger,
                        format!("{}: {message}", fl!("notification-failed-to-start-server")),
                    );
                }
            },
            #[cfg(feature = "host")]
            Message::HardwareLoaded(result) => self.hardware = Some(result.map(|boxed| *boxed)),
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
            | Message::Renumber(_)
            | Message::Settings(_)
            | Message::FileDropped(_)
            | Message::DropsSettled
            | Message::File(_)
            | Message::Confirm(_)
            | Message::CloseRequested
            | Message::FileSaved(..)
            | Message::AudioFilesPicked(_)
            | Message::AudioTargetPicked(..)
            | Message::RemoveConfirmed => {}
            Message::RemoveCancelled => self.pending_remove = None,
            Message::ClockTick => self.clock = header::local_clock(),
            Message::WindowFocused(focused) => self.window_focused = focused,
            Message::WindowSettled {
                size,
                position,
                maximized,
            } => {
                self.window_maximized = maximized;
                if !maximized {
                    self.window_size = size.or(self.window_size);
                    self.window_position = position.or(self.window_position);
                }
            }
            // Handled in `update_inner`, which can ask the window.
            Message::WindowMoved(..) | Message::WindowResized(..) => {}
            Message::Toast(message) => self.toasts.update(message),
            Message::Nothing => {}
            Message::Tick(now) => {
                let _ = self.playback.calculate_position(true, now);
            }
            #[cfg(feature = "host")]
            Message::BackendStarted(backend, state, settings) => {
                self.apply_preferences(&Preferences::from(&*settings));
                self.host_settings = *settings;
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
            #[cfg(feature = "host")]
            Message::SettingsImported(result) => match result {
                Ok(Some(settings)) => {
                    self.apply_preferences(&Preferences::from(&*settings));
                    self.host_settings = *settings;
                }
                Ok(None) => {}
                Err(message) => {
                    log::error!("settings import failed: {message}");
                    self.toasts.push(
                        toast::Kind::Danger,
                        format!("{}: {message}", fl!("notification-failed-to-apply-settings")),
                    );
                }
            },
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
        // The same for the question about replacing or closing the show.
        if self.confirm.is_some() {
            let is = |name: &str| press.names.iter().any(|n| n == name) && !press.repeat;
            return if is("Enter") {
                self.answer_confirm(file_ops::Answer::Yes)
            } else if is("Escape") {
                self.answer_confirm(file_ops::Answer::Cancel)
            } else {
                Task::none()
            };
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
        // Credits and license: Escape closes them; nothing reaches the cue
        // list behind them.
        if self.credits.is_some() || self.license_dialog.is_some() {
            if press.names.iter().any(|n| n == "Escape") && !press.repeat {
                if self.credits.is_some() {
                    self.credits = None;
                } else if let Some(dialog) = &mut self.license_dialog {
                    if dialog.activated {
                        dialog.activated = false;
                    } else {
                        self.license_dialog = None;
                    }
                }
            }
            return Task::none();
        }
        // The server panel: Escape closes the connect information, then
        // the panel; nothing reaches the cue list behind it.
        if let Some(panel) = &mut self.server_panel {
            if press.names.iter().any(|n| n == "Escape") && !press.repeat {
                if panel.info.is_some() {
                    panel.info = None;
                } else {
                    self.server_panel = None;
                }
            }
            return Task::none();
        }
        // The question about stopping audio for a hardware change.
        if self.audio_warning {
            let is = |name: &str| press.names.iter().any(|n| n == name) && !press.repeat;
            return if is("Enter") {
                self.audio_warning = false;
                self.finish_settings(true)
            } else if is("Escape") {
                self.audio_warning = false;
                Task::none()
            } else {
                Task::none()
            };
        }
        // The settings dialog takes Escape (cancel); other keys do not get
        // through to the cue list behind it.
        if let Some(dialog) = &mut self.settings_dialog {
            // An input waiting for a key takes any press, Escape included.
            if dialog.capturing.is_some() {
                if let (false, Some(name)) = (press.repeat, press.names.first()) {
                    dialog.capture(crate::domain::hotkey::format_binding(
                        press.mods,
                        name,
                        hotkeys::is_mac(),
                    ));
                }
                return Task::none();
            }
            if press.names.iter().any(|n| n == "Escape") && !press.repeat {
                self.settings_dialog = None;
            }
            return Task::none();
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
            A::Open => return self.file_command(file_ops::Command::Open),
            A::Save => return self.file_command(file_ops::Command::Save),
            A::SaveAs => return self.file_command(file_ops::Command::SaveAs),
            A::Export => return self.file_command(file_ops::Command::Export),
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
            | A::Renumber
            | A::Open
            | A::Save
            | A::SaveAs
            | A::Export => Vec::new(),
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

        if self.settings_dialog.is_some() && message == field::Message::Escape {
            self.settings_dialog = None;
        }

        let (outcome, task) = self.fields.update(message);
        let mut edit = Task::none();
        if let Some(outcome) = outcome {
            self.renumber.apply(&outcome);
            if self.settings_dialog.is_some() {
                // The editor fields then belong to the template, not to
                // the cue selected behind the dialog.
                self.apply_settings_outcome(&outcome);
            } else {
                // A step button press has no draft behind it: it applies
                // to the cue on screen.
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
        }
        if begins_editor_edit && self.settings_dialog.is_none() {
            self.editor.draft_cue = self.ui.selected;
        }

        let run = if submits_renumber {
            self.run_renumber()
        } else {
            Task::none()
        };

        Task::batch([task.map(Message::Field), edit, run])
    }

    /// Whether the show has changes that were not saved.
    #[cfg(feature = "host")]
    fn is_modified(&self) -> bool {
        self.backend
            .as_ref()
            .is_some_and(|backend| backend.handle().model_handle.is_modified())
    }

    #[cfg(not(feature = "host"))]
    fn is_modified(&self) -> bool {
        false
    }

    /// Does what the user asked once every question is answered.
    fn perform(&mut self, after: file_ops::After) -> Task<Message> {
        match after {
            file_ops::After::Quit => {
                self.save_session();
                iced::exit()
            }
            file_ops::After::New => self.reset_show(),
            file_ops::After::Open => self.open_show(),
        }
    }

    #[cfg(feature = "host")]
    fn reset_show(&self) -> Task<Message> {
        let Some(backend) = &self.backend else {
            return Task::none();
        };
        let handle = backend.handle();
        Task::perform(
            async move { handle.model_handle.reset().await },
            backend_result,
        )
    }

    #[cfg(feature = "host")]
    fn open_show(&self) -> Task<Message> {
        let Some(backend) = &self.backend else {
            return Task::none();
        };
        let handle = backend.handle();
        crate::host::files::dialog().then(move |dialog| {
            Task::perform(
                crate::host::files::open(handle.clone(), dialog),
                backend_result,
            )
        })
    }

    /// Saves (or saves as); `after` is what waits for it.
    #[cfg(feature = "host")]
    fn save(&self, ask_for_path: bool, after: Option<file_ops::After>) -> Task<Message> {
        let Some(backend) = &self.backend else {
            return Task::none();
        };
        let handle = backend.handle();
        crate::host::files::dialog().then(move |dialog| {
            Task::perform(
                crate::host::files::save(handle.clone(), dialog, ask_for_path),
                move |result| Message::FileSaved(result.map_err(|e| e.to_string()), after),
            )
        })
    }

    #[cfg(feature = "host")]
    fn export(&self) -> Task<Message> {
        let Some(backend) = &self.backend else {
            return Task::none();
        };
        let handle = backend.handle();
        crate::host::files::dialog().then(move |dialog| {
            Task::perform(
                crate::host::files::export(handle.clone(), dialog),
                |result| match result {
                    Ok(_) => Message::Nothing,
                    Err(error) => Message::BackendError(format!(
                        "{}: {error}",
                        fl!("notification-failed-to-export")
                    )),
                },
            )
        })
    }

    #[cfg(not(feature = "host"))]
    fn reset_show(&self) -> Task<Message> {
        Task::none()
    }

    #[cfg(not(feature = "host"))]
    fn open_show(&self) -> Task<Message> {
        Task::none()
    }

    #[cfg(not(feature = "host"))]
    fn save(&self, _ask_for_path: bool, _after: Option<file_ops::After>) -> Task<Message> {
        Task::none()
    }

    #[cfg(not(feature = "host"))]
    fn export(&self) -> Task<Message> {
        Task::none()
    }

    /// Writes down what the next run starts from: the window and the UI
    /// choices. Called when the app quits.
    fn save_session(&self) {
        let window = self.window_size.map(|size| WindowRect {
            width: size.width,
            height: size.height,
            x: self.window_position.map(|position| position.x),
            y: self.window_position.map(|position| position.y),
            maximized: self.window_maximized,
        });
        crate::persist::save(self.target, &Saved::capture(&self.ui, window));
    }

    /// Whether there is a log folder to show (not on the web, and not when
    /// the log file could not be opened).
    fn can_show_log_files(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::logging::dir().is_some()
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    /// Opens the log folder in the OS file manager.
    fn show_log_files(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(dir) = crate::logging::dir() {
            // Launching the file manager can block for a moment, so it
            // stays off the UI thread.
            std::thread::spawn(move || {
                if let Err(error) = opener::open(&dir) {
                    log::error!("failed to open {}: {error}", dir.display());
                }
            });
        }
    }

    /// What the license says right now (the host has a license).
    #[cfg(feature = "host")]
    fn license_info(&self) -> Option<license_dialog::Info> {
        use sbsp_license::data::LicenseEdition;

        let info = self.backend.as_ref()?.license_info()?;
        Some(license_dialog::Info {
            pro: info.edition == LicenseEdition::Pro,
            owner: info.owner,
            issue_time: info.issue_time,
            id: info.id.to_string(),
        })
    }

    #[cfg(not(feature = "host"))]
    fn license_info(&self) -> Option<license_dialog::Info> {
        None
    }

    fn handle_license(&mut self, message: license_dialog::Message) -> Task<Message> {
        use license_dialog::Message as M;

        match message {
            M::Close => {
                self.license_dialog = None;
                Task::none()
            }
            M::CloseNotice => {
                if let Some(dialog) = &mut self.license_dialog {
                    dialog.activated = false;
                }
                Task::none()
            }
            M::Reload => {
                let info = self.license_info();
                if let Some(dialog) = &mut self.license_dialog {
                    dialog.info = info;
                }
                Task::none()
            }
            M::Copy(text) => iced::clipboard::write(text),
            M::Activate => self.activate_license(),
        }
    }

    /// Asks for a license file and activates it.
    #[cfg(feature = "host")]
    fn activate_license(&self) -> Task<Message> {
        let Some(backend) = self.backend.clone() else {
            return Task::none();
        };
        crate::host::files::dialog().then(move |dialog| {
            let backend = backend.clone();
            Task::perform(
                async move {
                    let Some(path) = crate::host::files::pick_license(dialog).await else {
                        return Ok(false);
                    };
                    backend
                        .activate_license(path)
                        .await
                        .map(|()| true)
                        .map_err(|e| e.to_string())
                },
                Message::LicenseActivated,
            )
        })
    }

    #[cfg(not(feature = "host"))]
    fn activate_license(&self) -> Task<Message> {
        Task::none()
    }

    /// The footer's server button: opens the panel once the backend has
    /// said what the server is doing.
    #[cfg(feature = "host")]
    fn open_server_panel(&mut self) -> Task<Message> {
        match &self.backend {
            // As in the original, the server panel needs a Pro license.
            Some(backend) if !backend.is_pro() => {
                self.toasts.push(
                    toast::Kind::Info,
                    format!(
                        "{}: {}",
                        fl!("dialog-message-license-pro-title"),
                        fl!("dialog-message-license-server-panel")
                    ),
                );
                Task::none()
            }
            Some(backend) => Task::perform(backend.server_status(), |(options, running)| {
                Message::ServerOpened(Box::new(options), running)
            }),
            None => Task::none(),
        }
    }

    #[cfg(not(feature = "host"))]
    fn open_server_panel(&mut self) -> Task<Message> {
        Task::none()
    }

    fn handle_server_panel(&mut self, message: server_panel::Message) -> Task<Message> {
        use server_panel::Message as M;

        let Some(panel) = &mut self.server_panel else {
            return Task::none();
        };
        match message {
            M::Close => {
                self.server_panel = None;
                Task::none()
            }
            M::Toggle => self.toggle_server(),
            M::ShowInfo(index) => {
                panel.show_info(index, server_hostname());
                Task::none()
            }
            M::Copy(text) => {
                panel.update(&M::Copy(text.clone()));
                iced::clipboard::write(text)
            }
            other => {
                panel.update(&other);
                Task::none()
            }
        }
    }

    /// The panel's Start / Stop button.
    #[cfg(feature = "host")]
    fn toggle_server(&mut self) -> Task<Message> {
        let (Some(panel), Some(backend)) = (&self.server_panel, self.backend.clone()) else {
            return Task::none();
        };
        if panel.running {
            return Task::perform(
                async move {
                    backend.stop_server().await;
                    Ok(false)
                },
                Message::ServerToggled,
            );
        }
        let Some(options) = panel.options() else {
            self.toasts.push(
                toast::Kind::Danger,
                format!("{}: port", fl!("notification-failed-to-start-server")),
            );
            return Task::none();
        };
        Task::perform(
            async move {
                backend
                    .start_server(options)
                    .await
                    .map(|()| true)
                    .map_err(|e| e.to_string())
            },
            Message::ServerToggled,
        )
    }

    #[cfg(not(feature = "host"))]
    fn toggle_server(&mut self) -> Task<Message> {
        Task::none()
    }

    /// Edit > Import global settings: replaces the settings with a file's.
    #[cfg(feature = "host")]
    fn import_settings(&self) -> Task<Message> {
        let Some(backend) = self.backend.clone() else {
            return Task::none();
        };
        crate::host::files::dialog().then(move |dialog| {
            let backend = backend.clone();
            Task::perform(
                async move {
                    let Some(path) = crate::host::files::pick_settings(dialog).await else {
                        return Ok(None);
                    };
                    backend.import_settings(path).await.map(|s| Some(Box::new(s)))
                },
                |result| Message::SettingsImported(result.map_err(|e| e.to_string())),
            )
        })
    }

    /// Edit > Export global settings: writes the settings to a file.
    #[cfg(feature = "host")]
    fn export_settings(&self) -> Task<Message> {
        let Some(backend) = self.backend.clone() else {
            return Task::none();
        };
        crate::host::files::dialog().then(move |dialog| {
            let backend = backend.clone();
            Task::perform(
                async move {
                    let Some(path) = crate::host::files::settings_destination(dialog).await else {
                        return Ok(());
                    };
                    backend.export_settings(path).await
                },
                backend_result,
            )
        })
    }

    #[cfg(not(feature = "host"))]
    fn import_settings(&self) -> Task<Message> {
        Task::none()
    }

    #[cfg(not(feature = "host"))]
    fn export_settings(&self) -> Task<Message> {
        Task::none()
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

    fn handle_settings(&mut self, message: settings_dialog::Message) -> Task<Message> {
        match message {
            settings_dialog::Message::Field(message) => self.handle_field(message),
            settings_dialog::Message::Cancel => {
                self.settings_dialog = None;
                Task::none()
            }
            settings_dialog::Message::Done => self.finish_settings(false),
            settings_dialog::Message::Editor(message) => self.handle_template_editor(message.0),
            other => {
                // An edit still in progress belongs to what is on screen
                // now, not to what the click is about to show.
                if matches!(
                    other,
                    settings_dialog::Message::SelectTab(_)
                        | settings_dialog::Message::SelectTemplate(_)
                )
                    && let Some(outcome) = self.fields.flush() {
                        self.apply_settings_outcome(&outcome);
                    }
                if let Some(dialog) = &mut self.settings_dialog {
                    dialog.update(&other);
                }
                Task::none()
            }
        }
    }

    /// A finished edit of one of the dialog's fields: the dialog's own, or
    /// an editor field of the template being edited.
    fn apply_settings_outcome(&mut self, outcome: &field::Outcome) {
        let Some(dialog) = &mut self.settings_dialog else {
            return;
        };
        dialog.apply(outcome);
        let edited = dialog
            .template_cue()
            .and_then(|cue| self.template_editor.apply(outcome, cue));
        if let Some(edited) = edited {
            dialog.set_template_cue(edited);
        }
    }

    /// A message of the editor under the template list.
    fn handle_template_editor(&mut self, message: editor::Message) -> Task<Message> {
        let Some(dialog) = &mut self.settings_dialog else {
            return Task::none();
        };
        let update = self
            .template_editor
            .update(message, dialog.template_cue(), None, Instant::now());
        match update {
            editor::Update::Field(message) => return self.handle_field(message),
            editor::Update::Edit(edited) => dialog.set_template_cue(*edited),
            // Playing, seeking and analysed files mean nothing to a
            // template.
            editor::Update::None
            | editor::Update::PickAudio(_)
            | editor::Update::Seek(..)
            | editor::Update::MatchVolume(_) => {}
        }
        Task::none()
    }

    /// The settings as they are right now: the View menu's switches may
    /// have changed the theme and the language since they were applied.
    fn current_preferences(&self) -> Preferences {
        let mut preferences = self.preferences.clone();
        preferences.appearance.dark_mode = preferences::dark_mode(self.theme_mode);
        preferences.appearance.language = Some(self.lang.setting().to_owned());
        preferences
    }

    fn open_settings(&mut self) -> Task<Message> {
        self.template_editor = editor::Editor::default();
        self.hardware = None;
        let dialog = settings_dialog::SettingsDialog::new(
            &self.model.name,
            &self.model.settings,
            &self.current_preferences(),
        );
        self.settings_dialog = Some(match self.audio_settings() {
            Some(audio) => dialog.with_audio(audio),
            None => dialog,
        });
        self.load_hardware()
    }

    /// The audio hardware settings as saved (the host has them).
    #[cfg(feature = "host")]
    fn audio_settings(&self) -> Option<AudioHardwareSettings> {
        Some(self.host_settings.audio.clone())
    }

    #[cfg(not(feature = "host"))]
    fn audio_settings(&self) -> Option<AudioHardwareSettings> {
        None
    }

    #[cfg(feature = "host")]
    fn load_hardware(&self) -> Task<Message> {
        Task::perform(crate::host::backend::hardware(), |result| {
            Message::HardwareLoaded(result.map(Box::new).map_err(|e| e.to_string()))
        })
    }

    #[cfg(not(feature = "host"))]
    fn load_hardware(&self) -> Task<Message> {
        Task::none()
    }

    /// Whether an audio cue is playing, which a change of the output
    /// hardware would cut off.
    fn has_active_audio_cue(&self) -> bool {
        self.playback.active_cues.keys().any(|id| {
            self.model
                .cue_list
                .cues
                .get(id)
                .is_some_and(|cue| matches!(cue.params, CueParam::Audio(_)))
        })
    }

    /// The dialog's Done: applies the edited settings, saves the global
    /// ones and hands the show's own to the backend, each only when it
    /// changed.
    ///
    /// Changing the audio hardware while audio cues play asks first
    /// (`confirmed`: it has been answered).
    fn finish_settings(&mut self, confirmed: bool) -> Task<Message> {
        // The field being edited counts: the button's press may arrive
        // before the field notices it lost the focus.
        if let Some(outcome) = self.fields.flush() {
            self.apply_settings_outcome(&outcome);
        }
        let audio_changed = self
            .settings_dialog
            .as_ref()
            .is_some_and(|dialog| dialog.audio != self.audio_settings());
        if audio_changed && !confirmed && self.has_active_audio_cue() {
            self.audio_warning = true;
            return Task::none();
        }
        let Some(dialog) = self.settings_dialog.take() else {
            return Task::none();
        };

        let mut tasks = Vec::new();
        let preferences_changed = dialog.preferences != self.current_preferences();
        if preferences_changed {
            self.apply_preferences(&dialog.preferences);
        }
        if preferences_changed || audio_changed {
            tasks.push(self.save_preferences(dialog.audio));
        }
        if dialog.show != self.model.settings {
            tasks.push(self.send_edit(ModelEdit::UpdateShowSettings(dialog.show)));
        }
        if dialog.show_name != self.model.name {
            tasks.push(self.send_edit(ModelEdit::RenameShow(dialog.show_name)));
        }
        Task::batch(tasks)
    }

    /// Keeps the applied preferences in the settings file.
    #[cfg(feature = "host")]
    fn save_preferences(&mut self, audio: Option<AudioHardwareSettings>) -> Task<Message> {
        let p = self.preferences.clone();
        let settings = &mut self.host_settings;
        settings.general = p.general;
        settings.appearance = p.appearance;
        settings.hotkey = p.hotkey;
        settings.template = p.template;
        settings.name_format = p.name_format;
        if let Some(audio) = audio {
            settings.audio = audio;
        }
        match &self.backend {
            Some(backend) => Task::perform(
                backend.save_settings(self.host_settings.clone()),
                backend_result,
            ),
            None => Task::none(),
        }
    }

    #[cfg(not(feature = "host"))]
    fn save_preferences(&mut self, _audio: Option<AudioHardwareSettings>) -> Task<Message> {
        Task::none()
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
        crate::host::files::dialog().then(move |dialog| {
            let title = title.clone();
            Task::perform(
                async move {
                    dialog
                        .set_title(title)
                        .add_filter("Audio", new_cue::AUDIO_EXTENSIONS)
                        .pick_file()
                        .await
                        .map(|file| file.path().to_path_buf())
                },
                move |file| Message::AudioTargetPicked(id, file),
            )
        })
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
        crate::host::files::dialog().then(move |dialog| {
            let title = title.clone();
            Task::perform(
                async move {
                    dialog
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
        })
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

    /// A file menu entry or hotkey.
    fn file_command(&mut self, command: file_ops::Command) -> Task<Message> {
        use file_ops::{After, Command as C};
        match command {
            C::New => self.begin(After::New),
            C::Open => self.begin(After::Open),
            C::Save => self.save(false, None),
            C::SaveAs => self.save(true, None),
            C::Export => self.export(),
        }
    }

    /// The window's close button and the Quit menu item.
    fn request_close(&mut self) -> Task<Message> {
        // A question is already waiting; closing again changes nothing.
        if self.confirm.is_some() {
            return Task::none();
        }
        self.begin(file_ops::After::Quit)
    }

    /// Starts something that replaces or ends the show: asks what needs
    /// asking first, then does it.
    fn begin(&mut self, after: file_ops::After) -> Task<Message> {
        use file_ops::{Confirm, Step};
        match file_ops::first_step(
            after,
            !self.playback.active_cues.is_empty(),
            self.is_modified(),
        ) {
            Step::AskStop => {
                self.confirm = Some(Confirm::StopActive(after));
                Task::none()
            }
            Step::AskSave => {
                self.confirm = Some(Confirm::SaveChanges(after));
                Task::none()
            }
            Step::Go => self.perform(after),
        }
    }

    fn answer_confirm(&mut self, answer: file_ops::Answer) -> Task<Message> {
        use file_ops::{Answer, Confirm, Step};
        let Some(confirm) = self.confirm.take() else {
            return Task::none();
        };
        match (confirm, answer) {
            (_, Answer::Cancel) | (Confirm::StopActive(_), Answer::No) => Task::none(),
            (Confirm::StopActive(after), Answer::Yes) => {
                match file_ops::after_stop_confirmed(self.is_modified()) {
                    Step::Go => self.perform(after),
                    _ => {
                        self.confirm = Some(Confirm::SaveChanges(after));
                        Task::none()
                    }
                }
            }
            (Confirm::SaveChanges(after), Answer::Yes) => self.save(false, Some(after)),
            (Confirm::SaveChanges(after), Answer::No) => self.perform(after),
        }
    }

    fn file_saved(
        &mut self,
        result: Result<bool, String>,
        after: Option<file_ops::After>,
    ) -> Task<Message> {
        match result {
            Ok(true) => after.map_or_else(Task::none, |after| self.perform(after)),
            Ok(false) => Task::none(),
            Err(error) => {
                log::error!("save failed: {error}");
                self.toasts.push(
                    toast::Kind::Danger,
                    format!("{}: {error}", fl!("notification-failed-to-save")),
                );
                Task::none()
            }
        }
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
            "help.credits" => {
                self.credits = Some(credits::Dialog::default());
                Task::none()
            }
            "help.license" => {
                if cfg!(feature = "host") {
                    self.license_dialog = Some(license_dialog::Dialog::new(self.license_info()));
                }
                Task::none()
            }
            "help.show-log-files" => {
                self.show_log_files();
                Task::none()
            }
            "help.about" => {
                self.toasts
                    .push(toast::Kind::Info, fl!("menu-help-about-message"));
                Task::none()
            }
            "file.new" => self.file_command(file_ops::Command::New),
            "file.open" => self.file_command(file_ops::Command::Open),
            "file.save" => self.file_command(file_ops::Command::Save),
            "file.save-as" => self.file_command(file_ops::Command::SaveAs),
            "file.export" => self.file_command(file_ops::Command::Export),
            "file.quit" => self.request_close(),
            "edit.import-settings" => self.import_settings(),
            "edit.export-settings" => self.export_settings(),
            other => {
                log::warn!("unhandled menu id: {other}");
                Task::none()
            }
        }
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
                if let Some(message) = failure_message(&error) {
                    self.toasts.push(toast::Kind::Danger, message);
                }
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
        let modifiers = event::listen_with(|event, status, window| {
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
                iced::Event::Window(iced::window::Event::Moved(position)) => {
                    Some(Message::WindowMoved(window, position))
                }
                iced::Event::Window(iced::window::Event::Resized(size)) => {
                    Some(Message::WindowResized(window, size))
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

        let close = iced::window::close_requests().map(|_| Message::CloseRequested);

        Subscription::batch([
            close,
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
        // Opening and saving shows is the host's job; a remote or the web
        // build has no show of its own.
        let is_mac = hotkeys::is_mac();
        let shortcut = |action| self.hotkeys.shortcut(action, is_mac);
        let host_only = |id, label: String, action| {
            MenuNode::item(id, label)
                .enabled(cfg!(feature = "host"))
                .with_shortcut_opt(shortcut(action))
        };

        MenuSpec::new()
            .menu(
                fl!("menu-file"),
                vec![
                    MenuNode::item("file.new", fl!("menu-file-new"))
                        .enabled(cfg!(feature = "host")),
                    host_only("file.open", fl!("menu-file-open"), hotkeys::Action::Open),
                    host_only("file.save", fl!("menu-file-save"), hotkeys::Action::Save),
                    host_only(
                        "file.save-as",
                        fl!("menu-file-save-as"),
                        hotkeys::Action::SaveAs,
                    ),
                    host_only(
                        "file.export",
                        fl!("menu-file-export-to-folder"),
                        hotkeys::Action::Export,
                    ),
                    MenuNode::separator(),
                    MenuNode::item("file.quit", fl!("menu-file-quit")),
                ],
            )
            .menu(
                fl!("menu-edit-title"),
                vec![
                    MenuNode::item("edit.copy", fl!("menu-edit-copy"))
                        .enabled(has_selection)
                        .with_shortcut_opt(shortcut(hotkeys::Action::Copy)),
                    MenuNode::item("edit.cut", fl!("menu-edit-cut"))
                        .enabled(has_selection)
                        .with_shortcut_opt(shortcut(hotkeys::Action::Cut)),
                    MenuNode::item("edit.paste", fl!("menu-edit-paste"))
                        .enabled(editing && !self.clipboard.is_empty())
                        .with_shortcut_opt(shortcut(hotkeys::Action::Paste)),
                    MenuNode::item("edit.duplicate", fl!("menu-edit-duplicate"))
                        .enabled(editing && has_selection)
                        .with_shortcut_opt(shortcut(hotkeys::Action::Duplicate)),
                    MenuNode::separator(),
                    MenuNode::item("edit.import-settings", fl!("menu-edit-import-settings"))
                        .enabled(cfg!(feature = "host")),
                    MenuNode::item("edit.export-settings", fl!("menu-edit-export-settings"))
                        .enabled(cfg!(feature = "host")),
                    MenuNode::separator(),
                    MenuNode::item("edit.select-all", fl!("menu-edit-select-all-cues"))
                        .with_shortcut_opt(shortcut(hotkeys::Action::SelectAll)),
                    add("edit.delete", fl!("menu-edit-delete-cue"))
                        .with_shortcut_opt(shortcut(hotkeys::Action::Delete)),
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
                        .enabled(editing && has_selection)
                        .with_shortcut_opt(shortcut(hotkeys::Action::Renumber)),
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
            .menu(
                fl!("menu-help"),
                vec![
                    MenuNode::item("help.credits", fl!("menu-help-credits")),
                    MenuNode::item("help.license", fl!("menu-help-license"))
                        .enabled(cfg!(feature = "host")),
                    MenuNode::item("help.show-log-files", fl!("menu-help-show-log-files"))
                        .enabled(self.can_show_log_files()),
                    MenuNode::separator(),
                    MenuNode::item("help.about", fl!("menu-help-about")),
                ],
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
                (self.ui.mode == Mode::Edit).then_some(cue_list::EditMode {
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
            status_bar::view(
                &self.ui,
                self.model.cue_list.cues.len(),
                cfg!(feature = "host"),
            ).map(Message::StatusBar),
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

        let content = match &self.pending_remove {
            Some(ids) => modal::over_blocking(content, remove_dialog(ids.len())),
            None => content,
        };

        let content = match self.confirm {
            Some(confirm) => modal::over_blocking(
                content,
                file_ops::view(confirm).map(Message::Confirm),
            ),
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

        let content = match &self.settings_dialog {
            Some(dialog) => {
                let flat = crate::domain::flat_list::flatten(
                    &self.model.cue_list.root_ids,
                    &self.model.cue_list.cues,
                    &self.ui.expanded_rows,
                );
                let editing = settings_dialog::Editing {
                    editor: &self.template_editor,
                    model: &self.model,
                    flat: &flat,
                    hardware: self.hardware.as_ref(),
                };
                modal::over_blocking(
                    content,
                    settings_dialog::view(dialog, &self.fields, editing).map(Message::Settings),
                )
            }
            None => content,
        };

        let content = match &self.credits {
            Some(dialog) => modal::over_blocking(
                content,
                credits::view(dialog).map(Message::Credits),
            ),
            None => content,
        };

        let content = match &self.license_dialog {
            Some(dialog) => modal::over_blocking(
                content,
                license_dialog::view(dialog).map(Message::License),
            ),
            None => content,
        };

        let content = match &self.server_panel {
            Some(panel) => modal::over_blocking(
                content,
                server_panel::view(panel).map(Message::ServerPanel),
            ),
            None => content,
        };

        let content = if self.audio_warning {
            modal::over_blocking(content, audio_warning_dialog())
        } else {
            content
        };

        self.toasts.overlay(content, Message::Toast)
    }
}

/// This computer's name as the network knows it, for the address clients
/// connect to.
#[cfg(feature = "host")]
fn server_hostname() -> Option<String> {
    sbsp_backend::api::server::get_mdns_hostname().ok()
}

#[cfg(not(feature = "host"))]
fn server_hostname() -> Option<String> {
    None
}

/// "Stop playback and apply settings?" with Cancel / Confirm.
fn audio_warning_dialog<'a>() -> Element<'a, Message> {
    use iced::widget::{button, text};

    column![
        text(fl!("dialog-message-confirmation")).size(18),
        text(fl!("dialog-settings-global-audio-hardware-save-warning")),
        row![
            button(text(fl!("general-cancel")))
                .style(button::secondary)
                .on_press(Message::AudioWarning(false)),
            button(text(fl!("general-confirm")))
                .style(button::danger)
                .on_press(Message::AudioWarning(true)),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .into()
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
