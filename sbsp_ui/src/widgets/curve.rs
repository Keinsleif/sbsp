// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! A small picture of a fade curve, next to the fade fields. Ported from
//! `CurveViewer.vue`: a grid with the curve over it; `falling` draws the
//! curve of a fade *out* (the same curve from the top left down).

use iced::widget::canvas::{self, Canvas, Path, Stroke};
use iced::{Element, Length, Point, Rectangle, Renderer, Theme, mouse};

use crate::domain::easing::Curve;

/// The original's `w-16`.
pub const WIDTH: f32 = 64.0;
pub const HEIGHT: f32 = 48.0;

/// Without a curve (the fade is off) only the grid is drawn.
pub fn curve_view<'a, Message: 'a>(curve: Option<Curve>, falling: bool) -> Element<'a, Message> {
    Canvas::new(CurveProgram { curve, falling })
        .width(Length::Fixed(WIDTH))
        .height(Length::Fixed(HEIGHT))
        .into()
}

struct CurveProgram {
    curve: Option<Curve>,
    falling: bool,
}

/// How many segments the curve is drawn with (the original's 100).
const STEPS: u32 = 100;

impl<Message> canvas::Program<Message> for CurveProgram {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let palette = theme.extended_palette();
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        // One pixel in, so the outline is not cut off at the edge.
        let (left, top) = (1.0, 1.0);
        let width = (bounds.width - 2.0).max(1.0);
        let height = (bounds.height - 2.0).max(1.0);

        let grid = Stroke::default()
            .with_color(palette.background.strong.color.scale_alpha(0.5))
            .with_width(1.0);
        for i in 1..10 {
            let fraction = i as f32 / 10.0;
            frame.stroke(
                &Path::line(
                    Point::new(left + width * fraction, top),
                    Point::new(left + width * fraction, top + height),
                ),
                grid,
            );
            frame.stroke(
                &Path::line(
                    Point::new(left, top + height * fraction),
                    Point::new(left + width, top + height * fraction),
                ),
                grid,
            );
        }

        frame.stroke_rectangle(
            Point::new(left, top),
            iced::Size::new(width, height),
            Stroke::default()
                .with_color(palette.background.strong.color)
                .with_width(1.0),
        );

        if let Some(curve) = self.curve {
            let falling = self.falling;
            let path = Path::new(|builder| {
                for i in 0..=STEPS {
                    let x = f64::from(i) / f64::from(STEPS);
                    let y = point_height(curve.factor(x), falling);
                    let point = Point::new(left + width * x as f32, top + height * y);
                    if i == 0 {
                        builder.move_to(point);
                    } else {
                        builder.line_to(point);
                    }
                }
            });
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(palette.primary.base.color)
                    .with_width(2.0),
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Where on the picture (0 = top, 1 = bottom) a fade is, given how far it
/// has got (`factor`, 0..=1): a fade in climbs from the bottom, a fade out
/// falls from the top.
pub fn point_height(factor: f64, falling: bool) -> f32 {
    if falling {
        factor as f32
    } else {
        1.0 - factor as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fade_in_starts_at_the_bottom_and_a_fade_out_at_the_top() {
        assert_eq!(point_height(0.0, false), 1.0);
        assert_eq!(point_height(1.0, false), 0.0);
        assert_eq!(point_height(0.0, true), 0.0);
        assert_eq!(point_height(1.0, true), 1.0);
    }
}
