// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The server panel: start and stop the API server, its port, discovery
//! name and the passwords with the permissions each one gives. Ported from
//! `ServerPanelDialog.vue` and `ServerPanelPasswordRow.vue`.
//!
//! The panel edits a copy of the server options; starting the server hands
//! them to the backend. While the server runs they are locked.

use iced::widget::{
    Space, button, checkbox, column, container, qr_code, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length, Theme};
use sbsp_backend::api::{ApiServerOptions, PermissionInfo, Permissions};
use sbsp_backend::model::cue::Uuid;

use crate::fl;
use crate::widgets::icon::{self, Icon, Tone};

const DEFAULT_PORT: u16 = 5800;
const DEFAULT_NAME: &str = "Untitled SBS Player Server";
const PASSWORD_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const PASSWORD_LENGTH: usize = 16;

#[derive(Debug, Clone)]
pub enum Message {
    Port(String),
    Discoverable(bool),
    Name(String),
    Password(usize, String),
    Permission(usize, Permissions, bool),
    Generate(usize),
    Delete(usize),
    Add,
    ToggleVisible,
    ShowInfo(usize),
    CloseInfo,
    /// Put this text on the clipboard.
    Copy(String),
    /// Start or stop the server.
    Toggle,
    Close,
}

/// A password row.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub password: String,
    pub permission: Permissions,
}

/// What the connect information shows.
pub struct Info {
    pub hostname: Option<String>,
    pub port: u16,
    pub password: String,
    pub url: Option<String>,
    pub qr: Option<qr_code::Data>,
}

pub struct Panel {
    pub running: bool,
    port: String,
    discoverable: bool,
    name: String,
    rows: Vec<Row>,
    password_visible: bool,
    pub info: Option<Info>,
    /// Copied text the button should say so for (the last one).
    copied: Option<String>,
}

impl Panel {
    pub fn new(options: &ApiServerOptions, running: bool) -> Self {
        Self {
            running,
            port: options.port.to_string(),
            discoverable: options.discoverry.is_some(),
            name: options
                .discoverry
                .clone()
                .unwrap_or_else(|| DEFAULT_NAME.to_owned()),
            rows: options
                .auth_map
                .iter()
                .map(|info| Row {
                    password: info.password.clone(),
                    permission: info.permission,
                })
                .collect(),
            password_visible: false,
            info: None,
            copied: None,
        }
    }

    /// The options as edited; `None` while the port is not a port number.
    pub fn options(&self) -> Option<ApiServerOptions> {
        Some(ApiServerOptions {
            port: self.port.trim().parse().ok()?,
            discoverry: self.discoverable.then(|| self.name.clone()),
            auth_map: self
                .rows
                .iter()
                .map(|row| PermissionInfo {
                    password: row.password.clone(),
                    permission: row.permission,
                })
                .collect(),
            trust_ip_headers: false,
        })
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Everything but the buttons that need the app (copy, start/stop,
    /// connect information, close).
    pub fn update(&mut self, message: &Message) {
        // Nothing but looking is allowed while the server runs.
        let editable = !self.running;
        match message {
            Message::Port(port) if editable => {
                // Digits only, at most five of them.
                if port.chars().all(|c| c.is_ascii_digit()) && port.len() <= 5 {
                    self.port.clone_from(port);
                }
            }
            Message::Discoverable(on) if editable => self.discoverable = *on,
            Message::Name(name) if editable => self.name.clone_from(name),
            Message::Password(i, password) if editable => {
                if let Some(row) = self.rows.get_mut(*i) {
                    row.password.clone_from(password);
                }
            }
            Message::Permission(i, flag, on) if editable => {
                if let Some(row) = self.rows.get_mut(*i) {
                    row.permission.set(*flag, *on);
                }
            }
            Message::Generate(i) if editable => {
                if let Some(row) = self.rows.get_mut(*i) {
                    row.password = generate_password();
                }
            }
            Message::Delete(i) if editable && *i < self.rows.len() => {
                self.rows.remove(*i);
            }
            Message::Add if editable => self.rows.push(Row {
                password: generate_password(),
                permission: Permissions::READ,
            }),
            Message::ToggleVisible => self.password_visible = !self.password_visible,
            Message::CloseInfo => self.info = None,
            Message::Copy(text) => self.copied = Some(text.clone()),
            _ => {}
        }
    }

    /// Opens the connect information of one password.
    pub fn show_info(&mut self, index: usize, hostname: Option<String>) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        let port = self.options().map_or(DEFAULT_PORT, |options| options.port);
        let url = hostname
            .as_deref()
            .map(|hostname| connect_url(hostname, port, &row.password));
        self.info = Some(Info {
            hostname,
            port,
            password: row.password.clone(),
            qr: url.as_deref().and_then(|url| qr_code::Data::new(url).ok()),
            url,
        });
    }
}

/// The address a browser opens to control this server with a password.
pub fn connect_url(hostname: &str, port: u16, password: &str) -> String {
    let address = format!("{hostname}:{port}");
    format!("http://{address}/?address={address}#{password}")
}

/// A password made from the random bytes given (bytes that would favour
/// some characters are skipped); `None` if they run out.
pub fn password_from(bytes: impl IntoIterator<Item = u8>) -> Option<String> {
    let limit = u8::try_from(256 - 256 % PASSWORD_CHARS.len()).ok()?;
    let password: String = bytes
        .into_iter()
        .filter(|byte| *byte < limit)
        .take(PASSWORD_LENGTH)
        .map(|byte| char::from(PASSWORD_CHARS[usize::from(byte) % PASSWORD_CHARS.len()]))
        .collect();
    (password.len() == PASSWORD_LENGTH).then_some(password)
}

pub fn generate_password() -> String {
    loop {
        // A version 4 UUID is random except for the two bytes that say so.
        let bytes = Uuid::new_v4()
            .into_bytes()
            .into_iter()
            .enumerate()
            .filter(|(i, _)| *i != 6 && *i != 8)
            .map(|(_, byte)| byte);
        let more = Uuid::new_v4()
            .into_bytes()
            .into_iter()
            .enumerate()
            .filter(|(i, _)| *i != 6 && *i != 8)
            .map(|(_, byte)| byte);
        if let Some(password) = password_from(bytes.chain(more)) {
            return password;
        }
    }
}

fn status_color(theme: &Theme, running: bool) -> iced::Color {
    let palette = theme.extended_palette();
    if running {
        palette.success.base.color
    } else {
        palette.danger.base.color
    }
}

fn row_view<'a>(panel: &Panel, index: usize, row: &Row) -> Element<'a, Message> {
    let editable = !panel.running;

    let mut password = text_input(&fl!("dialog-server-info-password-not-set"), &row.password)
        .secure(!panel.password_visible)
        .width(200);
    if editable {
        password = password.on_input(move |value| Message::Password(index, value));
    }

    let flag = |label: String, flag: Permissions| {
        let mut input = checkbox(row.permission.contains(flag)).label(label);
        if editable {
            input = input.on_toggle(move |on| Message::Permission(index, flag, on));
        }
        input
    };

    let generate = button(text(fl!("dialog-server-generate")))
        .on_press_maybe(editable.then_some(Message::Generate(index)));
    let info = button(icon::toned(Icon::OpenInNew, 16.0, Tone::Text))
        .style(button::secondary)
        .on_press(Message::ShowInfo(index));
    let delete = button(icon::toned(Icon::TrashCan, 16.0, Tone::Text))
        .style(button::danger)
        .on_press_maybe(editable.then_some(Message::Delete(index)));

    row![
        password,
        flag(fl!("dialog-server-permissions-read"), Permissions::READ),
        flag(fl!("dialog-server-permissions-control"), Permissions::CONTROL),
        flag(fl!("dialog-server-permissions-edit"), Permissions::EDIT),
        generate,
        info,
        delete,
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn copy_field<'a>(label: String, value: &str, placeholder: String) -> Element<'a, Message> {
    let shown = text_input(&placeholder, value);
    let copy = button(icon::toned(Icon::ContentCopy, 16.0, Tone::Text))
        .style(button::secondary)
        .on_press_maybe((!value.is_empty()).then(|| Message::Copy(value.to_owned())));
    column![
        text(label).size(12),
        row![shown, copy].spacing(6).align_y(Alignment::Center)
    ]
    .spacing(2)
    .into()
}

fn info_view<'a>(info: &'a Info) -> Element<'a, Message> {
    let host = copy_field(
        fl!("dialog-server-info-host"),
        info.hostname.as_deref().unwrap_or(""),
        fl!("dialog-server-info-hostname-unavailable"),
    );
    let port = copy_field(fl!("dialog-server-port"), &info.port.to_string(), String::new());
    let qr: Element<'a, Message> = match &info.qr {
        Some(data) => qr_code(data).cell_size(6).into(),
        None => Space::new().into(),
    };

    column![
        text(fl!("dialog-server-connect-info")).size(18),
        row![container(host).width(Length::Fill), text(":"), container(port).width(110)]
            .spacing(8)
            .align_y(Alignment::End),
        copy_field(
            fl!("dialog-server-password"),
            &info.password,
            fl!("dialog-server-info-password-not-set"),
        ),
        copy_field(
            fl!("dialog-server-info-url"),
            info.url.as_deref().unwrap_or(""),
            fl!("dialog-server-info-url-unavailable"),
        ),
        container(qr).center_x(Length::Fill),
        row![
            Space::new().width(Length::Fill),
            button(text(fl!("general-close")))
                .style(button::secondary)
                .on_press(Message::CloseInfo),
        ],
    ]
    .spacing(10)
    .width(480)
    .into()
}

/// The connect information (when open) replaces the panel, as a second
/// dialog on top of it would need a second modal layer.
pub fn view<'a>(panel: &'a Panel) -> Element<'a, Message> {
    if let Some(info) = &panel.info {
        return info_view(info);
    }

    let running = panel.running;
    let status = row![
        text(format!("{} :", fl!("dialog-server-status"))),
        text(if running {
            fl!("dialog-server-running")
        } else {
            fl!("dialog-server-stopped")
        })
        .style(move |theme: &Theme| text::Style {
            color: Some(status_color(theme, running)),
        }),
    ]
    .spacing(6);

    let mut port = text_input("5800", &panel.port).width(100);
    let mut discoverable = checkbox(panel.discoverable).label(fl!("dialog-server-discoverable"));
    let mut name = text_input("", &panel.name).width(260);
    if !running {
        port = port.on_input(Message::Port);
        discoverable = discoverable.on_toggle(Message::Discoverable);
        if panel.discoverable {
            name = name.on_input(Message::Name);
        }
    }

    let options = column![
        status,
        column![text(fl!("dialog-server-port")).size(12), port].spacing(2),
        discoverable,
        column![text(fl!("dialog-server-server-name")).size(12), name].spacing(2),
    ]
    .spacing(14);

    let eye = button(icon::toned(
        if panel.password_visible { Icon::Eye } else { Icon::EyeOff },
        16.0,
        Tone::Muted,
    ))
    .padding(2)
    .style(button::text)
    .on_press(Message::ToggleVisible);

    let mut rows = column![
        row![text(fl!("dialog-server-password")).size(14), eye]
            .spacing(6)
            .align_y(Alignment::Center),
    ]
    .spacing(8);
    for (i, row) in panel.rows.iter().enumerate() {
        rows = rows.push(row_view(panel, i, row));
    }
    let add = button(icon::toned(Icon::Plus, 16.0, Tone::Text))
        .style(button::success)
        .on_press_maybe((!running).then_some(Message::Add));
    rows = rows.push(add);

    let start = button(text(if running {
        fl!("dialog-server-stop")
    } else {
        fl!("dialog-server-start")
    }))
    .style(if running { button::danger } else { button::success })
    .on_press(Message::Toggle);

    column![
        text(fl!("dialog-server-title")).size(18),
        row![
            options,
            container(scrollable(rows))
                .padding(8)
                .width(680)
                .height(260)
                .style(container::bordered_box),
        ]
        .spacing(16),
        row![
            button(text(fl!("general-close")))
                .style(button::secondary)
                .on_press(Message::Close),
            Space::new().width(Length::Fill),
            start,
        ],
    ].width(Length::Shrink)
    .spacing(14)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> ApiServerOptions {
        ApiServerOptions {
            port: 5800,
            discoverry: Some("Stage".to_owned()),
            auth_map: vec![PermissionInfo {
                password: "secret".to_owned(),
                permission: Permissions::READ | Permissions::CONTROL,
            }],
            trust_ip_headers: false,
        }
    }

    #[test]
    fn the_panel_starts_from_the_options_and_gives_them_back() {
        let panel = Panel::new(&options(), false);
        let back = panel.options().unwrap();
        assert_eq!(back.port, 5800);
        assert_eq!(back.discoverry.as_deref(), Some("Stage"));
        assert_eq!(back.auth_map[0].password, "secret");
        assert_eq!(back.auth_map[0].permission, Permissions::READ | Permissions::CONTROL);
    }

    #[test]
    fn discovery_off_means_no_name_but_the_name_is_kept() {
        let mut panel = Panel::new(&options(), false);
        panel.update(&Message::Discoverable(false));
        assert_eq!(panel.options().unwrap().discoverry, None);
        panel.update(&Message::Discoverable(true));
        assert_eq!(panel.options().unwrap().discoverry.as_deref(), Some("Stage"));

        let none = ApiServerOptions {
            discoverry: None,
            ..options()
        };
        let panel = Panel::new(&none, false);
        assert_eq!(panel.name, DEFAULT_NAME);
    }

    #[test]
    fn only_port_numbers_are_taken() {
        let mut panel = Panel::new(&options(), false);
        panel.update(&Message::Port("58x".to_owned()));
        assert_eq!(panel.port, "5800", "letters are refused");
        panel.update(&Message::Port("123456".to_owned()));
        assert_eq!(panel.port, "5800", "six digits are refused");
        panel.update(&Message::Port("70000".to_owned()));
        assert_eq!(panel.options().map(|o| o.port), None, "too big for a port");
        panel.update(&Message::Port(String::new()));
        assert_eq!(panel.options().map(|o| o.port), None);
        panel.update(&Message::Port("8080".to_owned()));
        assert_eq!(panel.options().map(|o| o.port), Some(8080));
    }

    #[test]
    fn rows_are_added_changed_and_deleted() {
        let mut panel = Panel::new(&options(), false);
        panel.update(&Message::Add);
        assert_eq!(panel.rows().len(), 2);
        assert_eq!(panel.rows()[1].permission, Permissions::READ);
        assert_eq!(panel.rows()[1].password.len(), PASSWORD_LENGTH);

        panel.update(&Message::Permission(1, Permissions::EDIT, true));
        assert_eq!(panel.rows()[1].permission, Permissions::READ | Permissions::EDIT);
        panel.update(&Message::Permission(1, Permissions::READ, false));
        assert_eq!(panel.rows()[1].permission, Permissions::EDIT);

        panel.update(&Message::Password(0, "new".to_owned()));
        assert_eq!(panel.rows()[0].password, "new");
        let before = panel.rows()[0].password.clone();
        panel.update(&Message::Generate(0));
        assert_ne!(panel.rows()[0].password, before);

        panel.update(&Message::Delete(0));
        assert_eq!(panel.rows().len(), 1);
        panel.update(&Message::Delete(5));
        assert_eq!(panel.rows().len(), 1, "no such row");
    }

    #[test]
    fn nothing_is_edited_while_the_server_runs() {
        let mut panel = Panel::new(&options(), true);
        let before = panel.options().unwrap();
        panel.update(&Message::Port("1".to_owned()));
        panel.update(&Message::Add);
        panel.update(&Message::Delete(0));
        panel.update(&Message::Password(0, "x".to_owned()));
        panel.update(&Message::Discoverable(false));
        let after = panel.options().unwrap();
        assert_eq!(after.port, before.port);
        assert_eq!(after.auth_map.len(), 1);
        assert_eq!(after.auth_map[0].password, "secret");
        assert!(after.discoverry.is_some());
    }

    #[test]
    fn the_password_comes_only_from_the_character_set() {
        let password = password_from((0..=255u8).cycle()).unwrap();
        assert_eq!(password.len(), PASSWORD_LENGTH);
        assert!(password.bytes().all(|b| PASSWORD_CHARS.contains(&b)));
        // Bytes at or above 248 would favour the first characters.
        assert_eq!(password_from([255u8; 40]), None);
        assert_eq!(password_from([1u8; 3]), None);
        assert_eq!(password_from([0u8; 16]).as_deref(), Some("AAAAAAAAAAAAAAAA"));
        assert_ne!(generate_password(), generate_password());
    }

    #[test]
    fn the_connect_address_carries_the_password() {
        assert_eq!(
            connect_url("host.local.", 5800, "pw"),
            "http://host.local.:5800/?address=host.local.:5800#pw"
        );
        assert_eq!(
            connect_url("h", 80, ""),
            "http://h:80/?address=h:80#",
            "an empty password leaves the fragment empty"
        );

        let mut panel = Panel::new(&options(), false);
        panel.show_info(0, Some("host.local.".to_owned()));
        let info = panel.info.as_ref().unwrap();
        assert_eq!(info.password, "secret");
        assert_eq!(
            info.url.as_deref(),
            Some("http://host.local.:5800/?address=host.local.:5800#secret")
        );
        assert!(info.qr.is_some());
        panel.update(&Message::CloseInfo);
        assert!(panel.info.is_none());

        panel.show_info(0, None);
        let info = panel.info.as_ref().unwrap();
        assert_eq!(info.url, None);
        assert!(info.qr.is_none());
    }
}
