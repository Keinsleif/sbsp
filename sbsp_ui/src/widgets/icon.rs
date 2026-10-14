// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Icons: the same Material Design Icons the Vue frontend drew with
//! `@mdi/js` (each icon is the 24x24 path data from that package, v7.4.47,
//! Apache-2.0), rendered as small inline SVGs and tinted from the theme.
//!
//! SVG rather than an icon font so the paths stay identical to the
//! original and need no font asset or subsetting step; add an icon by
//! pasting its `mdi*` path from `@mdi/js` into [`Icon`] and
//! [`Icon::svg`].

use iced::widget::{Svg, svg};
use iced::{Color, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Play,
    Pause,
    Stop,
    Upload,
    Repeat,
    ArrowRightBold,
    MenuDown,
    MenuRight,
    VolumeHigh,
    TimerSandEmpty,
    ChartBellCurveCumulative,
    PlayCircleOutline,
    StopCircleOutline,
    PauseCircleOutline,
    UploadCircleOutline,
    Group,
    ArrowCollapseDown,
    ArrowExpandDown,
    AlphaEBoxOutline,
    AlphaRBoxOutline,
    ChevronDoubleDown,
    TimerSand,
}

macro_rules! svg_icon {
    ($path:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d=""#,
            $path,
            r#""/></svg>"#
        )
        .as_bytes()
    };
}

impl Icon {
    fn svg(self) -> &'static [u8] {
        match self {
        Icon::Play => svg_icon!("M8,5.14V19.14L19,12.14L8,5.14Z"),
        Icon::Pause => svg_icon!("M14,19H18V5H14M6,19H10V5H6V19Z"),
        Icon::Stop => svg_icon!("M18,18H6V6H18V18Z"),
        Icon::Upload => svg_icon!("M9,16V10H5L12,3L19,10H15V16H9M5,20V18H19V20H5Z"),
        Icon::Repeat => svg_icon!("M17,17H7V14L3,18L7,22V19H19V13H17M7,7H17V10L21,6L17,2V5H5V11H7V7Z"),
        Icon::ArrowRightBold => svg_icon!("M4,15V9H12V4.16L19.84,12L12,19.84V15H4Z"),
        Icon::MenuDown => svg_icon!("M7,10L12,15L17,10H7Z"),
        Icon::MenuRight => svg_icon!("M10,17L15,12L10,7V17Z"),
        Icon::VolumeHigh => svg_icon!("M14,3.23V5.29C16.89,6.15 19,8.83 19,12C19,15.17 16.89,17.84 14,18.7V20.77C18,19.86 21,16.28 21,12C21,7.72 18,4.14 14,3.23M16.5,12C16.5,10.23 15.5,8.71 14,7.97V16C15.5,15.29 16.5,13.76 16.5,12M3,9V15H7L12,20V4L7,9H3Z"),
        Icon::TimerSandEmpty => svg_icon!("M6,2V8H6V8L10,12L6,16V16H6V22H18V16H18V16L14,12L18,8V8H18V2H6M16,16.5V20H8V16.5L12,12.5L16,16.5M12,11.5L8,7.5V4H16V7.5L12,11.5Z"),
        Icon::ChartBellCurveCumulative => svg_icon!("M4 19V20H22V22H2V2H4V17C7 17 10 15 12.1 11.4C15.1 6.4 18.4 4 22 4V6C19.2 6 16.5 8.1 13.9 12.5C11.3 16.6 7.7 19 4 19Z"),
        Icon::PlayCircleOutline => svg_icon!("M12,20C7.59,20 4,16.41 4,12C4,7.59 7.59,4 12,4C16.41,4 20,7.59 20,12C20,16.41 16.41,20 12,20M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2M10,16.5L16,12L10,7.5V16.5Z"),
        Icon::StopCircleOutline => svg_icon!("M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2M12,4C16.41,4 20,7.59 20,12C20,16.41 16.41,20 12,20C7.59,20 4,16.41 4,12C4,7.59 7.59,4 12,4M9,9V15H15V9"),
        Icon::PauseCircleOutline => svg_icon!("M13,16V8H15V16H13M9,16V8H11V16H9M12,2A10,10 0 0,1 22,12A10,10 0 0,1 12,22A10,10 0 0,1 2,12A10,10 0 0,1 12,2M12,4A8,8 0 0,0 4,12A8,8 0 0,0 12,20A8,8 0 0,0 20,12A8,8 0 0,0 12,4Z"),
        Icon::UploadCircleOutline => svg_icon!("M8 17V15H16V17H8M16 10L12 6L8 10H10.5V14H13.5V10H16M12 2C17.5 2 22 6.5 22 12C22 17.5 17.5 22 12 22C6.5 22 2 17.5 2 12C2 6.5 6.5 2 12 2M12 4C7.58 4 4 7.58 4 12C4 16.42 7.58 20 12 20C16.42 20 20 16.42 20 12C20 7.58 16.42 4 12 4Z"),
        Icon::Group => svg_icon!("M1,1V5H2V19H1V23H5V22H19V23H23V19H22V5H23V1H19V2H5V1M5,4H19V5H20V19H19V20H5V19H4V5H5M6,6V14H9V18H18V9H14V6M8,8H12V12H8M14,11H16V16H11V14H14"),
        Icon::ArrowCollapseDown => svg_icon!("M19.92,12.08L12,20L4.08,12.08L5.5,10.67L11,16.17V2H13V16.17L18.5,10.66L19.92,12.08M12,20H2V22H22V20H12Z"),
        Icon::ArrowExpandDown => svg_icon!("M22,4V2H2V4H11V18.17L5.5,12.67L4.08,14.08L12,22L19.92,14.08L18.5,12.67L13,18.17V4H22Z"),
        Icon::AlphaEBoxOutline => svg_icon!("M9,7H15V9H11V11H15V13H11V15H15V17H9V7M3,5A2,2 0 0,1 5,3H19A2,2 0 0,1 21,5V19A2,2 0 0,1 19,21H5C3.89,21 3,20.1 3,19V5M5,5V19H19V5H5Z"),
        Icon::AlphaRBoxOutline => svg_icon!("M9,7H13A2,2 0 0,1 15,9V11C15,11.84 14.5,12.55 13.76,12.85L15,17H13L11.8,13H11V17H9V7M11,9V11H13V9H11M5,3H19A2,2 0 0,1 21,5V19A2,2 0 0,1 19,21H5A2,2 0 0,1 3,19V5A2,2 0 0,1 5,3M5,5V19H19V5H5Z"),
        Icon::ChevronDoubleDown => svg_icon!("M16.59,5.59L18,7L12,13L6,7L7.41,5.59L12,10.17L16.59,5.59M16.59,11.59L18,13L12,19L6,13L7.41,11.59L12,16.17L16.59,11.59Z"),
        Icon::TimerSand => svg_icon!("M6,2H18V8H18V8L14,12L18,16V16H18V22H6V16H6V16L10,12L6,8V8H6V2M16,16.5L12,12.5L8,16.5V20H16V16.5M12,11.5L16,7.5V4H8V7.5L12,11.5M10,6H14V6.75L12,8.75L10,6.75V6Z"),
        }
    }
}

/// Which theme color an icon is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tone {
    /// The surrounding text color.
    #[default]
    Text,
    Muted,
    Primary,
    Success,
    Warning,
    Danger,
}

impl Tone {
    fn color(self, theme: &Theme) -> Color {
        let palette = theme.extended_palette();
        match self {
            Tone::Text => palette.background.base.text,
            Tone::Muted => palette.background.strong.color,
            Tone::Primary => palette.primary.base.color,
            Tone::Success => palette.success.base.color,
            Tone::Warning => palette.warning.base.color,
            Tone::Danger => palette.danger.base.color,
        }
    }
}

pub fn icon<'a>(icon: Icon, size: f32) -> Svg<'a> {
    toned(icon, size, Tone::Text)
}

pub fn toned<'a>(icon: Icon, size: f32, tone: Tone) -> Svg<'a> {
    colored(icon, size, move |theme| tone.color(theme))
}

/// For an icon whose color depends on more than a fixed tone, e.g. text
/// on a button that changes background when active.
pub fn colored<'a>(
    icon: Icon,
    size: f32,
    color: impl Fn(&Theme) -> Color + 'a,
) -> Svg<'a> {
    svg(svg::Handle::from_memory(icon.svg()))
        .width(size)
        .height(size)
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(color(theme)),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_a_wellformed_svg_with_a_path() {
        for icon in [
            Icon::Play,
            Icon::Pause,
            Icon::Stop,
            Icon::Upload,
            Icon::Repeat,
            Icon::ArrowRightBold,
            Icon::MenuDown,
            Icon::MenuRight,
            Icon::VolumeHigh,
            Icon::TimerSandEmpty,
            Icon::ChartBellCurveCumulative,
            Icon::PlayCircleOutline,
            Icon::StopCircleOutline,
            Icon::PauseCircleOutline,
            Icon::UploadCircleOutline,
            Icon::Group,
            Icon::ArrowCollapseDown,
            Icon::ArrowExpandDown,
            Icon::AlphaEBoxOutline,
            Icon::AlphaRBoxOutline,
            Icon::ChevronDoubleDown,
            Icon::TimerSand,
        ] {
            let text = std::str::from_utf8(icon.svg()).expect("svg is utf-8");
            assert!(text.starts_with("<svg"), "{icon:?}");
            assert!(text.ends_with("</svg>"), "{icon:?}");
            assert!(text.contains(" d=\"M"), "{icon:?}: path data missing");
        }
    }
}
