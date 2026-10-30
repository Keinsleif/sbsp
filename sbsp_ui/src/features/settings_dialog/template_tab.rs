// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The template tab: the cues a new cue starts from, one per kind, listed
//! above the same editor the main window has. Ported from
//! `TemplateSettings.vue` and `TemplateRow.vue`.
//!
//! The editor's pickers (chain target, fade target and so on) list the
//! cues of the show that is open, as in the original.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};
use sbsp_backend::helper::SupportedHardware;
use sbsp_backend::model::cue::{Cue, CueParam, group::GroupMode};
use crate::settings::TemplateSettings;

use super::{EditorMessage, Message};
use crate::domain::flat_list::FlatCueEntry;
use crate::domain::time::format_seconds;
use crate::features::cue_list::{chain_icon, type_icon};
use crate::features::editor::{self, Editor};
use crate::fl;
use crate::state::model::ShowModelState;
use crate::widgets::field::Drafts;
use crate::widgets::icon::{self, Icon};

/// One of the eight templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Audio,
    Wait,
    Fade,
    Start,
    Stop,
    Pause,
    Load,
    Group,
}

const KINDS: [Kind; 8] = [
    Kind::Audio,
    Kind::Wait,
    Kind::Fade,
    Kind::Start,
    Kind::Stop,
    Kind::Pause,
    Kind::Load,
    Kind::Group,
];

impl Kind {
    pub fn get(self, templates: &TemplateSettings) -> &Cue {
        match self {
            Kind::Audio => &templates.audio,
            Kind::Wait => &templates.wait,
            Kind::Fade => &templates.fade,
            Kind::Start => &templates.start,
            Kind::Stop => &templates.stop,
            Kind::Pause => &templates.pause,
            Kind::Load => &templates.load,
            Kind::Group => &templates.group,
        }
    }

    pub fn set(self, templates: &mut TemplateSettings, cue: Cue) {
        let target = match self {
            Kind::Audio => &mut templates.audio,
            Kind::Wait => &mut templates.wait,
            Kind::Fade => &mut templates.fade,
            Kind::Start => &mut templates.start,
            Kind::Stop => &mut templates.stop,
            Kind::Pause => &mut templates.pause,
            Kind::Load => &mut templates.load,
            Kind::Group => &mut templates.group,
        };
        *target = cue;
    }

    fn label(self) -> String {
        match self {
            Kind::Audio => fl!("dialog-settings-global-template-audio"),
            Kind::Wait => fl!("dialog-settings-global-template-wait"),
            Kind::Fade => fl!("dialog-settings-global-template-fade"),
            Kind::Start => fl!("dialog-settings-global-template-start"),
            Kind::Stop => fl!("dialog-settings-global-template-stop"),
            Kind::Pause => fl!("dialog-settings-global-template-pause"),
            Kind::Load => fl!("dialog-settings-global-template-load"),
            Kind::Group => fl!("dialog-settings-global-template-group"),
        }
    }
}

/// What the editor under the template list and the hardware tab need
/// besides the settings.
pub struct Editing<'a, 'f> {
    pub editor: &'a Editor,
    pub model: &'a ShowModelState,
    pub flat: &'f [FlatCueEntry],
    /// What the audio hardware tab lists (host only; `None` while it is
    /// being asked for).
    pub hardware: Option<&'a Result<SupportedHardware, String>>,
}

const W_TYPE: f32 = 150.0;
const W_NUMBER: f32 = 50.0;
const W_TIME: f32 = 80.0;
const W_MARK: f32 = 24.0;
/// The narrowest the list is laid out at; below it, it scrolls sideways.
const MIN_WIDTH: f32 = 600.0;

fn cell<'a>(content: impl Into<Element<'a, Message>>, width: Length) -> Element<'a, Message> {
    container(content).width(width).into()
}

fn repeats(cue: &Cue) -> bool {
    match &cue.params {
        CueParam::Audio(audio) => audio.repeat,
        CueParam::Group { base, .. } => matches!(base.mode, GroupMode::Playlist { repeat: true }),
        _ => false,
    }
}

fn header<'a>() -> Element<'a, Message> {
    let title = |label: String, width: Length| cell(text(label).size(12), width);
    row![
        title(fl!("dialog-settings-global-template-type"), Length::Fixed(W_TYPE)),
        title(fl!("main-number"), Length::Fixed(W_NUMBER)),
        title(fl!("main-name"), Length::Fill),
        title(fl!("main-pre-wait"), Length::Fixed(W_TIME)),
        title(fl!("main-duration"), Length::Fixed(W_TIME)),
        Space::new().width(W_MARK * 2.0),
    ]
    .spacing(6)
    .padding([2, 8])
    .into()
}

fn line<'a>(kind: Kind, cue: &Cue, selected: bool) -> Element<'a, Message> {
    let name = match &cue.name {
        Some(name) => name.clone(),
        None => fl!("dialog-settings-global-template-built-from-cue-param"),
    };
    let duration = crate::domain::cue::get_duration(Some(cue), |_| None);
    let mark = |glyph: Option<Icon>| -> Element<'a, Message> {
        match glyph {
            Some(glyph) => icon::icon(glyph, 16.0).into(),
            None => Space::new().into(),
        }
    };

    let columns = row![
        cell(
            row![icon::icon(type_icon(&cue.params), 16.0), text(kind.label()).size(13)]
                .spacing(6)
                .align_y(Alignment::Center),
            Length::Fixed(W_TYPE),
        ),
        cell(text(cue.number.clone()).size(13), Length::Fixed(W_NUMBER)),
        cell(text(name).size(13), Length::Fill),
        cell(text(format_seconds(Some(cue.pre_wait))).size(13), Length::Fixed(W_TIME)),
        cell(text(format_seconds(duration)).size(13), Length::Fixed(W_TIME)),
        cell(mark(repeats(cue).then_some(Icon::Repeat)), Length::Fixed(W_MARK)),
        cell(mark(chain_icon(&cue.chain)), Length::Fixed(W_MARK)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    button(columns)
        .width(Length::Fill)
        .padding([3, 8])
        .style(if selected { button::secondary } else { button::text })
        .on_press(Message::SelectTemplate(kind))
        .into()
}

pub fn view<'a>(
    templates: &'a TemplateSettings,
    selected: Option<Kind>,
    name_format: &'a crate::settings::NameFormatSettings,
    drafts: &'a Drafts,
    editing: Editing<'a, '_>,
) -> Element<'a, Message> {
    // Built again whenever the room changes, from what it borrows. Inside
    // the dialog's vertical scroll the height is not known, so it shrinks.
    let list = crate::widgets::min_width::min_width(MIN_WIDTH, move |width| {
        let mut list = column![header()].spacing(1).width(width);
        for kind in KINDS {
            list = list.push(line(kind, kind.get(templates), selected == Some(kind)));
        }
        list.into()
    })
    .height(Length::Shrink);

    let cue = selected.map(|kind| kind.get(templates));
    let editor = editor::view(
        editing.editor,
        editor::Context {
            cue,
            model: editing.model,
            name_format,
            drafts,
            duration: crate::domain::cue::get_duration(cue, |_| None),
            is_active: false,
            chain_override: None,
            file: editor::FileInfo::default(),
            lufs_target: editing.model.settings.audio.lufs_target,
            waveform: None,
            playing: None,
        },
        editing.flat.to_vec(),
    )
    .map(|message| Message::Editor(EditorMessage(message)));

    column![
        list,
        container(editor)
            .width(Length::Fill)
            .height(editor::HEIGHT + 2.0)
            .style(container::bordered_box),
    ]
    .spacing(8)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_reads_and_writes_its_own_template() {
        let mut templates = TemplateSettings::default();
        for (i, kind) in KINDS.into_iter().enumerate() {
            let mut cue = kind.get(&templates).clone();
            cue.number = format!("T{i}");
            kind.set(&mut templates, cue);
        }
        for (i, kind) in KINDS.into_iter().enumerate() {
            assert_eq!(kind.get(&templates).number, format!("T{i}"));
        }
    }

    #[test]
    fn the_kinds_match_their_cue_types() {
        let templates = TemplateSettings::default();
        assert!(matches!(Kind::Audio.get(&templates).params, CueParam::Audio(_)));
        assert!(matches!(Kind::Wait.get(&templates).params, CueParam::Wait(_)));
        assert!(matches!(Kind::Group.get(&templates).params, CueParam::Group { .. }));
    }
}
