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

use crate::time::{Duration, Instant};

use iced::widget::{column, container, row};
use iced::{Element, Length, Subscription, Task, event, keyboard, mouse};

use crate::domain::{clipboard, cue_edit};
use crate::domain::new_cue::{self, CueKind};
#[cfg(backend)]
use crate::domain::cursor::{CueEvent, CursorMove, advance};
#[cfg(client)]
use crate::features::{connect, file_list};
use crate::features::{
    credits, cue_list, editor, file_ops, header, hotkeys, license_dialog, mobile, renumber, server_panel,
    settings_dialog, side_bar, status_bar, update_dialog, viewer,
};
use crate::i18n::Language;
use crate::menu::{MenuNode, MenuSpec};
use crate::state::preferences::{self, Preferences};
use crate::state::{
    assets::AssetResults,
    level_meter::LevelMeterState,
    model::ShowModelState,
    playback::PlaybackState,
    ui::{KeepAwakeMode, Mode, SideBarTab, UiState},
};
use crate::persist::{Saved, WindowRect};
use crate::theme::ThemeMode;
use crate::widgets::{divider, field, menu_bar, modal, toast};

#[cfg(feature = "host")]
use crate::host::backend::HostPort;
use crate::port::{Control, ModelEdit};
#[cfg(backend)]
use crate::port::FullState;
#[cfg(backend)]
use crate::port::{Port, PortEvent};
#[cfg(client)]
use crate::remote::backend::{RemotePort, RemoteSettings};
#[cfg(installer)]
use crate::features::update_dialog::Stage;

use sbsp_backend::helper::SupportedHardware;
use sbsp_backend::model::cue::{Cue, CueParam, Uuid};
use crate::settings::{AudioHardwareSettings, NameFormatSettings, TemplateSettings};
#[cfg(feature = "host")]
use crate::settings::GlobalHostSettings;
#[cfg(client)]
use crate::settings::GlobalRemoteSettings;

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

/// How often the backend is asked where the playing cues are (the original
/// asked every 5 seconds too).
#[cfg(backend)]
const SYNC_INTERVAL: Duration = Duration::from_secs(5);

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

    let app = iced::application(
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
        ;
    // The browser fetches its fonts (see `State::new`).
    #[cfg(not(target_arch = "wasm32"))]
    let app = app
        .font(crate::theme::fonts::UI_REGULAR)
        .font(crate::theme::fonts::MONO_REGULAR);
    app.default_font(crate::theme::fonts::UI).run()
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
    update_dialog: Option<update_dialog::Dialog>,
    /// The update that was found, to download.
    #[cfg(installer)]
    update_found: Option<crate::update::Found>,
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
    /// The request that keeps the machine awake, held while
    /// `ui.keep_awake` and the window focus ask for it.
    power: crate::power::KeepAwake,
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
    #[cfg(backend)]
    assets_stale: bool,
    toasts: toast::Toasts,
    /// What the app talks to: the host's own backend, or the one a remote
    /// is connected to.
    #[cfg(backend)]
    backend: Option<Port>,
    /// The remote's own settings and where they are kept (loaded at start).
    #[cfg(client)]
    remote_settings: GlobalRemoteSettings,
    #[cfg(client)]
    remote_store: Option<RemoteSettings>,
    /// The host's audio files to choose from (the remote has none of its own).
    #[cfg(client)]
    file_picker: Option<file_list::Dialog>,
    /// The remote's connect screen (shown while there is no connection).
    #[cfg(client)]
    connect: connect::Screen,
    /// The phone layout: the screen shown, and the seek bar while dragged.
    mobile_tab: mobile::Tab,
    mobile_seek: Option<f64>,
    /// The connection is being closed by the user, so its end is not news.
    #[cfg(client)]
    disconnecting: bool,
    /// The browser has no fonts until they are fetched; nothing is drawn
    /// (it would be boxes) before then.
    #[cfg(target_arch = "wasm32")]
    fonts_ready: bool,
    /// A connection to make as soon as the settings are in (the web page's
    /// address asked for one).
    #[cfg(client)]
    autoconnect: Option<connect::Request>,
}

#[derive(Clone)]
pub enum Message {
    ToggleTheme,
    ToggleLanguage,
    ModifiersChanged(keyboard::Modifiers),
    KeyPressed(hotkeys::KeyPress),
    CueList(cue_list::Message),
    /// The phone layout's own controls.
    Mobile(mobile::Message),
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
    Update(update_dialog::Message),
    /// The feed was asked for a newer version (`true`: at startup, with no
    /// dialog open).
    #[cfg(installer)]
    UpdateChecked(bool, Result<Option<crate::update::Found>, String>),
    #[cfg(installer)]
    UpdateProgress(crate::update::Progress),
    /// Installing failed (on success the app has restarted).
    #[cfg(installer)]
    UpdateApplied(String),
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
    #[cfg(backend)]
    AssetRequestFailed(std::path::PathBuf, String),
    MeterTick(Instant),
    /// The browser's fonts arrived (or did not).
    #[cfg(target_arch = "wasm32")]
    FontsFetched(Result<std::sync::Arc<(Vec<u8>, Vec<u8>)>, String>),
    #[cfg(target_arch = "wasm32")]
    FontsLoaded(Result<(), String>),
    Header(header::Message),
    ClockTick,
    WindowFocused(bool),
    SetKeepAwake(KeepAwakeMode),
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
    /// The connect screen.
    #[cfg(client)]
    RemoteSettingsLoaded(RemoteSettings, Box<GlobalRemoteSettings>),
    #[cfg(client)]
    RemoteSettingsImported(Result<Option<Box<GlobalRemoteSettings>>, String>),
    /// Show the host's files to choose from.
    #[cfg(client)]
    OpenFilePicker(file_list::Purpose),
    #[cfg(client)]
    FilePicker(file_list::Message),
    /// The host sent its list of files.
    #[cfg(client)]
    FileListReceived(Vec<sbsp_backend::api::FileList>),
    #[cfg(client)]
    Connect(connect::Message),
    /// The hosts found on the network.
    #[cfg(client)]
    Discovered(Vec<sbsp_backend::api::client::ServiceEntry>),
    #[cfg(client)]
    RemoteConnected(RemotePort, Box<FullState>),
    #[cfg(client)]
    RemoteFailed(String),
    /// The connection ended.
    #[cfg(client)]
    RemoteClosed,
    /// Time to ask the backend where the playing cues are.
    #[cfg(backend)]
    SyncTick,
    #[cfg(backend)]
    PortEvent(PortEvent),
    #[cfg(backend)]
    BackendError(String),
    /// The settings read from a file the user chose (`None`: cancelled).
    #[cfg(feature = "host")]
    SettingsImported(Result<Option<Box<GlobalHostSettings>>, String>),
}

/// What to tell the user about a failed backend operation.
#[cfg(any(test, backend))]
fn failure_message(error: &sbsp_backend::event::BackendError) -> Option<String> {
    use sbsp_backend::event::BackendError as E;
    let custom;
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
        // The errors the API server reports (see `WsError`): the ids are the
        // ones `sbsp_backend`'s client uses.
        E::Custom { id, message } => {
            custom = match id {
                1 => fl!("notification-authentication-failed"),
                2 => fl!("notification-authentication-rate-limit-exceeded"),
                3 => fl!("notification-permission-denied"),
                _ => message.clone(),
            };
            (fl!("notification-operation-failed"), &custom)
        }
    };
    Some(format!("{title}: {detail}"))
}

/// Where the browser keeps the cue clipboard.
#[cfg(target_arch = "wasm32")]
const CLIPBOARD_KEY: &str = "sbsp.web.clipboard";

/// The connect screen to start with, and the connection to make at once.
/// Only the browser's page address can ask for one.
#[cfg(all(client, target_arch = "wasm32"))]
fn startup_connection() -> (connect::Screen, Option<connect::Request>) {
    crate::web::startup_connection()
}

#[cfg(all(client, not(target_arch = "wasm32")))]
fn startup_connection() -> (connect::Screen, Option<connect::Request>) {
    (connect::Screen::default(), None)
}

/// Turns the outcome of a fire-and-forget backend call into a `Message`
/// (`Task::perform` needs one value either way).
#[cfg(backend)]
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
        #[cfg(all(client, not(feature = "host")))]
        let boot_task = Task::perform(RemoteSettings::load(), |(store, settings)| {
            Message::RemoteSettingsLoaded(store, Box::new(settings))
        });
        #[cfg(not(any(feature = "host", client)))]
        let boot_task = Task::none();

        #[cfg(target_arch = "wasm32")]
        let boot_task = Task::batch([
            boot_task,
            Task::perform(crate::web::fetch_fonts(), Message::FontsFetched),
        ]);

        #[cfg(installer)]
        let boot_task = Task::batch([
            boot_task,
            Task::perform(crate::update::check(target), |result| {
                Message::UpdateChecked(true, result)
            }),
        ]);

        let is_host = cfg!(feature = "host");
        let lang = Language::default();
        // Redundant with LOADER's own lazy initialization today (it loads
        // the fallback language, which is also Language::default()), but
        // explicit rather than relying on that coincidence -- if the two
        // ever diverge this still does the right thing instead of silently
        // booting in the wrong language.
        crate::i18n::select(&lang.langid());

        #[cfg(client)]
        let (connect, autoconnect) = startup_connection();

        let mut state = Self {
            theme_mode: ThemeMode::default(),
            lang,
            name_format: NameFormatSettings::default(),
            lock_cursor_to_selection: crate::settings::GeneralSettings::default()
                .lock_cursor_to_selection,
            cue_view: cue_list::Viewport::default(),
            seek_amount: crate::settings::GeneralSettings::default().seek_amount,
            hotkeys: hotkeys::Bindings::from_settings(
                &crate::settings::hotkey::HotkeySettings::default(),
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
            update_dialog: None,
            #[cfg(installer)]
            update_found: None,
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
            power: crate::power::KeepAwake::default(),
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
            hide_controls: crate::settings::AppearanceSettings::default().hide_controls,
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
            #[cfg(backend)]
            assets_stale: false,
            mobile_tab: mobile::Tab::default(),
            mobile_seek: None,
            toasts: toast::Toasts::new(),
            #[cfg(backend)]
            backend: None,
            #[cfg(client)]
            remote_settings: GlobalRemoteSettings::default(),
            #[cfg(client)]
            remote_store: None,
            #[cfg(client)]
            file_picker: None,
            #[cfg(client)]
            connect,
            #[cfg(client)]
            disconnecting: false,
            #[cfg(client)]
            autoconnect,
            #[cfg(target_arch = "wasm32")]
            fonts_ready: false,
        };

        state.sync_power();
        state.refresh_clipboard();
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

    /// Starts or ends the "do not sleep" request to match the chosen mode
    /// and the window focus.
    fn sync_power(&mut self) {
        self.power
            .set(crate::power::wanted(self.ui.keep_awake, self.window_focused));
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
            Message::Mobile(message) => self.handle_mobile(message),
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
            Message::Update(message) => self.handle_update(message),
            #[cfg(installer)]
            Message::UpdateChecked(silent, result) => self.update_checked(silent, result),
            #[cfg(installer)]
            Message::UpdateProgress(progress) => self.update_progress(progress),
            #[cfg(installer)]
            Message::UpdateApplied(error) => {
                if let Some(dialog) = &mut self.update_dialog {
                    dialog.stage = update_dialog::Stage::Failed(error);
                }
                Task::none()
            }
            #[cfg(target_arch = "wasm32")]
            Message::FontsFetched(Ok(fonts)) => {
                let mono = fonts.1.clone();
                iced::font::load(fonts.0.clone())
                    .then(move |first| match first {
                        Ok(()) => iced::font::load(mono.clone()),
                        Err(error) => Task::done(Err(error)),
                    })
                    .map(|result| Message::FontsLoaded(result.map_err(|e| format!("{e:?}"))))
            }
            #[cfg(target_arch = "wasm32")]
            Message::FontsFetched(Err(error)) => Task::done(Message::FontsLoaded(Err(error))),
            #[cfg(target_arch = "wasm32")]
            Message::FontsLoaded(result) => {
                if let Err(error) = result {
                    log::error!("could not load the fonts: {error}");
                    self.toasts.push(toast::Kind::Danger, format!("fonts: {error}"));
                }
                self.fonts_ready = true;
                Task::none()
            }
            #[cfg(client)]
            Message::Connect(message) => self.handle_connect(message),
            #[cfg(client)]
            Message::OpenFilePicker(purpose) => self.open_file_picker(purpose),
            #[cfg(client)]
            Message::FilePicker(message) => self.handle_file_picker(message),
            #[cfg(client)]
            Message::FileListReceived(list) => {
                if let Some(picker) = &mut self.file_picker {
                    picker.set_list(list);
                }
                Task::none()
            }
            #[cfg(client)]
            Message::Discovered(services) => {
                self.connect.services = services;
                Task::none()
            }
            #[cfg(client)]
            Message::RemoteConnected(port, state) => self.remote_connected(port, *state),
            #[cfg(client)]
            Message::RemoteFailed(message) => {
                self.connect.connecting = false;
                log::error!("connection failed: {message}");
                self.toasts.push(
                    toast::Kind::Danger,
                    format!("{}: {message}", fl!("notification-connection-error")),
                );
                Task::none()
            }
            #[cfg(client)]
            Message::RemoteClosed => {
                self.remote_closed();
                Task::none()
            }
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
            #[cfg(all(client, not(feature = "host")))]
            message @ (Message::ToggleTheme | Message::ToggleLanguage) => {
                self.apply(message);
                self.save_remote_appearance()
            }
            #[cfg(client)]
            Message::RemoteSettingsLoaded(store, settings) => {
                self.apply_preferences(&Preferences::from(&*settings));
                self.remote_settings = *settings;
                self.remote_store = Some(store);
                match self.autoconnect.take() {
                    Some(request) => self.connect_to(request),
                    None => Task::none(),
                }
            }
            #[cfg(backend)]
            Message::SyncTick => {
                // A browser tab is closed without a word, so what the next
                // visit starts from is written down as it goes.
                #[cfg(target_arch = "wasm32")]
                {
                    self.save_session();
                    self.refresh_clipboard();
                }
                match &self.backend {
                    Some(backend) => {
                        Task::perform(backend.request_state_sync(), |()| Message::Nothing)
                    }
                    None => Task::none(),
                }
            }
            #[cfg(feature = "host")]
            message @ Message::BackendStarted(..) => {
                self.apply(message);
                self.request_assets()
            }
            #[cfg(backend)]
            message @ Message::PortEvent(_) => {
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
        match self.backend.as_ref().and_then(Port::host) {
            Some(backend) => Task::perform(
                backend.save_settings(self.host_settings.clone()),
                backend_result,
            ),
            None => Task::none(),
        }
    }

    /// The remote's View menu choices go into its own settings file.
    #[cfg(client)]
    fn save_remote_appearance(&mut self) -> Task<Message> {
        let appearance = &mut self.remote_settings.appearance;
        appearance.dark_mode = preferences::dark_mode(self.theme_mode);
        appearance.language = Some(self.lang.setting().to_owned());
        match &self.remote_store {
            Some(store) => Task::perform(store.save(self.remote_settings.clone()), backend_result),
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
            | Message::Update(_)
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
            Message::Menu(_) | Message::KeyPressed(_) | Message::Header(_) | Message::Mobile(_)
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
            Message::WindowFocused(focused) => {
                self.window_focused = focused;
                self.sync_power();
            }
            Message::SetKeepAwake(mode) => {
                self.ui.keep_awake = mode;
                self.sync_power();
            }
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
            // Handled in `update_inner`, which can start the connection.
            #[cfg(client)]
            Message::Connect(_)
            | Message::Discovered(_)
            | Message::RemoteConnected(..)
            | Message::RemoteFailed(_)
            | Message::RemoteClosed
            | Message::OpenFilePicker(_)
            | Message::FilePicker(_)
            | Message::FileListReceived(_)
            | Message::RemoteSettingsLoaded(..) => {}
            #[cfg(backend)]
            Message::SyncTick => {}
            Message::Toast(message) => self.toasts.update(message),
            #[cfg(target_arch = "wasm32")]
            Message::FontsFetched(_) | Message::FontsLoaded(_) => {}
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
                self.backend = Some(Port::Host(backend));
                self.assets_stale = true;
            }
            #[cfg(backend)]
            Message::AssetRequestFailed(path, message) => {
                log::warn!("asset request for {} failed: {message}", path.display());
                self.assets.add_error(path);
            }
            #[cfg(backend)]
            Message::PortEvent(event) => self.handle_port_event(event),
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
            #[cfg(client)]
            Message::RemoteSettingsImported(result) => match result {
                Ok(Some(settings)) => {
                    self.apply_preferences(&Preferences::from(&*settings));
                    self.remote_settings = *settings;
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
            #[cfg(backend)]
            Message::BackendError(message) => {
                log::error!("backend error: {message}");
                self.toasts.push(toast::Kind::Danger, message);
            },
            #[cfg(installer)]
            Message::UpdateChecked(_, _) | Message::UpdateProgress(_) | Message::UpdateApplied(_) => {}
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

        #[cfg(backend)]
        if let Some((left, right)) = self.backend.as_ref().and_then(|backend| backend.level()) {
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
        // The host's file list: Escape closes it.
        #[cfg(client)]
        if self.file_picker.is_some() {
            if press.names.iter().any(|n| n == "Escape") && !press.repeat {
                return self.handle_file_picker(file_list::Message::Cancel);
            }
            return Task::none();
        }
        // The password question of the connect screen: Escape cancels it.
        #[cfg(client)]
        if self.connect.prompt.is_some() {
            if press.names.iter().any(|n| n == "Escape") && !press.repeat {
                return self.handle_connect(connect::Message::Cancel);
            }
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
        if self.credits.is_some() || self.license_dialog.is_some() || self.update_dialog.is_some() {
            if press.names.iter().any(|n| n == "Escape") && !press.repeat {
                if let Some(dialog) = &self.update_dialog {
                    if !dialog.busy() {
                        self.update_dialog = None;
                    }
                } else if self.credits.is_some() {
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
            A::FadeOut => selected_rows().map(Control::SoftStop).collect(),
            A::HardStop => selected_rows().map(Control::HardStop).collect(),
            A::ToggleRepeat => selected_rows().map(Control::ToggleRepeat).collect(),
            A::PauseAndResume => match selected_active {
                Some((id, S::PreWaiting | S::Playing)) => vec![Control::Pause(id)],
                Some((id, S::PreWaitPaused | S::Paused)) => vec![Control::Resume(id)],
                _ => Vec::new(),
            },
            A::PauseAll => vec![Control::PauseAll],
            A::ResumeAll => vec![Control::ResumeAll],
            A::StopAll => vec![Control::StopAll],
            A::FadeAllOut => vec![Control::SoftStopAll],
            A::HardStopAll => vec![Control::HardStopAll],
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
    #[cfg(backend)]
    fn is_modified(&self) -> bool {
        self.backend
            .as_ref()
            .is_some_and(|backend| backend.is_modified())
    }

    #[cfg(not(backend))]
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
        let Some(backend) = self.backend.as_ref().and_then(Port::host) else {
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
        let Some(backend) = self.backend.as_ref().and_then(Port::host) else {
            return Task::none();
        };
        let handle = backend.handle();
        crate::files::dialog().then(move |dialog| {
            Task::perform(
                crate::files::open(handle.clone(), dialog),
                backend_result,
            )
        })
    }

    /// Saves (or saves as); `after` is what waits for it.
    #[cfg(feature = "host")]
    fn save(&self, ask_for_path: bool, after: Option<file_ops::After>) -> Task<Message> {
        let Some(backend) = self.backend.as_ref().and_then(Port::host) else {
            return Task::none();
        };
        let handle = backend.handle();
        crate::files::dialog().then(move |dialog| {
            Task::perform(
                crate::files::save(handle.clone(), dialog, ask_for_path),
                move |result| Message::FileSaved(result.map_err(|e| e.to_string()), after),
            )
        })
    }

    #[cfg(feature = "host")]
    fn export(&self) -> Task<Message> {
        let Some(backend) = self.backend.as_ref().and_then(Port::host) else {
            return Task::none();
        };
        let handle = backend.handle();
        crate::files::dialog().then(move |dialog| {
            Task::perform(
                crate::files::export(handle.clone(), dialog),
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

    /// Whether a backend is there to work with (a remote that has
    /// connected; the host once it has started).
    fn is_connected(&self) -> bool {
        #[cfg(backend)]
        {
            self.backend.is_some()
        }
        #[cfg(not(backend))]
        {
            false
        }
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

        let info = self.backend.as_ref().and_then(Port::host)?.license_info()?;
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

    /// Shows the host's files and asks the host for the list.
    #[cfg(client)]
    fn open_file_picker(&mut self, purpose: file_list::Purpose) -> Task<Message> {
        let Some(port) = self.backend.as_ref().and_then(Port::remote) else {
            return Task::none();
        };
        self.file_picker = Some(file_list::Dialog::new(purpose));
        Task::perform(port.request_file_list(), backend_result)
    }

    #[cfg(client)]
    fn handle_file_picker(&mut self, message: file_list::Message) -> Task<Message> {
        use file_list::{Outcome, Purpose};

        let Some(picker) = &mut self.file_picker else {
            return Task::none();
        };
        let purpose = picker.purpose;
        let Some(outcome) = picker.update(message) else {
            return Task::none();
        };
        self.file_picker = None;
        match (outcome, purpose) {
            (Outcome::Cancelled, _) => Task::none(),
            (Outcome::Files(files), Purpose::Add) => self.add_audio_cues(files),
            (Outcome::Files(files), Purpose::Target(id)) => {
                self.set_audio_target(id, files.into_iter().next())
            }
        }
    }

    /// A message from the connect screen; once the password is given, the
    /// connection is made.
    #[cfg(client)]
    fn handle_connect(&mut self, message: connect::Message) -> Task<Message> {
        let Some(request) = self.connect.update(message) else {
            return Task::none();
        };
        self.connect_to(request)
    }

    /// Starts connecting.
    #[cfg(client)]
    fn connect_to(&mut self, request: connect::Request) -> Task<Message> {
        self.connect.connecting = true;
        Task::perform(
            RemotePort::connect(request.address, request.password),
            |result| match result {
                Ok((port, state)) => Message::RemoteConnected(port, Box::new(state)),
                Err(e) => Message::RemoteFailed(e.to_string()),
            },
        )
    }

    /// Takes in the host's state, like the host does for its own backend,
    /// with the permissions the host granted.
    #[cfg(client)]
    fn remote_connected(&mut self, port: RemotePort, state: FullState) -> Task<Message> {
        log::info!(
            "connected to {}, {} active cue(s)",
            port.address(),
            state.show_state.active_cues.len()
        );
        self.connect.connecting = false;
        self.disconnecting = false;
        self.playback.update(&state.show_state, Instant::now());
        self.model = state.show_model;
        self.ui.set_playback_cursor(
            self.model.cue_list.root_ids.first().copied(),
            self.lock_cursor_to_selection,
            &self.model.cue_list.cues,
        );
        self.ui.set_permission(port.permission());
        self.assets_stale = true;
        let closed = port.closed();
        // The host sends its output level only to a connection that asks.
        let stream_level = port.stream_level(true);
        self.backend = Some(Port::Remote(port));
        Task::batch([
            Task::perform(closed, |()| Message::RemoteClosed),
            Task::perform(stream_level, |()| Message::Nothing),
            self.request_assets(),
        ])
    }

    /// The connection ended: back to the connect screen with nothing of
    /// the host's left.
    #[cfg(client)]
    fn remote_closed(&mut self) {
        self.backend = None;
        self.file_picker = None;
        self.model = ShowModelState::default();
        self.playback = PlaybackState::new();
        self.assets.clear();
        self.ui.reset_selected();
        self.ui.expanded_rows.clear();
        self.ui.playback_cursor = None;
        self.ui.set_permission(sbsp_backend::api::Permissions::empty());
        self.clipboard.clear();
        self.store_clipboard();
        self.cue_drag = None;
        self.pending_remove = None;
        if !std::mem::take(&mut self.disconnecting) {
            self.toasts
                .push(toast::Kind::Warning, fl!("notification-connection-lost"));
        }
    }

    /// The File menu's Disconnect.
    #[cfg(client)]
    fn disconnect(&mut self) -> Task<Message> {
        match self.backend.as_ref().and_then(Port::remote) {
            Some(port) => {
                self.disconnecting = true;
                Task::perform(port.disconnect(), |()| Message::Nothing)
            }
            None => Task::none(),
        }
    }

    fn handle_update(&mut self, message: update_dialog::Message) -> Task<Message> {
        use update_dialog::Message as M;

        match message {
            M::Close => {
                if self.update_dialog.as_ref().is_some_and(|d| !d.busy()) {
                    self.update_dialog = None;
                }
                Task::none()
            }
            M::Check => self.start_update_check(),
            M::Install => {
                #[cfg(installer)]
                if let Some(found) = self.update_found.clone() {
                    if let Some(dialog) = &mut self.update_dialog {
                        dialog.stage = Stage::Downloading(0);
                    }
                    return Task::run(
                        crate::update::download(self.target, found),
                        Message::UpdateProgress,
                    );
                }
                Task::none()
            }
        }
    }

    /// Opens the update dialog and asks the feed.
    fn start_update_check(&mut self) -> Task<Message> {
        use update_dialog::Stage;

        #[cfg(installer)]
        {
            if !crate::update::installed(self.target) {
                self.update_dialog = Some(update_dialog::Dialog::new(Stage::Unavailable));
                return Task::none();
            }
            self.update_found = None;
            self.update_dialog = Some(update_dialog::Dialog::new(Stage::Checking));
            Task::perform(crate::update::check(self.target), |result| {
                Message::UpdateChecked(false, result)
            })
        }
        #[cfg(not(installer))]
        {
            self.update_dialog = Some(update_dialog::Dialog::new(Stage::Unavailable));
            Task::none()
        }
    }

    #[cfg(installer)]
    fn update_checked(
        &mut self,
        silent: bool,
        result: Result<Option<crate::update::Found>, String>,
    ) -> Task<Message> {
        use update_dialog::Stage;

        if silent {
            match result {
                Ok(Some(found)) => {
                    log::info!("update {} is available", found.version());
                    self.update_found = Some(found);
                    self.toasts
                        .push(toast::Kind::Info, fl!("toast-update-available"));
                }
                Ok(None) => {}
                // Not installed, or no network: not worth bothering about.
                Err(error) => log::info!("update check skipped: {error}"),
            }
            return Task::none();
        }
        let Some(dialog) = &mut self.update_dialog else {
            return Task::none();
        };
        dialog.stage = match result {
            Ok(Some(found)) => {
                let version = found.version();
                self.update_found = Some(found);
                Stage::Available(version)
            }
            Ok(None) => Stage::UpToDate,
            Err(error) => {
                log::error!("update check failed: {error}");
                Stage::Failed(error)
            }
        };
        Task::none()
    }

    #[cfg(installer)]
    fn update_progress(&mut self, progress: crate::update::Progress) -> Task<Message> {
        use crate::update::Progress;
        use update_dialog::Stage;

        let Some(dialog) = &mut self.update_dialog else {
            return Task::none();
        };
        match progress {
            Progress::Percent(percent) => dialog.stage = Stage::Downloading(percent),
            Progress::Failed(error) => {
                log::error!("update download failed: {error}");
                dialog.stage = Stage::Failed(error);
            }
            Progress::Done => {
                dialog.stage = Stage::Restarting;
                if let Some(found) = self.update_found.clone() {
                    return Task::perform(
                        crate::update::apply(self.target, found),
                        Message::UpdateApplied,
                    );
                }
            }
        }
        Task::none()
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
        let Some(backend) = self.backend.as_ref().and_then(Port::host).cloned() else {
            return Task::none();
        };
        crate::files::dialog().then(move |dialog| {
            let backend = backend.clone();
            Task::perform(
                async move {
                    let Some(path) = crate::files::pick_license(dialog).await else {
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
        match self.backend.as_ref().and_then(Port::host) {
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
        let (Some(panel), Some(backend)) = (&self.server_panel, self.backend.as_ref().and_then(Port::host).cloned()) else {
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
        let Some(backend) = self.backend.as_ref().and_then(Port::host).cloned() else {
            return Task::none();
        };
        crate::files::dialog().then(move |dialog| {
            let backend = backend.clone();
            Task::perform(
                async move {
                    let Some(path) = crate::files::pick_settings(dialog).await else {
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
        let Some(backend) = self.backend.as_ref().and_then(Port::host).cloned() else {
            return Task::none();
        };
        crate::files::dialog().then(move |dialog| {
            let backend = backend.clone();
            Task::perform(
                async move {
                    let Some(path) = crate::files::settings_destination(dialog).await else {
                        return Ok(());
                    };
                    backend.export_settings(path).await
                },
                backend_result,
            )
        })
    }

    #[cfg(all(feature = "remote", not(feature = "host")))]
    fn import_settings(&self) -> Task<Message> {
        let Some(store) = self.remote_store.clone() else {
            return Task::none();
        };
        crate::files::dialog().then(move |dialog| {
            let store = store.clone();
            Task::perform(
                async move {
                    let Some(path) = crate::files::pick_settings(dialog).await else {
                        return Ok(None);
                    };
                    store.import(path).await.map(|s| Some(Box::new(s)))
                },
                |result| Message::RemoteSettingsImported(result.map_err(|e| e.to_string())),
            )
        })
    }

    #[cfg(all(feature = "remote", not(feature = "host")))]
    fn export_settings(&self) -> Task<Message> {
        let Some(store) = self.remote_store.clone() else {
            return Task::none();
        };
        crate::files::dialog().then(move |dialog| {
            let store = store.clone();
            Task::perform(
                async move {
                    let Some(path) = crate::files::settings_destination(dialog).await else {
                        return Ok(());
                    };
                    store.export(path).await
                },
                backend_result,
            )
        })
    }

    #[cfg(not(any(feature = "host", feature = "remote")))]
    fn import_settings(&self) -> Task<Message> {
        Task::none()
    }

    #[cfg(not(any(feature = "host", feature = "remote")))]
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
        match self.backend.as_ref().and_then(Port::host) {
            Some(backend) => Task::perform(
                backend.save_settings(self.host_settings.clone()),
                backend_result,
            ),
            None => Task::none(),
        }
    }

    #[cfg(all(client, not(feature = "host")))]
    fn save_preferences(&mut self, _audio: Option<AudioHardwareSettings>) -> Task<Message> {
        let p = self.preferences.clone();
        let settings = &mut self.remote_settings;
        settings.general = p.general;
        settings.appearance = p.appearance;
        settings.hotkey = p.hotkey;
        settings.template = p.template;
        settings.name_format = p.name_format;
        match &self.remote_store {
            Some(store) => Task::perform(store.save(self.remote_settings.clone()), backend_result),
            None => Task::none(),
        }
    }

    #[cfg(not(any(feature = "host", client)))]
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
        crate::files::dialog().then(move |dialog| {
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

    #[cfg(all(client, not(feature = "host")))]
    fn pick_audio_target(&self, id: Uuid) -> Task<Message> {
        Task::done(Message::OpenFilePicker(file_list::Purpose::Target(id)))
    }

    #[cfg(not(any(feature = "host", client)))]
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
    #[cfg(backend)]
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

    #[cfg(backend)]
    fn send(&self, controls: Vec<Control>) -> Task<Message> {
        let Some(backend) = &self.backend else {
            return Task::none();
        };
        if controls.is_empty() {
            return Task::none();
        }

        Task::perform(backend.send(controls), backend_result)
    }

    /// Web has no backend connection yet (Phase 9).
    #[cfg(not(backend))]
    fn send(&self, controls: Vec<Control>) -> Task<Message> {
        if !controls.is_empty() {
            log::warn!("no backend to send {controls:?} to");
        }
        Task::none()
    }

    #[cfg(backend)]
    fn send_edit(&self, edit: ModelEdit) -> Task<Message> {
        let Some(backend) = &self.backend else {
            return Task::none();
        };
        Task::perform(backend.edit(edit), backend_result)
    }

    /// Web has no backend connection yet (Phase 9).
    #[cfg(not(backend))]
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
        crate::files::dialog().then(move |dialog| {
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

    #[cfg(all(client, not(feature = "host")))]
    fn pick_audio_files(&self) -> Task<Message> {
        Task::done(Message::OpenFilePicker(file_list::Purpose::Add))
    }

    #[cfg(not(any(feature = "host", client)))]
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
        // A remote's files are on the wrong computer: the host cannot play
        // them, and the cue would name a path that is only here.
        if cfg!(all(client, not(feature = "host")))
            || self.ui.mode != Mode::Edit
            || files.is_empty()
        {
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
            self.store_clipboard();
        }
    }

    /// The browser keeps the clipboard in its storage as well, so that it
    /// survives a reload and is the same in every tab of the page (the
    /// system clipboard is out of reach of a page served over plain HTTP).
    fn store_clipboard(&self) {
        #[cfg(target_arch = "wasm32")]
        match serde_json::to_string(&self.clipboard) {
            Ok(json) => crate::browser::set(CLIPBOARD_KEY, &json),
            Err(error) => log::warn!("could not keep the clipboard: {error}"),
        }
    }

    /// Takes the stored clipboard (see `store_clipboard`) in place of this
    /// tab's, which another tab may have replaced.
    fn refresh_clipboard(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(json) = crate::browser::get(CLIPBOARD_KEY) {
            match serde_json::from_str::<Vec<Cue>>(&json) {
                Ok(cues) => self.clipboard = cues,
                Err(error) => log::warn!("ignoring the stored clipboard: {error}"),
            }
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
        self.refresh_clipboard();
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
            "tools.keep-awake.never" => self.update(Message::SetKeepAwake(KeepAwakeMode::Never)),
            "tools.keep-awake.focused" => {
                self.update(Message::SetKeepAwake(KeepAwakeMode::WhileFocused))
            }
            "tools.keep-awake.always" => self.update(Message::SetKeepAwake(KeepAwakeMode::Always)),
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
            "help.check-update" => self.start_update_check(),
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
            #[cfg(client)]
            "file.disconnect" => self.disconnect(),
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
    #[cfg(backend)]
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
    #[cfg(backend)]
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

    #[cfg(backend)]
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

        #[cfg(backend)]
        let backend_events = self
            .backend
            .as_ref()
            .map(|backend| backend.events().map(Message::PortEvent))
            .unwrap_or(Subscription::none());
        #[cfg(not(backend))]
        let backend_events = Subscription::none();

        // The hosts on the network, while there is no connection to choose
        // from them for.
        #[cfg(client)]
        let discovery = if self.backend.is_none() {
            crate::remote::backend::discovery().map(Message::Discovered)
        } else {
            Subscription::none()
        };
        #[cfg(not(client))]
        let discovery = Subscription::none();

        // The host's list of files, while it is being chosen from.
        #[cfg(client)]
        let file_lists = match (&self.file_picker, self.backend.as_ref().and_then(Port::remote)) {
            (Some(_), Some(port)) => port.file_lists().map(Message::FileListReceived),
            _ => Subscription::none(),
        };
        #[cfg(not(client))]
        let file_lists = Subscription::none();

        // The positions of playing cues are worked out between the
        // backend's reports; asking for one every few seconds keeps them
        // from drifting (and, for a remote, from the connection's delay).
        #[cfg(backend)]
        let sync = if self.backend.is_some() {
            iced::time::every(SYNC_INTERVAL).map(|_| Message::SyncTick)
        } else {
            Subscription::none()
        };
        #[cfg(not(backend))]
        let sync = Subscription::none();

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
            discovery,
            file_lists,
            sync,
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

        let mut file_menu = vec![
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
        ];
        // A remote can end its connection.
        if cfg!(client) {
            let quit = file_menu.len() - 1;
            file_menu.insert(
                quit,
                MenuNode::item("file.disconnect", fl!("menu-file-disconnect"))
                    .enabled(self.is_connected()),
            );
        }

        MenuSpec::new()
            .menu(fl!("menu-file"), file_menu)
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
                        .enabled(cfg!(any(feature = "host", feature = "remote"))),
                    MenuNode::item("edit.export-settings", fl!("menu-edit-export-settings"))
                        .enabled(cfg!(any(feature = "host", feature = "remote"))),
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
                    MenuNode::submenu(
                        fl!("menu-tools-keep-awake"),
                        [
                            ("tools.keep-awake.never", fl!("menu-tools-keep-awake-never"), KeepAwakeMode::Never),
                            ("tools.keep-awake.focused", fl!("menu-tools-keep-awake-focused"), KeepAwakeMode::WhileFocused),
                            ("tools.keep-awake.always", fl!("menu-tools-keep-awake-always"), KeepAwakeMode::Always),
                        ]
                        .into_iter()
                        .map(|(id, label, mode)| {
                            MenuNode::item(id, label)
                                .checked(self.ui.keep_awake == mode)
                                // The web build cannot hold the request.
                                .enabled(cfg!(any(feature = "host", feature = "remote")))
                        })
                        .collect(),
                    ),
                ],
            )
            .menu(
                fl!("menu-help"),
                vec![
                    MenuNode::item("help.credits", fl!("menu-help-credits")),
                    MenuNode::item("help.license", fl!("menu-help-license"))
                        .enabled(cfg!(feature = "host")),
                    MenuNode::item("help.check-update", fl!("menu-help-check-update"))
                        .enabled(cfg!(installer)),
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

    fn menu_header(&self) -> Element<'_, Message> {
        container(menu_bar::menu_bar(&self.menu_spec(), Message::Menu))
            .width(Length::Fill)
            .padding(4)
            .into()
    }

    /// The main screen of the desktop (and of a browser wide enough for it).
    fn desktop_view(&self) -> Element<'_, Message> {
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
                .height(height)
                .padding(1);
            column![main, divider::horizontal(), bottom].into()
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

        column![
            self.menu_header(),
            transport,
            divider::horizontal(),
            body,
            divider::horizontal(),
            footer
        ]
        .into()
    }

    /// The main screen of a phone.
    fn mobile_view(&self) -> Element<'_, Message> {
        let can_control = self.ui.permission.contains(sbsp_backend::api::Permissions::CONTROL);
        let tab = mobile::effective_tab(self.mobile_tab, can_control);
        let body: Element<'_, Message> = match tab {
            mobile::Tab::List => cue_list::view(
                &self.model,
                &self.ui,
                &self.playback,
                &self.assets,
                &self.name_format,
                self.cue_view,
                None,
            )
            .map(Message::CueList),
            mobile::Tab::Controls => mobile::controls(self.mobile_controls()).map(Message::Mobile),
            mobile::Tab::Active => side_bar::active_cues(
                &self.model,
                &self.ui,
                &self.playback,
                &self.name_format,
            )
            .map(Message::SideBar),
        };
        mobile::frame(self.clock, tab, can_control, body, |tab| {
            Message::Mobile(mobile::Message::Tab(tab))
        })
    }

    /// What the controls screen shows: the cue under the playback cursor.
    fn mobile_controls(&self) -> mobile::Controls {
        use sbsp_backend::controller::state::StateParam;

        let cue = self
            .ui
            .playback_cursor
            .and_then(|id| self.model.cue_list.cues.get(&id));
        let active = self
            .ui
            .playback_cursor
            .and_then(|id| self.playback.active_cues.get(&id));
        mobile::Controls {
            title: cue
                .map(|cue| {
                    let name = crate::domain::cue::display_name(
                        cue,
                        &self.model.cue_list.cues,
                        &self.name_format,
                    );
                    header::cue_title(cue, name)
                })
                .unwrap_or_default(),
            duration: cue.and_then(|cue| cue_list::asset_duration(cue, &self.assets)),
            status: active.map(|active| active.status),
            playing: active.map(|active| mobile::Playing {
                position: active.position,
                duration: active.duration,
                repeating: matches!(active.params, StateParam::Audio(audio) if audio.repeating),
            }),
            dragging: self.mobile_seek,
        }
    }

    fn handle_mobile(&mut self, message: mobile::Message) -> Task<Message> {
        let cursor = self.ui.playback_cursor;
        let running = cursor.is_some_and(|id| self.playback.active_cues.contains_key(&id));

        match message {
            mobile::Message::Tab(tab) => {
                self.mobile_tab = tab;
                Task::none()
            }
            mobile::Message::Transport(message) => self.handle_header(message),
            mobile::Message::Skip(direction) => {
                let target = mobile::skip(
                    direction,
                    cursor,
                    &self.model.cue_list.root_ids,
                    &self.model.cue_list.cues,
                );
                if let Some(id) = target {
                    self.ui.set_playback_cursor(
                        Some(id),
                        self.lock_cursor_to_selection,
                        &self.model.cue_list.cues,
                    );
                }
                Task::none()
            }
            mobile::Message::Rewind | mobile::Message::FastForward => match cursor {
                Some(id) if running => {
                    let amount = if message == mobile::Message::Rewind {
                        -self.seek_amount
                    } else {
                        self.seek_amount
                    };
                    self.send(vec![Control::SeekBy(id, amount)])
                }
                _ => Task::none(),
            },
            mobile::Message::ToggleRepeat => match cursor {
                Some(id) if running => self.send(vec![Control::ToggleRepeat(id)]),
                _ => Task::none(),
            },
            mobile::Message::Seek(position) => {
                self.mobile_seek = Some(position);
                Task::none()
            }
            mobile::Message::SeekReleased => match (self.mobile_seek.take(), cursor) {
                (Some(position), Some(id)) if running => {
                    self.send(vec![Control::SeekTo(id, position)])
                }
                _ => Task::none(),
            },
        }
    }

    fn view(&self) -> Element<'_, Message> {
        #[cfg(target_arch = "wasm32")]
        if !self.fonts_ready {
            return container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        // A remote without a connection has only the connect screen.
        #[cfg(client)]
        if self.needs_connection() {
            return self.connect_view(self.menu_header());
        }

        // A phone gets its own layout (the web build; the desktop keeps the
        // one it has however narrow the window is).
        let content: Element<'_, Message> = if cfg!(target_arch = "wasm32") {
            iced::widget::responsive(move |size| {
                if size.width < mobile::MAX_WIDTH {
                    self.mobile_view()
                } else {
                    self.desktop_view()
                }
            })
            .into()
        } else {
            self.desktop_view()
        };

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

        let content = match &self.update_dialog {
            Some(dialog) => modal::over_blocking(
                content,
                update_dialog::view(dialog).map(Message::Update),
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

        #[cfg(client)]
        let content = match &self.file_picker {
            Some(picker) => modal::over_blocking(
                content,
                file_list::view(picker).map(Message::FilePicker),
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

impl State {
    /// The remote's screen before (and between) connections: the menu, the
    /// list of hosts, and the questions about the connection on top.
    #[cfg(client)]
    fn connect_view<'a>(&'a self, header: Element<'a, Message>) -> Element<'a, Message> {
        let content: Element<'a, Message> = column![
            header,
            divider::horizontal(),
            connect::view(&self.connect).map(Message::Connect),
        ]
        .into();

        let content = match &self.prompt_for_connect() {
            Some(prompt) => modal::over_blocking(
                content,
                connect::prompt_view(prompt).map(Message::Connect),
            ),
            None => content,
        };
        let content = if self.connect.connecting {
            modal::over_blocking(content, connect::connecting_view())
        } else {
            content
        };
        let content = match &self.credits {
            Some(dialog) => modal::over_blocking(
                content,
                credits::view(dialog).map(Message::Credits),
            ),
            None => content,
        };
        let content = match &self.update_dialog {
            Some(dialog) => modal::over_blocking(
                content,
                update_dialog::view(dialog).map(Message::Update),
            ),
            None => content,
        };
        self.toasts.overlay(content, Message::Toast)
    }

    /// Whether this is a remote that has no host to work with yet. (Not
    /// any build with the `remote` feature: the tests of the main screen
    /// run in one, with `target` "test".)
    #[cfg(client)]
    fn needs_connection(&self) -> bool {
        matches!(self.target, "remote" | "web") && self.backend.is_none()
    }

    #[cfg(client)]
    fn prompt_for_connect(&self) -> Option<&connect::Prompt> {
        self.connect.prompt.as_ref()
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
