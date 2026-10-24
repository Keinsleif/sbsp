// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The remote's file picker: the audio files the host offers, as a tree to
//! choose from. A remote has no audio files of its own to use, so adding an
//! audio cue or choosing a cue's file shows this instead of the system's
//! file dialog. Ported from `FileListDialog.vue`.
//!
//! The list comes from the host (`FileList`: folders and files, each file
//! with the path the host knows it by) after the dialog asks for it.

use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use sbsp_backend::api::FileList;
use sbsp_backend::model::cue::Uuid;

use crate::domain::new_cue::AUDIO_EXTENSIONS;
use crate::fl;
use crate::widgets::icon::{self, Icon, Tone};

const WIDTH: f32 = 640.0;
const HEIGHT: f32 = 480.0;
const INDENT: f32 = 20.0;

/// What the chosen files are for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// New audio cues, one per file (any number of files).
    Add,
    /// The file of this audio cue (one file).
    Target(Uuid),
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Opens or closes a folder (by its key).
    Toggle(String),
    /// Chooses a file, or takes it back.
    Pick(PathBuf),
    Open,
    Cancel,
}

/// What the dialog decided.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Files(Vec<PathBuf>),
    Cancelled,
}

pub struct Dialog {
    pub purpose: Purpose,
    list: Vec<FileList>,
    /// The list has arrived (until then there is nothing to show).
    loaded: bool,
    expanded: HashSet<String>,
    /// In the order they were chosen.
    selected: Vec<PathBuf>,
}

/// A line of the tree as shown.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Dir {
        key: String,
        name: String,
        depth: usize,
        open: bool,
    },
    File {
        name: String,
        path: PathBuf,
        depth: usize,
        selected: bool,
    },
}

fn is_audio(extension: &str) -> bool {
    AUDIO_EXTENSIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(extension.trim_start_matches('.')))
}

/// Whether the entry is, or holds, an audio file.
fn has_audio(entry: &FileList) -> bool {
    match entry {
        FileList::File { extension, .. } => is_audio(extension),
        FileList::Dir { files, .. } => files.iter().any(has_audio),
    }
}

impl Dialog {
    pub fn new(purpose: Purpose) -> Self {
        Self {
            purpose,
            list: Vec::new(),
            loaded: false,
            expanded: HashSet::new(),
            selected: Vec::new(),
        }
    }

    /// The host's (new) list. What was chosen before is dropped, as the
    /// files may be gone.
    pub fn set_list(&mut self, list: Vec<FileList>) {
        self.list = list;
        self.loaded = true;
        self.selected.clear();
    }

    pub fn selected(&self) -> &[PathBuf] {
        &self.selected
    }

    /// The lines to show: folders that hold audio files, and those files;
    /// the contents of a folder only while it is open.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        self.collect(&self.list, "", 0, &mut rows);
        rows
    }

    fn collect(&self, list: &[FileList], parent: &str, depth: usize, rows: &mut Vec<Row>) {
        for entry in list.iter().filter(|entry| has_audio(entry)) {
            match entry {
                FileList::Dir { name, files } => {
                    // The key is the path of names, which is unique and
                    // does not change when the list is sent again.
                    let key = if parent.is_empty() {
                        name.clone()
                    } else {
                        format!("{parent}/{name}")
                    };
                    let open = self.expanded.contains(&key);
                    rows.push(Row::Dir {
                        key: key.clone(),
                        name: name.clone(),
                        depth,
                        open,
                    });
                    if open {
                        self.collect(files, &key, depth + 1, rows);
                    }
                }
                FileList::File { name, path, .. } => rows.push(Row::File {
                    name: name.clone(),
                    path: path.clone(),
                    depth,
                    selected: self.selected.contains(path),
                }),
            }
        }
    }

    /// Applies a message; `Some` when the dialog is done.
    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        match message {
            Message::Toggle(key) => {
                if !self.expanded.remove(&key) {
                    self.expanded.insert(key);
                }
            }
            Message::Pick(path) => match self.purpose {
                Purpose::Add => match self.selected.iter().position(|chosen| *chosen == path) {
                    Some(index) => {
                        self.selected.remove(index);
                    }
                    None => self.selected.push(path),
                },
                Purpose::Target(_) => self.selected = vec![path],
            },
            Message::Open => {
                return (!self.selected.is_empty())
                    .then(|| Outcome::Files(std::mem::take(&mut self.selected)));
            }
            Message::Cancel => return Some(Outcome::Cancelled),
        }
        None
    }
}

pub fn view(dialog: &Dialog) -> Element<'_, Message> {
    let body: Element<'_, Message> = if !dialog.loaded {
        container(text(fl!("dialog-file-select-loading")))
            .center(Length::Fill)
            .into()
    } else if dialog.rows().is_empty() {
        container(text(fl!("dialog-file-select-empty")))
            .center(Length::Fill)
            .into()
    } else {
        scrollable(column(dialog.rows().into_iter().map(row_view)).width(Length::Fill))
            .height(Length::Fill)
            .into()
    };

    column![
        text(fl!("dialog-file-select-title")).size(18),
        container(body)
            .height(Length::Fill)
            .width(Length::Fill)
            .style(container::bordered_box),
        row![
            Space::new().width(Length::Fill),
            button(text(fl!("general-cancel")))
                .style(button::secondary)
                .on_press(Message::Cancel),
            button(text(fl!("general-open")))
                .style(button::primary)
                .on_press_maybe((!dialog.selected.is_empty()).then_some(Message::Open)),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .width(WIDTH)
    .height(HEIGHT)
    .into()
}

fn row_view<'a>(line: Row) -> Element<'a, Message> {
    match line {
        Row::Dir {
            key,
            name,
            depth,
            open,
        } => button(
            row![
                Space::new().width(depth as f32 * INDENT),
                icon::toned(
                    if open { Icon::MenuDown } else { Icon::MenuRight },
                    18.0,
                    Tone::Text
                ),
                text(name),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .style(button::text)
        .width(Length::Fill)
        .padding([3, 6])
        .on_press(Message::Toggle(key))
        .into(),
        Row::File {
            name,
            path,
            depth,
            selected,
        } => button(
            row![
                Space::new().width(depth as f32 * INDENT),
                icon::toned(Icon::FileMusic, 18.0, Tone::Text),
                text(name),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .style(if selected { button::primary } else { button::text })
        .width(Length::Fill)
        .padding([3, 6])
        .on_press(Message::Pick(path))
        .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, path: &str, extension: &str) -> FileList {
        FileList::File {
            name: name.to_owned(),
            path: PathBuf::from(path),
            extension: extension.to_owned(),
        }
    }

    fn dir(name: &str, files: Vec<FileList>) -> FileList {
        FileList::Dir {
            name: name.to_owned(),
            files,
        }
    }

    fn sample() -> Vec<FileList> {
        vec![
            dir(
                "music",
                vec![
                    file("a.mp3", "/m/a.mp3", "mp3"),
                    file("notes.txt", "/m/notes.txt", "txt"),
                    dir("deep", vec![file("b.WAV", "/m/deep/b.WAV", "WAV")]),
                ],
            ),
            dir("docs", vec![file("readme.md", "/d/readme.md", "md")]),
            file("top.flac", "/top.flac", "flac"),
        ]
    }

    fn names(dialog: &Dialog) -> Vec<String> {
        dialog
            .rows()
            .into_iter()
            .map(|row| match row {
                Row::Dir { name, .. } | Row::File { name, .. } => name,
            })
            .collect()
    }

    #[test]
    fn only_audio_files_and_the_folders_holding_them_are_listed() {
        let mut dialog = Dialog::new(Purpose::Add);
        dialog.set_list(sample());
        // `docs` has no audio; `music` is closed.
        assert_eq!(names(&dialog), ["music", "top.flac"]);

        dialog.update(Message::Toggle("music".to_owned()));
        assert_eq!(names(&dialog), ["music", "a.mp3", "deep", "top.flac"]);

        dialog.update(Message::Toggle("music/deep".to_owned()));
        assert_eq!(names(&dialog), ["music", "a.mp3", "deep", "b.WAV", "top.flac"]);

        dialog.update(Message::Toggle("music".to_owned()));
        assert_eq!(names(&dialog), ["music", "top.flac"]);
    }

    #[test]
    fn several_files_can_be_chosen_to_add_and_one_for_a_target() {
        let mut add = Dialog::new(Purpose::Add);
        add.set_list(sample());
        add.update(Message::Pick(PathBuf::from("/top.flac")));
        add.update(Message::Pick(PathBuf::from("/m/a.mp3")));
        assert_eq!(add.selected(), [PathBuf::from("/top.flac"), PathBuf::from("/m/a.mp3")]);
        add.update(Message::Pick(PathBuf::from("/top.flac")));
        assert_eq!(add.selected(), [PathBuf::from("/m/a.mp3")], "picking again takes it back");

        let mut target = Dialog::new(Purpose::Target(Uuid::nil()));
        target.set_list(sample());
        target.update(Message::Pick(PathBuf::from("/top.flac")));
        target.update(Message::Pick(PathBuf::from("/m/a.mp3")));
        assert_eq!(target.selected(), [PathBuf::from("/m/a.mp3")]);
    }

    #[test]
    fn open_needs_a_choice_and_hands_the_files_over_in_order() {
        let mut dialog = Dialog::new(Purpose::Add);
        dialog.set_list(sample());
        assert_eq!(dialog.update(Message::Open), None);

        dialog.update(Message::Pick(PathBuf::from("/top.flac")));
        assert_eq!(
            dialog.update(Message::Open),
            Some(Outcome::Files(vec![PathBuf::from("/top.flac")]))
        );
        assert_eq!(dialog.update(Message::Cancel), Some(Outcome::Cancelled));
    }

    #[test]
    fn a_new_list_clears_the_choice_but_keeps_the_open_folders() {
        let mut dialog = Dialog::new(Purpose::Add);
        dialog.set_list(sample());
        dialog.update(Message::Toggle("music".to_owned()));
        dialog.update(Message::Pick(PathBuf::from("/m/a.mp3")));
        assert!(matches!(
            dialog.rows()[1],
            Row::File { selected: true, .. }
        ));

        dialog.set_list(sample());
        assert!(dialog.selected().is_empty());
        assert_eq!(names(&dialog), ["music", "a.mp3", "deep", "top.flac"]);
    }
}
