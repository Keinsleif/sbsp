// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Phase 0 prototype. Everything here is disposable; it exists to verify the
//! items listed in the migration plan on real machines.

use std::collections::BTreeSet;

use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_editor, text_input,
};
use iced::time::{Duration, Instant};
use iced::{Element, Length, Subscription, Theme, event, keyboard, window};
use unic_langid::langid;

use crate::fl;

const ROW_COUNTS: [usize; 3] = [100, 500, 2000];

pub fn run(target: &'static str) -> iced::Result {
    iced::application(move || App::new(target), App::update, App::view)
        .title(|_: &App| fl!("proto-title"))
        .theme(|app: &App| if app.dark { Theme::Dark } else { Theme::Light })
        .subscription(App::subscription)
        .window_size((1100.0, 780.0))
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lang {
    En,
    Ja,
}

#[derive(Debug, Clone)]
struct CueRow {
    number: usize,
    indent: usize,
    is_group: bool,
}

struct App {
    target: &'static str,
    lang: Lang,
    dark: bool,
    single_line: String,
    editor: text_editor::Content,
    rows: Vec<CueRow>,
    selected: BTreeSet<usize>,
    anchor: usize,
    modifiers: keyboard::Modifiers,
    animate: bool,
    progress: f32,
    frames: FrameCounter,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleLanguage,
    ToggleTheme,
    ToggleAnimation,
    SingleLineChanged(String),
    Editor(text_editor::Action),
    SetRowCount(usize),
    Select(usize),
    KeyPressed(keyboard::Key),
    ModifiersChanged(keyboard::Modifiers),
    Frame(Instant),
}

impl App {
    fn new(target: &'static str) -> Self {
        crate::i18n::select(&langid!("en"));
        Self {
            target,
            lang: Lang::En,
            dark: true,
            single_line: String::new(),
            editor: text_editor::Content::new(),
            rows: build_rows(500),
            selected: BTreeSet::new(),
            anchor: 0,
            modifiers: keyboard::Modifiers::default(),
            animate: false,
            progress: 0.0,
            frames: FrameCounter::default(),
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleLanguage => {
                self.lang = match self.lang {
                    Lang::En => Lang::Ja,
                    Lang::Ja => Lang::En,
                };
                match self.lang {
                    Lang::En => crate::i18n::select(&langid!("en")),
                    Lang::Ja => crate::i18n::select(&langid!("ja")),
                }
            }
            Message::ToggleTheme => self.dark = !self.dark,
            Message::ToggleAnimation => {
                self.animate = !self.animate;
                self.frames = FrameCounter::default();
            }
            Message::SingleLineChanged(value) => self.single_line = value,
            Message::Editor(action) => self.editor.perform(action),
            Message::SetRowCount(count) => {
                self.rows = build_rows(count);
                self.selected.clear();
                self.anchor = 0;
            }
            Message::Select(index) => self.select(index),
            Message::KeyPressed(key) => {
                use keyboard::Key;
                use keyboard::key::Named;
                let current = self.selected.iter().next_back().copied().unwrap_or(0);
                match key {
                    Key::Named(Named::ArrowDown) => {
                        let next = (current + 1).min(self.rows.len().saturating_sub(1));
                        self.select(next);
                    }
                    Key::Named(Named::ArrowUp) => self.select(current.saturating_sub(1)),
                    _ => {}
                }
            }
            Message::ModifiersChanged(modifiers) => self.modifiers = modifiers,
            Message::Frame(now) => {
                self.frames.tick(now);
                self.progress = (self.progress + 0.004) % 1.0;
            }
        }
    }

    fn select(&mut self, index: usize) {
        if self.modifiers.shift() {
            let (from, to) = (self.anchor.min(index), self.anchor.max(index));
            self.selected = (from..=to).collect();
        } else if self.modifiers.command() {
            if !self.selected.remove(&index) {
                self.selected.insert(index);
            }
            self.anchor = index;
        } else {
            self.selected = BTreeSet::from([index]);
            self.anchor = index;
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let keys = event::listen_with(|event, status, _window| {
            // Ignore keys already consumed by a focused text widget.
            if status == event::Status::Captured {
                return None;
            }
            match event {
                iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    Some(Message::KeyPressed(key))
                }
                iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                    Some(Message::ModifiersChanged(modifiers))
                }
                _ => None,
            }
        });

        if self.animate {
            Subscription::batch([keys, window::frames().map(Message::Frame)])
        } else {
            keys
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let header = row![
            text(fl!("proto-title")).size(22),
            text(fl!("proto-target", target = self.target)),
            text(fl!(
                "proto-renderer-env",
                value = std::env::var("ICED_BACKEND").unwrap_or_else(|_| "(default)".into())
            )),
            button(text(fl!("proto-toggle-language"))).on_press(Message::ToggleLanguage),
            button(text(fl!("proto-toggle-theme"))).on_press(Message::ToggleTheme),
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center);

        let animation = row![
            button(text(fl!("proto-toggle-animation"))).on_press(Message::ToggleAnimation),
            text(fl!("proto-fps", fps = format!("{:.1}", self.frames.fps()))),
            container(
                container(text(""))
                    .width(Length::FillPortion((self.progress * 1000.0) as u16 + 1))
                    .height(8)
                    .style(|theme: &Theme| container::Style {
                        background: Some(theme.extended_palette().primary.strong.color.into()),
                        ..Default::default()
                    }),
            )
            .width(240)
            .height(8),
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center);

        let ime = column![
            text(fl!("proto-ime-heading")).size(18),
            text_input(&fl!("proto-ime-hint"), &self.single_line)
                .on_input(Message::SingleLineChanged)
                .padding(8),
            text_editor(&self.editor)
                .placeholder(fl!("proto-ime-multiline-hint"))
                .on_action(Message::Editor)
                .height(90),
        ]
        .spacing(8);

        let counts = row(ROW_COUNTS.iter().map(|&count| {
            Element::from(
                button(text(fl!("proto-count-button", count = count)))
                    .on_press(Message::SetRowCount(count)),
            )
        }))
        .spacing(8);

        let list_header = row![
            text(fl!("proto-list-heading", count = self.rows.len())).size(18),
            counts,
            text(fl!("proto-list-selected", count = self.selected.len())),
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center);

        let list = scrollable(
            column(
                self.rows
                    .iter()
                    .enumerate()
                    .map(|(index, cue)| self.cue_row(index, cue)),
            )
            .width(Length::Fill),
        )
        .height(Length::Fill);

        column![
            header,
            animation,
            ime,
            list_header,
            text(fl!("proto-list-hint")).size(12),
            list
        ]
        .spacing(12)
        .padding(16)
        .into()
    }

    fn cue_row<'a>(&self, index: usize, cue: &CueRow) -> Element<'a, Message> {
        let selected = self.selected.contains(&index);
        let name = if cue.is_group {
            fl!("proto-list-group-name", number = cue.number)
        } else {
            fl!("proto-list-cue-name", number = cue.number)
        };
        let marker = if cue.is_group { "▾" } else { "•" };

        let content = row![
            text(format!("{:>4}", index + 1)).width(48),
            iced::widget::Space::new().width((cue.indent * 20) as f32),
            text(marker).width(16),
            text(name),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        mouse_area(
            container(content)
                .padding([4, 8])
                .width(Length::Fill)
                .style(move |theme: &Theme| container::Style {
                    background: selected
                        .then(|| theme.extended_palette().primary.weak.color.into()),
                    text_color: selected.then(|| theme.extended_palette().primary.weak.text),
                    ..Default::default()
                }),
        )
        .on_press(Message::Select(index))
        .into()
    }
}

fn build_rows(count: usize) -> Vec<CueRow> {
    (0..count)
        .map(|i| {
            let is_group = i % 10 == 0;
            CueRow {
                number: i + 1,
                indent: if is_group { 0 } else { 1 + (i % 3 == 0) as usize },
                is_group,
            }
        })
        .collect()
}

/// Frame rate averaged over the last second.
#[derive(Default)]
struct FrameCounter {
    window_start: Option<Instant>,
    count: u32,
    fps: f32,
}

impl FrameCounter {
    fn tick(&mut self, now: Instant) {
        let start = *self.window_start.get_or_insert(now);
        self.count += 1;
        let elapsed = now.duration_since(start);
        if elapsed >= Duration::from_secs(1) {
            self.fps = self.count as f32 / elapsed.as_secs_f32();
            self.count = 0;
            self.window_start = Some(now);
        }
    }

    fn fps(&self) -> f32 {
        self.fps
    }
}
