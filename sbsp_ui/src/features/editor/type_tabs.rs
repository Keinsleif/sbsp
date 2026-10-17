// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The editor's tabs that belong to one kind of cue: audio, fade,
//! playback (start / stop / pause / load) and group. Ported from
//! `AudioBasicEditor.vue`, `FadeBasicEditor.vue`,
//! `PlaybakcBasicEditor.vue` and `GroupBasicEditor.vue`.
//!
//! While the cue is playing its settings are greyed out, as in the
//! original.

use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, scrollable, text, tooltip};
use iced::{Alignment, Element, Length};

use sbsp_backend::model::cue::audio::SoundType;
use sbsp_backend::model::cue::{Cue, CueParam};

use crate::domain::cue_edit::{self, FadeSlot, GroupModeKind};
use crate::domain::field::TimeSpec;
use crate::domain::flat_list::FlatCueEntry;
use crate::fl;
use crate::widgets::field;
use crate::widgets::icon::{Icon, icon};
use crate::widgets::divider;
use crate::widgets::inputs::{
    PanningFader, ThumbAmount, VolumeFader, labeled, panning_fader, time_input, volume_fader,
};

use super::parts::{Choice, FadeInput, check, cue_choices, fade_param_input, picker};
use super::{AUDIO_TARGET, Context, Editor, END, Message, PAN, START, VOLUME};

fn body<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(container(content).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(super) fn audio<'a>(ctx: &Context<'a>, cue: &'a Cue) -> Element<'a, Message> {
    let CueParam::Audio(audio) = &cue.params else {
        return column![].into();
    };
    let enabled = !ctx.is_active;

    let target: Element<'a, field::Message> = crate::widgets::field::text_field(
        AUDIO_TARGET,
        &audio.target.to_string_lossy(),
        ctx.drafts,
        enabled,
    )
    .align_x(Horizontal::Center)
    .width(Length::Fill)
    .into();
    let target = labeled(
        Some(&fl!("main-bottom-editor-audio-target-file")),
        target.map(Message::Field),
    );
    let pick = button(icon(Icon::FileMusic, 20.0))
        .style(button::secondary)
        .on_press_maybe(enabled.then_some(Message::PickAudio));

    let is_static = audio.sound_type == SoundType::Static;
    let fades = row![
        container(fade_param_input(FadeInput {
            slot: FadeSlot::AudioIn,
            label: fl!("main-bottom-editor-audio-fade-in"),
            param: audio.fade_in_param,
            toggle: true,
            both_curves: false,
            enabled,
            drafts: ctx.drafts,
        }))
        .width(Length::FillPortion(1)),
        container(fade_param_input(FadeInput {
            slot: FadeSlot::AudioOut,
            label: fl!("main-bottom-editor-audio-fade-out"),
            param: audio.fade_out_param,
            toggle: true,
            both_curves: false,
            enabled,
            drafts: ctx.drafts,
        }))
        .width(Length::FillPortion(1)),
    ]
    .spacing(12);

    body(
        column![
            row![container(target).width(Length::Fill), pick]
                .spacing(8)
                .align_y(Alignment::End),
            check(
                fl!("main-bottom-editor-audio-load-entire-file-on-memory"),
                is_static,
                enabled.then_some(Message::SoundStatic(!is_static)),
            ),
            fades,
        ]
        .spacing(12)
        .padding(4),
    )
}

pub(super) fn fade<'a>(
    editor: &'a Editor,
    ctx: &Context<'a>,
    cue: &'a Cue,
    flat: &[FlatCueEntry],
) -> Element<'a, Message> {
    let CueParam::Fade(fade) = &cue.params else {
        return column![].into();
    };
    let enabled = !ctx.is_active;

    // A fade needs something to fade: an audio cue or a group.
    let targets = cue_choices(ctx, flat, |other| {
        matches!(other.params, CueParam::Audio(_) | CueParam::Group { .. })
    });
    let target = labeled(
        Some(&fl!("main-bottom-editor-target-cue")),
        picker(targets, Some(&fade.target), enabled, Message::FadeTarget),
    );

    let volume_label = fl!("main-bottom-editor-fade-target-volume");
    let volume = volume_fader(
        VolumeFader {
            id: VOLUME,
            label: &volume_label,
            volume: editor.volume_for(cue),
            thumb: ThumbAmount::Full,
            enabled,
        },
        ctx.drafts,
    )
    .map(Message::Volume);

    let param = fade_param_input(FadeInput {
        slot: FadeSlot::Fade,
        label: fl!("main-bottom-editor-fade-fade-parameter"),
        param: Some(fade.fade_param),
        toggle: false,
        both_curves: true,
        enabled,
        drafts: ctx.drafts,
    });

    body(
        column![
            container(target).max_width(600.0),
            volume,
            container(param).align_x(Horizontal::Left),
        ]
        .spacing(4)
        .padding(4),
    )
}

pub(super) fn playback<'a>(
    ctx: &Context<'a>,
    cue: &'a Cue,
    flat: &[FlatCueEntry],
) -> Element<'a, Message> {
    let Some(current) = cue_edit::playback_target(cue) else {
        return column![].into();
    };
    let enabled = !ctx.is_active;

    // Any other cue can be started, stopped, paused or loaded.
    let targets = cue_choices(ctx, flat, |other| other.id != cue.id);
    let target = labeled(
        Some(&fl!("main-bottom-editor-target-cue")),
        picker(targets, Some(&current), enabled, Message::PlaybackTarget),
    );

    let mut content = column![container(target).max_width(600.0)].spacing(12).padding(4);
    if let CueParam::Stop(stop) = &cue.params {
        content = content.push(check(
            fl!("main-bottom-editor-playback-hard"),
            stop.hard,
            enabled.then_some(Message::StopHard(!stop.hard)),
        ));
    }
    body(content)
}

pub(super) fn group<'a>(ctx: &Context<'a>, cue: &'a Cue) -> Element<'a, Message> {
    use sbsp_backend::model::cue::group::GroupMode;

    let Some(mode) = cue_edit::group_mode(cue) else {
        return column![].into();
    };
    let enabled = !ctx.is_active;

    let kinds = [
        (GroupModeKind::Playlist, fl!("main-bottom-editor-group-mode-playlist")),
        (GroupModeKind::Concurrency, fl!("main-bottom-editor-group-mode-concurrency")),
        (GroupModeKind::StartFirst, fl!("main-bottom-editor-group-mode-start-first")),
    ]
    .into_iter()
    .map(|(kind, label)| Choice::new(kind, label))
    .collect();
    let mode_elem = labeled(
        Some(&fl!("main-bottom-editor-group-mode-label")),
        picker(
            kinds,
            Some(&cue_edit::group_mode_kind(&mode)),
            enabled,
            Message::GroupMode,
        ),
    );

    let mut content = column![container(mode_elem).max_width(400.0)].spacing(12).padding(4);
    match mode {
        GroupMode::Playlist { repeat } => {
            content = content.push(check(
                fl!("main-bottom-editor-time-levels-repeat"),
                repeat,
                enabled.then_some(Message::GroupRepeat(!repeat)),
            ));
        }
        GroupMode::StartFirst { enter } => {
            content = content.push(check(
                fl!("main-bottom-editor-group-advance-cursor-into"),
                enter,
                enabled.then_some(Message::GroupEnter(!enter)),
            ));
        }
        GroupMode::Concurrency => {}
    }
    body(content)
}

/// Height of the row of faders (the dividers between them need it).
const FADERS_HEIGHT: f32 = 72.0;

/// Start and end time, volume (with the LUFS / MAX buttons), pan and
/// repeat of an audio cue. The waveform of the original's tab is a
/// separate part (step 3c).
pub(super) fn levels<'a>(
    editor: &'a Editor,
    ctx: &Context<'a>,
    cue: &'a Cue,
) -> Element<'a, Message> {
    let CueParam::Audio(audio) = &cue.params else {
        return column![].into();
    };
    let active = ctx.is_active;
    let spec = TimeSpec::default();

    let time = |id, label: String, value: Option<f64>| {
        container(
            time_input(id, Some(&label), value, &spec, ctx.drafts, !active).map(Message::Field),
        )
        .width(Length::Fixed(120.0))
    };
    let times = row![
        time(
            START,
            fl!("main-bottom-editor-time-levels-start-time"),
            cue_edit::audio_start(cue),
        ),
        time(
            END,
            fl!("main-bottom-editor-time-levels-end-time"),
            cue_edit::audio_end(cue, ctx.file.duration),
        ),
    ]
    .spacing(12);

    // The volume can be changed while the cue plays; the app passes the
    // change on to the playing cue.
    let volume_label = fl!("main-bottom-editor-time-levels-volume");
    let volume = volume_fader(
        VolumeFader {
            id: VOLUME,
            label: &volume_label,
            volume: editor.volume_for(cue),
            thumb: ThumbAmount::Decreased,
            enabled: true,
        },
        ctx.drafts,
    )
    .map(Message::Volume);

    let hint = |label: String| {
        container(text(label).size(12))
            .padding(6)
            .style(container::rounded_box)
    };
    let match_button = |label: &'static str, message: Message, known: bool, hint_text: String| {
        tooltip(
            button(text(label).center())
                .padding([2, 8])
                .style(button::secondary)
                .on_press_maybe(known.then_some(message)),
            hint(hint_text),
            tooltip::Position::Right,
        )
    };
    let matches = column![
        match_button(
            "LUFS",
            Message::VolumeToLufs,
            ctx.file.integrated_lufs.is_some(),
            fl!(
                "main-bottom-editor-time-levels-lufs-description",
                targetLUFS = ctx.lufs_target
            ),
        ),
        match_button(
            "MAX",
            Message::VolumeToPeak,
            ctx.file.peak.is_some(),
            fl!("main-bottom-editor-time-levels-peak-description"),
        ),
    ]
    .spacing(4);

    let pan_label = fl!("main-bottom-editor-time-levels-pan");
    let pan = panning_fader(
        PanningFader {
            id: PAN,
            label: &pan_label,
            panning: editor.pan_for(cue),
            enabled: !active,
        },
        ctx.drafts,
    )
    .map(Message::Pan);

    let faders = row![
        row![container(volume).width(Length::Fill), matches]
            .spacing(8)
            .width(Length::FillPortion(3)),
        divider::vertical(),
        container(pan).width(Length::FillPortion(2)),
        divider::vertical(),
        check(
            fl!("main-bottom-editor-time-levels-repeat"),
            audio.repeat,
            (!active).then_some(Message::Repeat(!audio.repeat)),
        ),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .height(FADERS_HEIGHT);

    body(column![times, faders].spacing(12).padding(4))
}
