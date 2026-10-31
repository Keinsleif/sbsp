// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The waveform of an audio cue with its start / end markers and volume
//! envelope, all draggable. Ported from `WaveformEditor.vue`.
//!
//! - Drag the blue lines: the cue's start and end.
//! - With the envelope shown: drag a bar up or down to change its volume,
//!   its round handles sideways to change its extent; click a bar or
//!   handle to select the segment; double click to add or split a segment.
//! - While the cue plays nothing can be edited; a click seeks instead.
//!
//! The widget keeps no copy of the cue: a drag publishes the whole new
//! envelope and range with every move ([`Message::Changed`]), the owner
//! keeps them as its working state and the next `view` draws them, and
//! the end of a drag publishes [`Message::Commit`].

use std::cell::RefCell;
use std::path::PathBuf;
use crate::time::{Duration, Instant};

use iced::mouse;
use iced::widget::canvas::{self, Cache, Canvas, LineDash, Path, Stroke};

use crate::widgets::held_size::HeldSize;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use crate::domain::envelope::{self, Segment};
use crate::features::viewer::{self, TimeRange};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// The envelope and range as they are while dragging.
    Changed {
        segments: Vec<Segment>,
        range: TimeRange,
    },
    /// The drag ended: save what `Changed` showed.
    Commit,
    /// A click on a segment.
    Select(usize),
    /// A double click, at this position within the played part (0..1).
    Split(f64),
    /// A click while playing: seek to this many seconds.
    Seek(f64),
}

/// The waveform geometry, kept between frames; rebuilt only when what it
/// was built from changes.
#[derive(Default)]
pub struct Surface {
    cache: Cache,
    key: RefCell<Option<Key>>,
    held: HeldSize,
}

#[derive(Debug, Clone, PartialEq)]
struct Key {
    path: PathBuf,
    samples: usize,
    gain_bits: Option<u32>,
}

pub struct Props<'a> {
    pub surface: &'a Surface,
    pub path: PathBuf,
    pub waveform: Option<&'a [f32]>,
    /// dB the waveform is scaled by (`None`: as it is).
    pub gain: Option<f32>,
    pub segments: Vec<Segment>,
    pub range: TimeRange,
    pub envelope_visible: bool,
    pub selected: Option<usize>,
    /// The file's duration, for the tooltip.
    pub duration: Option<f64>,
    /// How far through the played part a playing cue is (0..1).
    pub playhead: Option<f64>,
    /// The playing cue's duration: set while it plays, when a click seeks
    /// and nothing can be edited.
    pub seek_duration: Option<f64>,
    pub height: f32,
    /// Keep the waveform as drawn (the window is being resized).
    pub frozen: bool,
}

pub fn view(props: Props<'_>) -> Element<'_, Message> {
    let height = props.height;
    Canvas::new(Editor { props })
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .into()
}

// ---------------------------------------------------------------------
// Geometry

const HANDLE_RADIUS: f32 = 8.0;
const LOCKED_RADIUS: f32 = 3.0;
/// Half the thickness of the grab area around a bar or a range marker.
const GRAB: f32 = 10.0;
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const DOUBLE_CLICK_DISTANCE: f32 = 6.0;
/// How far the pointer must go before a press becomes a drag.
const DRAG_THRESHOLD: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Volume,
    Start,
    End,
    RangeStart,
    RangeEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Drag(Kind, usize),
    /// The first segment's start and the last one's end cannot move, but
    /// still hide what is below them.
    Locked,
}

/// Where a position within the played part (0..1) is, in pixels.
fn px(range: TimeRange, width: f32, x: f64) -> f32 {
    ((range.start + x * range.delta()) * f64::from(width)) as f32
}

/// The other way round; `None` for an empty range.
fn within_range(range: TimeRange, width: f32, px: f32) -> Option<f64> {
    (range.delta() > 0.0 && width > 0.0)
        .then(|| (f64::from(px / width) - range.start) / range.delta())
}

fn start_marker(range: TimeRange, width: f32) -> f32 {
    range.start as f32 * (width - 1.0)
}

fn end_marker(range: TimeRange, width: f32) -> f32 {
    range.end as f32 * (width - 1.0) - 1.0
}

/// What is under `point`, topmost first (the order the original draws in,
/// reversed): later segments over earlier ones, a segment's end handle
/// over its start handle over its bar, all of them over the range markers.
fn hit(
    point: Point,
    size: Size,
    segments: &[Segment],
    range: TimeRange,
    envelope_visible: bool,
) -> Option<Hit> {
    if envelope_visible {
        let last = segments.len().saturating_sub(1);
        for (i, segment) in segments.iter().enumerate().rev() {
            let y = envelope::volume_to_y(segment.volume, f64::from(size.height)) as f32;
            let start = px(range, size.width, segment.start);
            let end = px(range, size.width, segment.end);
            let near = |x: f32, radius: f32| {
                (point.x - x).powi(2) + (point.y - y).powi(2) <= radius * radius
            };

            if i == last {
                if near(end, LOCKED_RADIUS) {
                    return Some(Hit::Locked);
                }
            } else if near(end, HANDLE_RADIUS) {
                return Some(Hit::Drag(Kind::End, i));
            }
            if i == 0 {
                if near(start, LOCKED_RADIUS) {
                    return Some(Hit::Locked);
                }
            } else if near(start, HANDLE_RADIUS) {
                return Some(Hit::Drag(Kind::Start, i));
            }
            if point.x >= start && point.x <= end && (point.y - y).abs() <= GRAB {
                return Some(Hit::Drag(Kind::Volume, i));
            }
        }
    }

    if (point.x - end_marker(range, size.width)).abs() <= GRAB {
        return Some(Hit::Drag(Kind::RangeEnd, 0));
    }
    if (point.x - start_marker(range, size.width)).abs() <= GRAB {
        return Some(Hit::Drag(Kind::RangeStart, 0));
    }
    None
}

/// The envelope and range after the pointer, at `point` (inside the
/// widget), has dragged `kind` / `index`.
fn dragged(
    kind: Kind,
    index: usize,
    point: Point,
    size: Size,
    segments: &[Segment],
    range: TimeRange,
) -> Option<(Vec<Segment>, TimeRange)> {
    let width = f64::from(size.width);
    match kind {
        Kind::Volume => {
            let volume = envelope::y_to_volume(f64::from(point.y), f64::from(size.height));
            Some((envelope::set_volume(segments, index, volume), range))
        }
        Kind::Start => {
            let x = within_range(range, size.width, point.x)?;
            Some((envelope::move_start(segments, index, x), range))
        }
        Kind::End => {
            let x = within_range(range, size.width, point.x)?;
            Some((envelope::move_end(segments, index, x), range))
        }
        Kind::RangeStart => {
            let start = (f64::from(point.x) / width).clamp(0.0, range.end);
            Some((segments.to_vec(), TimeRange { start, ..range }))
        }
        Kind::RangeEnd => {
            let end = (f64::from(point.x) / width).clamp(range.start, 1.0);
            Some((segments.to_vec(), TimeRange { end, ..range }))
        }
    }
}

// ---------------------------------------------------------------------
// The canvas program

struct Editor<'a> {
    props: Props<'a>,
}

#[derive(Default)]
pub struct State {
    drag: Option<Drag>,
    hovering: bool,
    last_press: Option<(Instant, Point)>,
}

struct Drag {
    kind: Kind,
    index: usize,
    /// Where the pointer went down, within the widget.
    origin: Point,
    moved: bool,
}

impl Editor<'_> {
    fn editable(&self) -> bool {
        self.props.seek_duration.is_none()
    }

    fn key(&self) -> Key {
        Key {
            path: self.props.path.clone(),
            samples: self.props.waveform.map_or(0, <[f32]>::len),
            gain_bits: self.props.gain.map(f32::to_bits),
        }
    }

    fn draw_envelope(&self, frame: &mut canvas::Frame, theme: &Theme) {
        let Props {
            segments,
            range,
            selected,
            ..
        } = &self.props;
        if segments.is_empty() {
            return;
        }
        let size = frame.size();
        let palette = theme.extended_palette();
        let primary = palette.primary.base.color;
        let orange = Color::from_rgb8(249, 115, 22);
        let editable = self.editable();
        let height = f64::from(size.height);
        let y = |segment: &Segment| envelope::volume_to_y(segment.volume, height) as f32;
        let x = |value: f64| px(*range, size.width, value);

        // The area under the envelope.
        let area = Path::new(|builder| {
            for (i, segment) in segments.iter().enumerate() {
                let point = Point::new(x(segment.start), y(segment));
                if i == 0 {
                    builder.move_to(point);
                } else {
                    builder.line_to(point);
                }
                builder.line_to(Point::new(x(segment.end), y(segment)));
            }
            builder.line_to(Point::new(x(1.0), size.height));
            builder.line_to(Point::new(x(0.0), size.height));
            builder.close();
        });
        frame.fill(&area, primary.scale_alpha(0.2));

        // The ramps over the gaps between segments. One stroke each: a dash
        // pattern runs on across the pieces of a single path and would join
        // them with slanted lines.
        for pair in segments.windows(2) {
            let ramp = Path::line(
                Point::new(x(pair[0].end), y(&pair[0])),
                Point::new(x(pair[1].start), y(&pair[1])),
            );
            frame.stroke(
                &ramp,
                Stroke {
                    line_dash: LineDash {
                        segments: &[8.0, 4.0],
                        offset: 0,
                    },
                    ..Stroke::default().with_color(primary).with_width(3.0)
                },
            );
        }

        let last = segments.len() - 1;
        for (i, segment) in segments.iter().enumerate() {
            let chosen = *selected == Some(i);
            let fill = if chosen {
                orange
            } else if editable {
                primary
            } else {
                palette.background.strong.color
            };
            let (left, right, top) = (x(segment.start), x(segment.end), y(segment));

            frame.fill_rectangle(Point::new(left, top - 2.0), Size::new(right - left, 4.0), fill);
            frame.stroke_rectangle(
                Point::new(left, top - 2.0),
                Size::new(right - left, 4.0),
                Stroke::default()
                    .with_color(palette.background.strong.color)
                    .with_width(1.0),
            );

            let handle = |at: f32, locked: bool, frame: &mut canvas::Frame| {
                let locked = locked || !editable;
                let circle = Path::circle(
                    Point::new(at, top),
                    if locked { LOCKED_RADIUS } else { HANDLE_RADIUS },
                );
                let color = if locked {
                    Color::from_rgb8(0x44, 0x44, 0x44)
                } else if chosen {
                    orange
                } else {
                    primary
                };
                frame.fill(&circle, color);
                if !locked {
                    frame.stroke(
                        &circle,
                        Stroke::default()
                            .with_color(palette.background.strong.color)
                            .with_width(2.0),
                    );
                }
            };
            handle(left, i == 0, frame);
            handle(right, i == last, frame);
        }
    }

    fn draw_overlay(&self, frame: &mut canvas::Frame, theme: &Theme, cursor: Option<Point>) {
        let size = frame.size();
        let range = self.props.range;
        let blue = Color::from_rgb8(0, 0, 255);

        for at in [start_marker(range, size.width), end_marker(range, size.width)] {
            frame.fill_rectangle(Point::new(at, 0.0), Size::new(2.0, size.height), blue);
        }

        if self.props.envelope_visible {
            self.draw_envelope(frame, theme);
        }

        if let Some(ratio) = self.props.playhead {
            let at = px(range, size.width - 1.0, ratio);
            frame.fill_rectangle(
                Point::new(at, 0.0),
                Size::new(2.0, size.height),
                Color::from_rgb8(255, 255, 0),
            );
        }

        if let Some(point) = cursor {
            viewer::draw_tooltip(frame, theme, point, self.props.duration);
        }
    }
}

impl canvas::Program<Message> for Editor<'_> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let size = bounds.size();

        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(drag) = &mut state.drag {
                    let position = cursor.position()?;
                    let point = Point::new(
                        (position.x - bounds.x).clamp(0.0, size.width),
                        (position.y - bounds.y).clamp(0.0, size.height),
                    );
                    // A click is not a drag: ignore the pointer's jitter.
                    if !drag.moved
                        && (point.x - drag.origin.x).hypot(point.y - drag.origin.y) < DRAG_THRESHOLD
                    {
                        return None;
                    }
                    drag.moved = true;
                    let (segments, range) = dragged(
                        drag.kind,
                        drag.index,
                        point,
                        size,
                        &self.props.segments,
                        self.props.range,
                    )?;
                    return Some(canvas::Action::publish(Message::Changed { segments, range }));
                }

                let over = cursor.is_over(bounds);
                if over || state.hovering {
                    state.hovering = over;
                    return Some(canvas::Action::request_redraw());
                }
                None
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let point = cursor.position_in(bounds)?;

                // While it plays, a click seeks.
                if let Some(duration) = self.props.seek_duration {
                    let position = within_range(self.props.range, size.width, point.x)?;
                    return (position > 0.0 && position < 1.0)
                        .then(|| canvas::Action::publish(Message::Seek(position * duration)).and_capture());
                }

                let now = Instant::now();
                let double = state.last_press.is_some_and(|(at, previous)| {
                    now.saturating_duration_since(at) <= DOUBLE_CLICK
                        && (previous.x - point.x).hypot(previous.y - point.y)
                            <= DOUBLE_CLICK_DISTANCE
                });
                state.last_press = if double { None } else { Some((now, point)) };
                if double {
                    state.drag = None;
                    if !self.props.envelope_visible {
                        return None;
                    }
                    let x = within_range(self.props.range, size.width, point.x)?;
                    return Some(canvas::Action::publish(Message::Split(x)).and_capture());
                }

                match hit(
                    point,
                    size,
                    &self.props.segments,
                    self.props.range,
                    self.props.envelope_visible,
                )? {
                    Hit::Locked => Some(canvas::Action::request_redraw().and_capture()),
                    Hit::Drag(kind, index) => {
                        state.drag = Some(Drag {
                            kind,
                            index,
                            origin: point,
                            moved: false,
                        });
                        Some(canvas::Action::request_redraw().and_capture())
                    }
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let drag = state.drag.take()?;
                let message = if drag.moved {
                    Message::Commit
                } else if matches!(drag.kind, Kind::Volume | Kind::Start | Kind::End) {
                    Message::Select(drag.index)
                } else {
                    return None;
                };
                Some(canvas::Action::publish(message))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let key = self.key();
        {
            let mut last = self.props.surface.key.borrow_mut();
            if last.as_ref() != Some(&key) {
                self.props.surface.cache.clear();
                *last = Some(key);
            }
        }

        let waveform = self
            .props
            .surface
            .cache
            .draw(
                renderer,
                self.props.surface.held.pick(bounds.size(), self.props.frozen),
                |frame| {
                    viewer::draw_peaks(frame, self.props.waveform, self.props.gain);
                },
            );

        let mut overlay = canvas::Frame::new(renderer, bounds.size());
        self.draw_overlay(&mut overlay, theme, cursor.position_in(bounds));

        vec![waveform, overlay.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if !self.editable() {
            return mouse::Interaction::default();
        }
        if let Some(drag) = &state.drag {
            return interaction(drag.kind);
        }
        cursor
            .position_in(bounds)
            .and_then(|point| {
                hit(
                    point,
                    bounds.size(),
                    &self.props.segments,
                    self.props.range,
                    self.props.envelope_visible,
                )
            })
            .map_or_else(mouse::Interaction::default, |hit| match hit {
                Hit::Drag(kind, _) => interaction(kind),
                Hit::Locked => mouse::Interaction::default(),
            })
    }
}

fn interaction(kind: Kind) -> mouse::Interaction {
    match kind {
        Kind::Volume => mouse::Interaction::ResizingVertically,
        Kind::Start | Kind::End | Kind::RangeStart | Kind::RangeEnd => {
            mouse::Interaction::ResizingHorizontally
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Size = Size {
        width: 400.0,
        height: 100.0,
    };
    const FULL: TimeRange = TimeRange { start: 0.0, end: 1.0 };

    fn seg(start: f64, end: f64, volume: f64) -> Segment {
        Segment { start, end, volume }
    }

    fn two() -> Vec<Segment> {
        vec![seg(0.0, 0.4, 0.0), seg(0.6, 1.0, -6.0)]
    }

    #[test]
    fn positions_follow_the_played_part() {
        let range = TimeRange { start: 0.25, end: 0.75 };
        assert_eq!(px(range, 400.0, 0.0), 100.0);
        assert_eq!(px(range, 400.0, 1.0), 300.0);
        assert_eq!(within_range(range, 400.0, 200.0), Some(0.5));
        assert_eq!(within_range(TimeRange { start: 0.5, end: 0.5 }, 400.0, 200.0), None);
    }

    #[test]
    fn a_handle_a_bar_and_a_marker_are_hit_in_that_order() {
        let segments = two();
        let at = |x: f32, y: f32| hit(Point::new(x, y), SIZE, &segments, FULL, true);

        // The first segment's bar is at the top (0 dB): y = 0.
        assert_eq!(at(100.0, 5.0), Some(Hit::Drag(Kind::Volume, 0)));
        // The first start (x = 0) and the last end (x = 400) are locked.
        assert_eq!(at(1.0, 1.0), Some(Hit::Locked));
        // The first segment's end handle, round, at the end of the bar.
        assert_eq!(at(160.0, 2.0), Some(Hit::Drag(Kind::End, 0)));
        // The second segment's start handle.
        let y = envelope::volume_to_y(-6.0, 100.0) as f32;
        assert_eq!(at(240.0, y), Some(Hit::Drag(Kind::Start, 1)));
        // The second bar, between its handles.
        assert_eq!(at(320.0, y + 3.0), Some(Hit::Drag(Kind::Volume, 1)));
        // Nothing in the middle of the waveform.
        assert_eq!(at(200.0, 80.0), None);
    }

    #[test]
    fn the_range_markers_can_be_grabbed_with_and_without_the_envelope() {
        let range = TimeRange { start: 0.25, end: 0.75 };
        let start = start_marker(range, SIZE.width);
        let end = end_marker(range, SIZE.width);

        assert_eq!(
            hit(Point::new(start + 4.0, 50.0), SIZE, &[], range, false),
            Some(Hit::Drag(Kind::RangeStart, 0))
        );
        assert_eq!(
            hit(Point::new(end - 4.0, 50.0), SIZE, &[], range, true),
            Some(Hit::Drag(Kind::RangeEnd, 0))
        );
        // A hidden envelope cannot be grabbed.
        assert_eq!(hit(Point::new(250.0, 2.0), SIZE, &two(), FULL, false), None);
    }

    #[test]
    fn dragging_changes_the_envelope_or_the_range() {
        let segments = two();

        let (louder, _) = dragged(Kind::Volume, 1, Point::new(300.0, 0.0), SIZE, &segments, FULL).unwrap();
        assert_eq!(louder[1].volume, 0.0);

        let (moved, _) = dragged(Kind::End, 0, Point::new(100.0, 0.0), SIZE, &segments, FULL).unwrap();
        assert!((moved[0].end - 0.25).abs() < 1e-9);

        let (_, range) = dragged(Kind::RangeStart, 0, Point::new(100.0, 0.0), SIZE, &segments, FULL).unwrap();
        assert_eq!(range, TimeRange { start: 0.25, end: 1.0 });
        // The start marker cannot pass the end marker.
        let narrow = TimeRange { start: 0.0, end: 0.5 };
        let (_, range) = dragged(Kind::RangeStart, 0, Point::new(300.0, 0.0), SIZE, &segments, narrow).unwrap();
        assert_eq!(range.start, 0.5);

        // An empty range cannot place segments.
        let empty = TimeRange { start: 0.5, end: 0.5 };
        assert!(dragged(Kind::End, 0, Point::new(100.0, 0.0), SIZE, &segments, empty).is_none());
    }
}
