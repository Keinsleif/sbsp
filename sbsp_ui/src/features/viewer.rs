// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The bottom viewer: the selected audio cue's waveform with its start/end
//! markers, the live playback position, a hover time tooltip, and
//! click-to-seek. Ported from `BottomViewer.vue`, `WaveformPath.vue` and
//! `waveform.worker.ts`.
//!
//! The original rendered on an `OffscreenCanvas` in a worker because
//! re-drawing on a webview's main thread would stall it. Here the
//! waveform geometry sits in a [`canvas::Cache`] (rebuilt only when the
//! cue, its waveform data, its gain or the width changes) and only the
//! cheap overlay (markers, position, tooltip) is redrawn per frame, so no
//! worker is needed.
//!
//! The editor that replaces this area in Edit mode is Phase 6; until then
//! the viewer is shown in every mode.

use std::cell::RefCell;
use std::path::PathBuf;

use iced::mouse;
use iced::widget::canvas::{self, Cache, Canvas, Path as CanvasPath, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use sbsp_backend::controller::state::{ActiveCue, PlaybackStatus};
use sbsp_backend::model::cue::{audio::AudioCueParam, CueParam, Uuid};

use crate::domain::time::format_seconds;
use crate::state::assets::AssetResults;
use crate::state::model::ShowModelState;
use crate::state::playback::PlaybackState;
use crate::state::ui::{Mode, UiState};

/// The original's `h-16`.
pub const HEIGHT: f32 = 64.0;

/// A scaled sample at or above this is drawn as clipped (f32 epsilon over
/// full scale, as in the worker).
const CLIP_THRESHOLD: f32 = 1.0 + f32::EPSILON;

const TOOLTIP_W: f32 = 128.0;
const TOOLTIP_H: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    /// Seek this cue to this many seconds.
    Seek(Uuid, f64),
}

/// What the cached waveform geometry was built from. Compared on every
/// draw; a difference clears the cache. (The width is tracked by the
/// cache itself.)
#[derive(Debug, Clone, PartialEq)]
struct CacheKey {
    path: PathBuf,
    samples: usize,
    gain_bits: u32,
}

/// Owned by the app so the cache survives the per-frame `view()` rebuild.
#[derive(Default)]
pub struct Viewer {
    cache: Cache,
    key: RefCell<Option<CacheKey>>,
}

/// Where the cue's start/end sit on the file, as fractions of its length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeRange {
    pub start: f64,
    pub end: f64,
}

impl TimeRange {
    pub const FULL: Self = Self { start: 0.0, end: 1.0 };

    /// A missing (or zero, like the original's `||`) end time means "to
    /// the end of the file".
    pub fn new(duration: Option<f64>, start_time: Option<f64>, end_time: Option<f64>) -> Self {
        let duration = duration.filter(|d| *d != 0.0).unwrap_or(1.0);
        let start = start_time.unwrap_or(0.0) / duration;
        let end = end_time.filter(|t| *t != 0.0).unwrap_or(duration) / duration;
        Self { start, end }
    }

    pub fn delta(&self) -> f64 {
        self.end - self.start
    }
}

/// The waveform's gain: the original shows the file normalized to its peak
/// (`-peak` dB), or untouched when no peak is known yet.
pub fn gain_db(peak: Option<f32>) -> f32 {
    match peak {
        Some(peak) if peak != 0.0 => -peak,
        _ => 0.0,
    }
}

/// The loudest sample under each of `width` pixel columns. A column that
/// starts past the end of the data is 0.
pub fn column_peaks(waveform: &[f32], width: usize) -> Vec<f32> {
    if waveform.is_empty() || width == 0 {
        return vec![0.0; width];
    }
    let per_pixel = waveform.len() as f64 / width as f64;

    (0..width)
        .map(|x| {
            let start = (x as f64 * per_pixel).floor() as usize;
            let end = ((x + 1) as f64 * per_pixel).floor() as usize;
            let Some(&first) = waveform.get(start) else {
                return 0.0;
            };
            waveform[start..end.min(waveform.len())]
                .iter()
                .fold(first, |max, &sample| max.max(sample))
        })
        .collect()
}

/// `x` is the click position as a fraction of the viewer width. `None`
/// outside the cue's own start..end.
pub fn seek_seconds(x: f64, range: TimeRange, cue_duration: f64) -> Option<f64> {
    if range.delta() <= 0.0 {
        return None;
    }
    let position = (x - range.start) / range.delta();
    (0.0..=1.0)
        .contains(&position)
        .then_some(position * cue_duration)
}

/// How far through its duration an active cue is. Pre-wait counts as the
/// start (the waveform only shows the audio itself).
pub fn position_ratio(active: &ActiveCue) -> f64 {
    let waiting = matches!(
        active.status,
        PlaybackStatus::PreWaiting | PlaybackStatus::PreWaitPaused
    );
    if active.duration != 0.0 && !waiting {
        active.position / active.duration
    } else {
        0.0
    }
}

/// The waveform as vertical lines around the center line, scaled by `gain`
/// dB (`None`: as it is, and nothing is marked as clipping). Samples at or
/// over full scale are drawn red.
pub fn draw_peaks(frame: &mut canvas::Frame, waveform: Option<&[f32]>, gain: Option<f32>) {
    let size = frame.size();
    let center_y = size.height / 2.0;
    let gray = Color::from_rgba8(128, 128, 128, 0.8);

    frame.fill_rectangle(Point::new(0.0, center_y), Size::new(size.width, 1.0), gray);

    let Some(waveform) = waveform else {
        return;
    };
    let scale = gain.map_or(1.0, |gain| 10f32.powf(gain / 20.0));
    let amplitude = size.height * 0.375;

    let mut normal = Vec::new();
    let mut clipped = Vec::new();
    for (x, peak) in column_peaks(waveform, size.width as usize)
        .into_iter()
        .enumerate()
    {
        if peak <= 0.0 {
            continue;
        }
        let scaled = peak * scale;
        let line = (
            x as f32 + 0.5,
            center_y - scaled * amplitude,
            center_y + scaled * amplitude,
        );
        if gain.is_none() || scaled < CLIP_THRESHOLD {
            normal.push(line);
        } else {
            clipped.push(line);
        }
    }

    let stroke = |lines: &[(f32, f32, f32)], color: Color, frame: &mut canvas::Frame| {
        if lines.is_empty() {
            return;
        }
        let path = CanvasPath::new(|builder| {
            for &(x, top, bottom) in lines {
                builder.move_to(Point::new(x, top));
                builder.line_to(Point::new(x, bottom));
            }
        });
        frame.stroke(&path, Stroke::default().with_color(color).with_width(1.0));
    };
    stroke(&normal, gray, frame);
    stroke(&clipped, Color::from_rgba8(200, 96, 96, 0.8), frame);
}

/// "position / duration" next to the pointer, for a pointer over the whole
/// width of a file of `duration` seconds.
pub fn draw_tooltip(frame: &mut canvas::Frame, theme: &Theme, cursor: Point, duration: Option<f64>) {
    let size = frame.size();
    let label = match duration {
        Some(duration) if size.width > 0.0 => format!(
            "{} / {}",
            format_seconds(Some(f64::from(cursor.x / size.width) * duration)),
            format_seconds(Some(duration)),
        ),
        _ => "--:--.-- / --:--.--".to_string(),
    };

    // Right of the pointer, or left of it when there is no room.
    let x = if cursor.x + 16.0 + TOOLTIP_W > size.width {
        (cursor.x - 16.0 - TOOLTIP_W).max(0.0)
    } else {
        cursor.x + 16.0
    };
    let y = cursor.y.min(size.height - TOOLTIP_H).max(0.0);

    let palette = theme.extended_palette();
    let origin = Point::new(x, y);
    let box_size = Size::new(TOOLTIP_W, TOOLTIP_H);
    frame.fill_rectangle(origin, box_size, palette.background.weak.color);
    frame.stroke_rectangle(
        origin,
        box_size,
        Stroke::default()
            .with_color(palette.background.strong.color)
            .with_width(1.0),
    );
    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(x + TOOLTIP_W / 2.0, y + TOOLTIP_H / 2.0),
        color: palette.background.weak.text,
        size: 12.0.into(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        font: crate::theme::fonts::UI,
        ..canvas::Text::default()
    });
}

/// What is selected, resolved against the model and the asset cache.
struct Target<'a> {
    id: Uuid,
    audio: Option<&'a AudioCueParam>,
    waveform: Option<&'a [f32]>,
    gain_db: f32,
    duration: Option<f64>,
    range: TimeRange,
    active: Option<&'a ActiveCue>,
    can_seek: bool,
}

pub fn view<'a>(
    viewer: &'a Viewer,
    model: &'a ShowModelState,
    ui: &UiState,
    playback: &'a PlaybackState,
    assets: &'a AssetResults,
) -> Element<'a, Message> {
    let target = ui
        .selected
        .and_then(|id| model.cue_list.cues.get(&id))
        .map(|cue| {
            let audio = match &cue.params {
                CueParam::Audio(audio) => Some(audio),
                _ => None,
            };
            let data = audio.and_then(|audio| assets.get(&audio.target));
            let duration = audio
                .and_then(|audio| assets.get_metadata(&audio.target))
                .and_then(|metadata| metadata.duration);
            let active = playback.active_cues.get(&cue.id);

            Target {
                id: cue.id,
                audio,
                waveform: data
                    .map(|data| data.waveform.as_slice())
                    .filter(|waveform| !waveform.is_empty()),
                gain_db: gain_db(data.map(|data| data.peak)),
                duration,
                range: audio
                    .map(|audio| TimeRange::new(duration, audio.start_time, audio.end_time))
                    .unwrap_or(TimeRange::FULL),
                active,
                can_seek: active.is_some() && ui.mode != Mode::View,
            }
        });

    Canvas::new(Waveform { viewer, target })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

struct Waveform<'a> {
    viewer: &'a Viewer,
    target: Option<Target<'a>>,
}

impl Waveform<'_> {
    fn cache_key(&self) -> Option<CacheKey> {
        let target = self.target.as_ref()?;
        Some(CacheKey {
            path: target.audio.map(|a| a.target.clone()).unwrap_or_default(),
            samples: target.waveform.map_or(0, <[f32]>::len),
            gain_bits: target.gain_db.to_bits(),
        })
    }

    fn draw_waveform(&self, frame: &mut canvas::Frame) {
        let gain = self.target.as_ref().map(|t| t.gain_db);
        draw_peaks(frame, self.target.as_ref().and_then(|t| t.waveform), gain);
    }

    fn draw_overlay(
        &self,
        frame: &mut canvas::Frame,
        theme: &Theme,
        cursor: Option<Point>,
    ) {
        let Some(target) = &self.target else {
            return;
        };
        let size = frame.size();
        let span = (size.width - 1.0).max(0.0);

        if target.audio.is_some() {
            let blue = Color::from_rgb8(0, 0, 255);
            frame.fill_rectangle(
                Point::new(target.range.start as f32 * span - 1.0, 0.0),
                Size::new(2.0, size.height),
                blue,
            );
            frame.fill_rectangle(
                Point::new(target.range.end as f32 * span, 0.0),
                Size::new(2.0, size.height),
                blue,
            );
        }

        if let Some(active) = target.active {
            let ratio = position_ratio(active) as f32;
            let x = (target.range.start as f32 + ratio * target.range.delta() as f32) * span;
            frame.fill_rectangle(
                Point::new(x, 0.0),
                Size::new(2.0, size.height),
                Color::from_rgb8(255, 255, 0),
            );
        }

        if let Some(point) = cursor {
            self.draw_tooltip(frame, theme, point, target);
        }
    }

    fn draw_tooltip(
        &self,
        frame: &mut canvas::Frame,
        theme: &Theme,
        cursor: Point,
        target: &Target<'_>,
    ) {
        draw_tooltip(frame, theme, cursor, target.duration);
    }
}

impl canvas::Program<Message> for Waveform<'_> {
    /// Whether the pointer was over the viewer at the last event, so one
    /// more redraw can clear the tooltip once it leaves.
    type State = bool;

    fn update(
        &self,
        hovering: &mut bool,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let over = cursor.is_over(bounds);
                if over || *hovering {
                    *hovering = over;
                    return Some(canvas::Action::request_redraw());
                }
                None
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let target = self.target.as_ref().filter(|target| target.can_seek)?;
                let active = target.active?;
                let point = cursor.position_in(bounds)?;
                let seconds = seek_seconds(
                    f64::from(point.x / bounds.width),
                    target.range,
                    active.duration,
                )?;
                Some(canvas::Action::publish(Message::Seek(target.id, seconds)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _hovering: &bool,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let key = self.cache_key();
        {
            let mut last = self.viewer.key.borrow_mut();
            if *last != key {
                self.viewer.cache.clear();
                *last = key;
            }
        }

        let waveform = self
            .viewer
            .cache
            .draw(renderer, bounds.size(), |frame| self.draw_waveform(frame));

        let mut overlay = canvas::Frame::new(renderer, bounds.size());
        self.draw_overlay(&mut overlay, theme, cursor.position_in(bounds));

        vec![waveform, overlay.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_range_is_the_whole_file_without_start_and_end() {
        assert_eq!(TimeRange::new(Some(10.0), None, None), TimeRange::FULL);
    }

    #[test]
    fn the_range_follows_start_and_end_time() {
        let range = TimeRange::new(Some(10.0), Some(2.0), Some(8.0));
        assert_eq!(range, TimeRange { start: 0.2, end: 0.8 });
        assert!((range.delta() - 0.6).abs() < 1e-12);
    }

    #[test]
    fn a_zero_end_time_means_the_end_of_the_file() {
        let range = TimeRange::new(Some(10.0), Some(1.0), Some(0.0));
        assert_eq!(range.end, 1.0);
    }

    #[test]
    fn an_unknown_duration_does_not_divide_by_zero() {
        let range = TimeRange::new(None, None, None);
        assert!(range.start.is_finite() && range.end.is_finite());
    }

    #[test]
    fn the_gain_normalizes_to_the_peak() {
        assert_eq!(gain_db(Some(-6.0)), 6.0);
        assert_eq!(gain_db(Some(0.0)), 0.0);
        assert_eq!(gain_db(None), 0.0);
    }

    #[test]
    fn each_column_takes_the_loudest_sample_it_covers() {
        let waveform = [0.1, 0.5, 0.2, 0.9];
        assert_eq!(column_peaks(&waveform, 2), vec![0.5, 0.9]);
        assert_eq!(column_peaks(&waveform, 4), waveform.to_vec());
    }

    #[test]
    fn more_columns_than_samples_repeat_samples_and_run_out() {
        let columns = column_peaks(&[0.4, 0.8], 4);
        assert_eq!(columns, vec![0.4, 0.4, 0.8, 0.8]);
    }

    #[test]
    fn no_waveform_gives_silent_columns() {
        assert_eq!(column_peaks(&[], 3), vec![0.0; 3]);
        assert!(column_peaks(&[1.0], 0).is_empty());
    }

    #[test]
    fn a_click_inside_the_range_maps_to_the_cue_position() {
        let range = TimeRange { start: 0.2, end: 0.8 };
        let seconds = seek_seconds(0.5, range, 30.0).unwrap();
        assert!((seconds - 15.0).abs() < 1e-9);
    }

    #[test]
    fn a_click_outside_the_range_does_not_seek() {
        let range = TimeRange { start: 0.2, end: 0.8 };
        assert_eq!(seek_seconds(0.1, range, 30.0), None);
        assert_eq!(seek_seconds(0.9, range, 30.0), None);
    }

    #[test]
    fn an_empty_range_never_seeks() {
        let range = TimeRange { start: 0.5, end: 0.5 };
        assert_eq!(seek_seconds(0.5, range, 30.0), None);
    }
}
