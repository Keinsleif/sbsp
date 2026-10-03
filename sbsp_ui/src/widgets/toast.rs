// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A stack of auto-dismissing toast notifications, replacing PrimeVue's
//! `Toast` (see the migration plan, section 6).
//!
//! [`Toasts`] owns its own state (the current list of notifications) and is
//! meant to be embedded as a field of a larger `State`, with its `Message`
//! wrapped the same way as any other nested TEA module (see the earlier
//! discussion on global/shared state: edit mode, settings, etc. all follow
//! this "own a field, thread it through as an argument" shape).
//!
//! ```ignore
//! struct State {
//!     toasts: widgets::toast::Toasts,
//!     // ...
//! }
//!
//! enum Message {
//!     Toast(widgets::toast::Message),
//!     // ...
//! }
//!
//! // in `view`:
//! toasts.overlay(main_content, Message::Toast)
//!
//! // in `subscription`:
//! toasts.subscription().map(Message::Toast)
//! ```

use std::time::{Duration, Instant};

use iced::widget::{button, column, container, row, stack, text};
use iced::{Alignment, Element, Length, Subscription, Theme};

const DEFAULT_LIFETIME: Duration = Duration::from_secs(4);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Info,
    Success,
    Warning,
    Danger,
}

#[derive(Debug)]
struct Toast {
    id: u64,
    kind: Kind,
    message: String,
    expires_at: Instant,
}

#[derive(Debug, Default)]
pub struct Toasts {
    items: Vec<Toast>,
    next_id: u64,
}

#[derive(Debug, Clone)]
pub enum Message {
    Dismiss(u64),
    Tick(Instant),
}

impl Toasts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues a new toast with the default lifetime.
    pub fn push(&mut self, kind: Kind, message: impl Into<String>) {
        self.push_for(kind, message, DEFAULT_LIFETIME);
    }

    /// Queues a new toast with an explicit lifetime (e.g. a longer one for
    /// an error the user is more likely to need to read in full).
    pub fn push_for(&mut self, kind: Kind, message: impl Into<String>, lifetime: Duration) {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Toast {
            id,
            kind,
            message: message.into(),
            expires_at: Instant::now() + lifetime,
        });
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Dismiss(id) => self.items.retain(|toast| toast.id != id),
            Message::Tick(now) => self.items.retain(|toast| toast.expires_at > now),
        }
    }

    /// Only polls while there is something to expire, so an idle app with
    /// no active toasts costs nothing.
    pub fn subscription(&self) -> Subscription<Message> {
        if self.items.is_empty() {
            Subscription::none()
        } else {
            iced::time::every(POLL_INTERVAL).map(Message::Tick)
        }
    }

    /// Wraps `base` with the toast stack rendered on top, anchored to the
    /// bottom-right corner. Returns `base` unchanged (no extra stack layer)
    /// when there is nothing to show.
    pub fn overlay<'a, AppMessage>(
        &self,
        base: Element<'a, AppMessage>,
        on_message: impl Fn(Message) -> AppMessage + Clone + 'a,
    ) -> Element<'a, AppMessage>
    where
        AppMessage: Clone + 'a,
    {
        if self.items.is_empty() {
            return base;
        }

        let list = column(
            self.items
                .iter()
                .map(|toast| toast_card(toast, on_message.clone())),
        )
        .spacing(8)
        .width(320);

        let positioned = container(list)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::End)
            .padding(16);

        stack![base, positioned].into()
    }
}

fn toast_card<'a, AppMessage>(
    toast: &Toast,
    on_message: impl Fn(Message) -> AppMessage + 'a,
) -> Element<'a, AppMessage>
where
    AppMessage: Clone + 'a,
{
    let id = toast.id;
    let kind = toast.kind;

    let content = row![
        text(toast.message.clone()).width(Length::Fill),
        button(text("×"))
            .padding(4)
            .style(button::text)
            .on_press(on_message(Message::Dismiss(id))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(content)
        .padding(12)
        .width(Length::Fill)
        .style(move |theme: &Theme| card_style(theme, kind))
        .into()
}

fn card_style(theme: &Theme, kind: Kind) -> container::Style {
    let palette = theme.extended_palette();
    let accent = match kind {
        Kind::Info => palette.primary.base,
        Kind::Success => palette.success.base,
        Kind::Warning => palette.warning.base,
        Kind::Danger => palette.danger.base,
    };

    container::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.base.text),
        border: iced::Border {
            color: accent.color,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}
