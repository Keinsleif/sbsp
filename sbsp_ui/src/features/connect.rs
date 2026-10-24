// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The remote's connect screen: the hosts found on the network, the address
//! typed in by hand, and the password asked for when connecting. Ported
//! from `ConnectView.vue`.
//!
//! Choosing a host (or pressing Connect) asks for the password first; an
//! empty one means the host has none for what this connection may do.

use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use sbsp_backend::api::client::ServiceEntry;

use crate::fl;

pub const DEFAULT_PORT: u16 = 5800;
/// Width of the password dialog.
const PROMPT_WIDTH: f32 = 420.0;

#[derive(Debug, Clone)]
pub enum Message {
    Host(String),
    Port(String),
    /// A row of the list was clicked: its address goes into the fields.
    Pick(String, u16),
    /// A row's Connect button.
    ConnectTo(String, u16),
    /// The Connect button next to the fields.
    ConnectTyped,
    Password(String),
    Submit,
    Cancel,
}

/// The password question.
#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    pub address: String,
    pub password: String,
}

/// What the screen asks the app to do.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub address: String,
    pub password: Option<String>,
}

pub struct Screen {
    pub host: String,
    pub port: String,
    pub services: Vec<ServiceEntry>,
    pub prompt: Option<Prompt>,
    /// A connection is being made; nothing else can start one.
    pub connecting: bool,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: DEFAULT_PORT.to_string(),
            services: Vec::new(),
            prompt: None,
            connecting: false,
        }
    }
}

impl Screen {
    /// The address typed in, if it can be connected to.
    fn typed_address(&self) -> Option<String> {
        let host = self.host.trim();
        let port: u16 = self.port.parse().ok()?;
        (!host.is_empty()).then(|| format!("{host}:{port}"))
    }

    pub fn can_connect(&self) -> bool {
        !self.connecting && self.typed_address().is_some()
    }

    /// Applies a message; returns the connection to make, once the
    /// password has been given.
    pub fn update(&mut self, message: Message) -> Option<Request> {
        match message {
            Message::Host(host) => self.host = host,
            // Digits only, as many as a port has.
            Message::Port(port) => {
                if port.len() <= 5 && port.chars().all(|c| c.is_ascii_digit()) {
                    self.port = port;
                }
            }
            Message::Pick(host, port) => {
                self.host = host;
                self.port = port.to_string();
            }
            Message::ConnectTo(host, port) => {
                self.host = host;
                self.port = port.to_string();
                self.ask_password();
            }
            Message::ConnectTyped => self.ask_password(),
            Message::Password(password) => {
                if let Some(prompt) = &mut self.prompt {
                    prompt.password = password;
                }
            }
            Message::Submit => {
                let prompt = self.prompt.take()?;
                let password = prompt.password.trim();
                self.connecting = true;
                return Some(Request {
                    address: prompt.address,
                    password: (!password.is_empty()).then(|| password.to_owned()),
                });
            }
            Message::Cancel => self.prompt = None,
        }
        None
    }

    fn ask_password(&mut self) {
        if self.connecting {
            return;
        }
        if let Some(address) = self.typed_address() {
            self.prompt = Some(Prompt {
                address,
                password: String::new(),
            });
        }
    }
}

/// The screen itself: the list of hosts, and the fields below it.
pub fn view(screen: &Screen) -> Element<'_, Message> {
    let header = row![
        container(text(fl!("view-connect-remote-name")).size(13)).width(Length::FillPortion(3)),
        container(text(fl!("view-connect-remote-host")).size(13)).width(Length::FillPortion(3)),
        container(text(fl!("view-connect-remote-port")).size(13)).width(80),
        Space::new().width(100),
    ]
    .spacing(8)
    .padding([6, 8]);

    let rows = column(screen.services.iter().map(|entry| {
        let connect = button(text(fl!("view-connect-connect")))
            .style(button::secondary)
            .padding([2, 10])
            .on_press_maybe(
                (!screen.connecting)
                    .then(|| Message::ConnectTo(entry.host.clone(), entry.port)),
            );
        mouse_area(
            row![
                container(text(entry.server_name.clone())).width(Length::FillPortion(3)),
                container(text(entry.host.clone())).width(Length::FillPortion(3)),
                container(text(entry.port.to_string())).width(80),
                connect,
            ]
            .spacing(8)
            .padding([4, 8])
            .align_y(Alignment::Center),
        )
        .on_press(Message::Pick(entry.host.clone(), entry.port))
        .into()
    }));

    let fields = row![
        text_input(&fl!("view-connect-remote-host"), &screen.host)
            .on_input(Message::Host)
            .on_submit(Message::ConnectTyped)
            .width(Length::FillPortion(3)),
        text_input(&fl!("view-connect-remote-port"), &screen.port)
            .on_input(Message::Port)
            .on_submit(Message::ConnectTyped)
            .width(100),
        button(text(fl!("view-connect-connect")))
            .style(button::primary)
            .on_press_maybe(screen.can_connect().then_some(Message::ConnectTyped)),
    ]
    .spacing(8)
    .padding(8)
    .align_y(Alignment::Center);

    column![
        header,
        scrollable(rows).height(Length::Fill),
        fields,
    ]
    .into()
}

/// The password question, as a dialog.
pub fn prompt_view(prompt: &Prompt) -> Element<'_, Message> {
    column![
        text(fl!("view-connect-password-prompt")).size(18),
        text(prompt.address.clone()).size(13),
        text_input("", &prompt.password)
            .secure(true)
            .on_input(Message::Password)
            .on_submit(Message::Submit),
        row![
            Space::new().width(Length::Fill),
            button(text(fl!("general-cancel")))
                .style(button::secondary)
                .on_press(Message::Cancel),
            button(text(fl!("view-connect-connect")))
                .style(button::primary)
                .on_press(Message::Submit),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .width(PROMPT_WIDTH)
    .into()
}

/// What is shown while the connection is being made.
pub fn connecting_view<'a, Message: 'a>() -> Element<'a, Message> {
    container(text(fl!("view-connect-connecting")).size(18))
        .padding([8, 24])
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(host: &str, port: &str) -> Screen {
        Screen {
            host: host.to_owned(),
            port: port.to_owned(),
            ..Screen::default()
        }
    }

    #[test]
    fn the_port_takes_up_to_five_digits_only() {
        let mut screen = Screen::default();
        screen.update(Message::Port("58x".to_owned()));
        assert_eq!(screen.port, "5800");
        screen.update(Message::Port("123456".to_owned()));
        assert_eq!(screen.port, "5800");
        screen.update(Message::Port("6000".to_owned()));
        assert_eq!(screen.port, "6000");
        screen.update(Message::Port(String::new()));
        assert!(!typed("host", &screen.port).can_connect(), "an empty port");
    }

    #[test]
    fn connecting_needs_a_host_and_a_valid_port() {
        assert!(typed("example.local", "5800").can_connect());
        assert!(!typed("", "5800").can_connect());
        assert!(!typed("  ", "5800").can_connect());
        assert!(!typed("example.local", "70000").can_connect());
    }

    #[test]
    fn the_password_is_asked_before_connecting() {
        let mut screen = typed("example.local", "5800");
        assert_eq!(screen.update(Message::ConnectTyped), None);
        assert_eq!(
            screen.prompt,
            Some(Prompt {
                address: "example.local:5800".to_owned(),
                password: String::new(),
            })
        );

        screen.update(Message::Password(" secret ".to_owned()));
        let request = screen.update(Message::Submit).unwrap();
        assert_eq!(request.address, "example.local:5800");
        assert_eq!(request.password.as_deref(), Some("secret"));
        assert!(screen.prompt.is_none());
        assert!(screen.connecting);
    }

    #[test]
    fn an_empty_password_means_none_and_cancel_asks_nothing() {
        let mut screen = Screen::default();
        screen.update(Message::ConnectTo("host".to_owned(), 6000));
        assert_eq!((screen.host.as_str(), screen.port.as_str()), ("host", "6000"));
        let request = screen.update(Message::Submit).unwrap();
        assert_eq!(request.password, None);

        let mut screen = typed("host", "5800");
        screen.update(Message::ConnectTyped);
        assert_eq!(screen.update(Message::Cancel), None);
        assert!(screen.prompt.is_none() && !screen.connecting);
    }

    #[test]
    fn nothing_starts_while_a_connection_is_being_made() {
        let mut screen = typed("host", "5800");
        screen.connecting = true;
        screen.update(Message::ConnectTyped);
        screen.update(Message::ConnectTo("other".to_owned(), 1));
        assert!(screen.prompt.is_none());
        assert!(!screen.can_connect());
    }

    #[test]
    fn picking_a_row_fills_the_fields_without_connecting() {
        let mut screen = Screen::default();
        assert_eq!(screen.update(Message::Pick("a.local".to_owned(), 5801)), None);
        assert_eq!((screen.host.as_str(), screen.port.as_str()), ("a.local", "5801"));
        assert!(screen.prompt.is_none());
    }
}
