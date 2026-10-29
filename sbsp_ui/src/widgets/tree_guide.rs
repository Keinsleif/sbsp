// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The thin lines at the left of a cue list row that show how deep in
//! groups it sits: a vertical line for each enclosing group, and at the
//! innermost one a short branch out to the row (a corner at the closing
//! "end" row).

use iced::widget::{Space, canvas};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

/// How far each level indents.
pub const LEVEL_WIDTH: f32 = 26.0;

/// Where the line of the innermost level sits within its column.
const LINE_X: f32 = 10.0;
/// How far the branch reaches towards the row.
const BRANCH: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A cue: the innermost line goes on to the rows below.
    Item,
    /// The closing row of a group: the line ends in a corner.
    End,
}

/// The guide for a row `level` groups deep. Nothing (and no width) at the
/// top level.
pub fn view<'a, Message: 'a>(level: usize, kind: Kind, height: f32) -> Element<'a, Message> {
    if level == 0 {
        return Space::new().into();
    }
    canvas(Guide { level, kind })
        .width(Length::Fixed(LINE_X + (level - 1) as f32 * LEVEL_WIDTH + BRANCH))
        .height(Length::Fixed(height))
        .into()
}

struct Guide {
    level: usize,
    kind: Kind,
}

/// The lines as rectangles `(x, y, width, height)`, for a row `height`
/// tall: full-height lines for the enclosing levels, then the innermost
/// one (full height, or down to the middle for an end row) and its branch.
fn lines(level: usize, kind: Kind, height: f32) -> Vec<(f32, f32, f32, f32)> {
    if level == 0 {
        return Vec::new();
    }
    let mut lines: Vec<_> = (0..level - 1)
        .map(|i| (LINE_X + i as f32 * LEVEL_WIDTH, 0.0, 1.0, height))
        .collect();

    let x = LINE_X + (level - 1) as f32 * LEVEL_WIDTH;
    let middle = (height / 2.0).floor();
    let reach = match kind {
        Kind::Item => height,
        Kind::End => middle,
    };
    lines.push((x, 0.0, 1.0, reach));
    lines.push((x, middle, BRANCH, 1.0));
    lines
}

impl<Message> canvas::Program<Message> for Guide {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let color = theme.extended_palette().background.strong.color;

        for (x, y, width, height) in lines(self.level, self.kind, bounds.height) {
            frame.fill_rectangle(Point::new(x, y), Size::new(width, height), color);
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_top_level_has_no_lines() {
        assert!(lines(0, Kind::Item, 30.0).is_empty());
    }

    #[test]
    fn one_level_deep_is_a_line_and_a_branch() {
        assert_eq!(
            lines(1, Kind::Item, 30.0),
            vec![(8.0, 0.0, 1.0, 30.0), (8.0, 15.0, 8.0, 1.0)]
        );
    }

    #[test]
    fn each_enclosing_level_adds_a_full_height_line() {
        let result = lines(3, Kind::Item, 30.0);
        assert_eq!(
            result,
            vec![
                (8.0, 0.0, 1.0, 30.0),
                (24.0, 0.0, 1.0, 30.0),
                (40.0, 0.0, 1.0, 30.0),
                (40.0, 15.0, 8.0, 1.0),
            ]
        );
    }

    #[test]
    fn an_end_row_turns_a_corner_at_the_middle() {
        assert_eq!(
            lines(2, Kind::End, 30.0),
            vec![
                (8.0, 0.0, 1.0, 30.0),
                (24.0, 0.0, 1.0, 15.0),
                (24.0, 15.0, 8.0, 1.0),
            ]
        );
    }

    #[test]
    fn the_guide_is_as_wide_as_its_levels() {
        // The branch ends where the next level's line would be, so the
        // lines of consecutive levels never overlap.
        assert_eq!(LINE_X + BRANCH, LEVEL_WIDTH);
    }
}
