// SPDX-License-Identifier: GPL-3.0-or-later
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
use crate::widgets::waveform_editor;

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
            check(
                fl!("main-bottom-editor-fade-stop-on-complete"),
                fade.stop_target_on_complete,
                enabled.then_some(Message::FadeStopOnComplete(!fade.stop_target_on_complete)),
            ),
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
/// The waveform editor, and the columns beside it.
const WAVE_HEIGHT: f32 = 108.0;

/// The waveform with its start / end markers and envelope, start and end
/// time, volume (with the LUFS / MAX buttons), pan and repeat of an audio
/// cue.
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

    let hint = |label: String| {
        container(text(label).size(12))
            .padding(6)
            .style(container::rounded_box)
    };
    let tipped = |content: Element<'a, Message>, label: String, position: tooltip::Position| {
        tooltip(content, hint(label), position)
    };

    let time = |id, label: String, value: Option<f64>, glyph: Icon, hint_text: String, message: Message| {
        row![
            container(
                time_input(id, Some(&label), value, &spec, ctx.drafts, !active).map(Message::Field)
            )
            .width(Length::Fill),
            tipped(
                button(icon(glyph, 18.0))
                    .padding(4)
                    .style(button::secondary)
                    .on_press_maybe((!active).then_some(message))
                    .into(),
                hint_text,
                tooltip::Position::Right,
            ),
        ]
        .spacing(4)
        .align_y(Alignment::End)
    };
    let times = column![
        time(
            START,
            fl!("main-bottom-editor-time-levels-start-time"),
            cue_edit::audio_start(cue),
            Icon::SkipNext,
            fl!("main-bottom-editor-time-levels-skip-first-silence"),
            Message::SkipFirstSilence,
        ),
        time(
            END,
            fl!("main-bottom-editor-time-levels-end-time"),
            cue_edit::audio_end(cue, ctx.file.duration),
            Icon::SkipPrevious,
            fl!("main-bottom-editor-time-levels-skip-last-silence"),
            Message::SkipLastSilence,
        ),
    ]
    .spacing(8)
    .width(Length::Fixed(200.0));

    // The volume can be changed while the cue plays; the app passes the
    // change on to the playing cue.
    let volume_value = editor.volume_for(cue);
    let (segments, range) = editor.shown(cue);
    let selected = editor.selected_in(cue);
    let wave = container(
        waveform_editor::view(waveform_editor::Props {
            surface: &editor.waveform,
            path: audio.target.clone(),
            waveform: ctx.waveform,
            gain: editor.scale_waveform.then_some(volume_value as f32),
            segments,
            range,
            envelope_visible: editor.envelope_visible,
            selected,
            duration: ctx.file.duration,
            playhead: ctx.playing.map(|playing| playing.ratio),
            seek_duration: ctx.playing.map(|playing| playing.duration),
            height: WAVE_HEIGHT,
        })
        .map(Message::Wave),
    )
    .width(Length::Fill)
    .height(WAVE_HEIGHT)
    .style(container::bordered_box);

    let tool = |glyph: Icon, message: Message, enabled: bool| {
        button(icon(glyph, 18.0))
            .padding(3)
            .style(button::secondary)
            .on_press_maybe(enabled.then_some(message))
    };
    let tools = column![
        tool(Icon::Plus, Message::AddSegment, !active),
        tool(
            Icon::Minus,
            Message::RemoveSegment,
            !active && selected.is_some()
        ),
        tool(
            Icon::TrashCan,
            Message::ClearSegments,
            !active && editor.envelope_visible
        ),
        tool(
            if editor.envelope_visible {
                Icon::Eye
            } else {
                Icon::EyeOff
            },
            Message::ToggleEnvelope,
            true
        ),
    ]
    .spacing(4)
    .align_x(Alignment::Center);

    let top = row![times, wave, tools].spacing(8).height(WAVE_HEIGHT);

    let volume_label = fl!("main-bottom-editor-time-levels-volume");
    let volume = volume_fader(
        VolumeFader {
            id: VOLUME,
            label: &volume_label,
            volume: volume_value,
            thumb: ThumbAmount::Decreased,
            enabled: true,
        },
        ctx.drafts,
    )
    .map(Message::Volume);

    let match_button = |label: &'static str, message: Message, known: bool, hint_text: String| {
        tipped(
            button(text(label).center())
                .padding([2, 8])
                .style(button::secondary)
                .on_press_maybe(known.then_some(message))
                .into(),
            hint_text,
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
        column![
            check(
                fl!("main-bottom-editor-time-levels-repeat"),
                audio.repeat,
                (!active).then_some(Message::Repeat(!audio.repeat)),
            ),
            check(
                fl!("main-bottom-editor-time-levels-scale-waveform"),
                editor.scale_waveform,
                Some(Message::ToggleScaleWaveform),
            ),
        ],
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .height(FADERS_HEIGHT);

    body(column![top, faders].spacing(8).padding(4))
}
