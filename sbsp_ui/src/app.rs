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
use iced::{Element, Length, Subscription, Task};

use crate::menu::{MenuNode, MenuSpec};
use crate::state::{assets::AssetResults, model::ShowModelState, playback::PlaybackState, ui::UiState};
use crate::theme::ThemeMode;
use crate::widgets::{menu_bar, toast};

#[cfg(feature = "host")]
use crate::host::backend::HostPort;
#[cfg(feature = "host")]
use crate::port::{FullState, PortEvent};

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
    target: &'static str,
    theme_mode: ThemeMode,
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
    Menu(crate::menu::MenuId),
    Toast(toast::Message),
    Tick(Instant),
    #[cfg(feature = "host")]
    BackendStarted(HostPort, FullState),
    #[cfg(feature = "host")]
    PortEvent(PortEvent),
    #[cfg(feature = "host")]
    BackendError(String),
}

impl State {
    fn new(target: &'static str) -> (Self, Task<Message>) {
        #[cfg(feature = "host")]
        let boot_task = Task::perform(HostPort::start(), |result| match result {
            Ok((backend, state)) => Message::BackendStarted(backend, state),
            Err(e) => Message::BackendError(e.to_string()),
        });
        #[cfg(not(feature = "host"))]
        let boot_task = Task::none();

        let is_host = cfg!(feature = "host");

        let state = Self {
            target,
            theme_mode: ThemeMode::default(),
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

    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleTheme => {
                self.theme_mode = match self.theme_mode {
                    ThemeMode::System => ThemeMode::Dark,
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::System,
                };
            }
            Message::Menu(id) => self.handle_menu(id),
            Message::Toast(message) => self.toasts.update(message),
            Message::Tick(now) => {
                let _ = self.playback.calculate_position(true, now);
            }
            #[cfg(feature = "host")]
            Message::BackendStarted(backend, state) => {
                log::info!("backend started, {} active cue(s) at connect", state.show_state.active_cues.len());
                self.playback.update(&state.show_state, Instant::now());
                self.model = state.show_model;
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

    fn handle_menu(&mut self, id: crate::menu::MenuId) {
        match id.0 {
            "view.toggle-theme" => self.update(Message::ToggleTheme),
            "help.about" => self.toasts.push(toast::Kind::Info, fl!("menu-help-about-message")),
            // Closing the window cleanly needs update() to return a Task,
            // which nothing else here needs yet; a hard exit is a
            // placeholder until Phase 7's window-state work gives this a
            // reason to change.
            "file.quit" => std::process::exit(0),
            other => log::warn!("unhandled menu id: {other}"),
        }
    }

    #[cfg(feature = "host")]
    fn handle_port_event(&mut self, event: PortEvent) {
        use sbsp_backend::event::BackendEvent;

        match event {
            BackendEvent::CueStatus(param) => self.playback.handle_cue_state_event(&param, Instant::now()),
            BackendEvent::SyncState(data) => self.playback.handle_sync_event(&data, Instant::now()),
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

        Subscription::batch([toasts, tick, backend_events])
    }

    fn menu_spec(&self) -> MenuSpec {
        MenuSpec::new()
            .menu(
                fl!("menu-file"),
                vec![MenuNode::item("file.quit", fl!("menu-file-quit"))],
            )
            .menu(
                fl!("menu-view"),
                vec![MenuNode::item("view.toggle-theme", fl!("menu-view-toggle-theme"))],
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

        let main = container(text(fl!("shell-main-placeholder", target = self.target)))
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
