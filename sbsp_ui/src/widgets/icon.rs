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
    DockRight,
    DockBottom,
    ChevronDown,
    ChevronUp,
    CheckboxMarked,
    CheckboxBlankOutline,
    FileMusic,
    Plus,
    Minus,
    TrashCan,
    Eye,
    EyeOff,
    SkipNext,
    SkipPrevious,
    ContentCopy,
    ContentCut,
    ContentPaste,
    ContentDuplicate,
    DragVertical,
    Cog,
    OpenInNew,
    Server,
    Close,
    Alert,
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
        Icon::Close => svg_icon!("M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"),
        Icon::Alert => svg_icon!("M13,14H11V10H13M13,18H11V16H13M1,21H23L12,2L1,21Z"),
        Icon::OpenInNew => svg_icon!("M14,3V5H17.59L7.76,14.83L9.17,16.24L19,6.41V10H21V3M19,19H5V5H12V3H5C3.89,3 3,3.9 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V12H19V19Z"),
        Icon::Server => svg_icon!("M4,1H20A1,1 0 0,1 21,2V6A1,1 0 0,1 20,7H4A1,1 0 0,1 3,6V2A1,1 0 0,1 4,1M4,9H20A1,1 0 0,1 21,10V14A1,1 0 0,1 20,15H4A1,1 0 0,1 3,14V10A1,1 0 0,1 4,9M4,17H20A1,1 0 0,1 21,18V22A1,1 0 0,1 20,23H4A1,1 0 0,1 3,22V18A1,1 0 0,1 4,17M9,5H10V3H9V5M9,13H10V11H9V13M9,21H10V19H9V21M5,2V4H7V2H5M5,10V12H7V10H5M5,18V20H7V18H5Z"),
        Icon::Cog => svg_icon!("M12,15.5A3.5,3.5 0 0,1 8.5,12A3.5,3.5 0 0,1 12,8.5A3.5,3.5 0 0,1 15.5,12A3.5,3.5 0 0,1 12,15.5M19.43,12.97C19.47,12.65 19.5,12.33 19.5,12C19.5,11.67 19.47,11.34 19.43,11L21.54,9.37C21.73,9.22 21.78,8.95 21.66,8.73L19.66,5.27C19.54,5.05 19.27,4.96 19.05,5.05L16.56,6.05C16.04,5.66 15.5,5.32 14.87,5.07L14.5,2.42C14.46,2.18 14.25,2 14,2H10C9.75,2 9.54,2.18 9.5,2.42L9.13,5.07C8.5,5.32 7.96,5.66 7.44,6.05L4.95,5.05C4.73,4.96 4.46,5.05 4.34,5.27L2.34,8.73C2.21,8.95 2.27,9.22 2.46,9.37L4.57,11C4.53,11.34 4.5,11.67 4.5,12C4.5,12.33 4.53,12.65 4.57,12.97L2.46,14.63C2.27,14.78 2.21,15.05 2.34,15.27L4.34,18.73C4.46,18.95 4.73,19.03 4.95,18.95L7.44,17.94C7.96,18.34 8.5,18.68 9.13,18.93L9.5,21.58C9.54,21.82 9.75,22 10,22H14C14.25,22 14.46,21.82 14.5,21.58L14.87,18.93C15.5,18.67 16.04,18.34 16.56,17.94L19.05,18.95C19.27,19.03 19.54,18.95 19.66,18.73L21.66,15.27C21.78,15.05 21.73,14.78 21.54,14.63L19.43,12.97Z"),
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
        Icon::DockRight => svg_icon!("M20 4H4A2 2 0 0 0 2 6V18A2 2 0 0 0 4 20H20A2 2 0 0 0 22 18V6A2 2 0 0 0 20 4M15 18H4V6H15Z"),
        Icon::DockBottom => svg_icon!("M20 4H4A2 2 0 0 0 2 6V18A2 2 0 0 0 4 20H20A2 2 0 0 0 22 18V6A2 2 0 0 0 20 4M20 13H4V6H20Z"),
        Icon::ChevronDown => svg_icon!("M7.41,8.58L12,13.17L16.59,8.58L18,10L12,16L6,10L7.41,8.58Z"),
        Icon::ChevronUp => svg_icon!("M7.41,15.41L12,10.83L16.59,15.41L18,14L12,8L6,14L7.41,15.41Z"),
        Icon::CheckboxMarked => svg_icon!("M10,17L5,12L6.41,10.58L10,14.17L17.59,6.58L19,8M19,3H5C3.89,3 3,3.89 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V5C21,3.89 20.1,3 19,3Z"),
        Icon::FileMusic => svg_icon!("M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M13,13H11V18A2,2 0 0,1 9,20A2,2 0 0,1 7,18A2,2 0 0,1 9,16C9.4,16 9.7,16.1 10,16.3V11H13V13M13,9V3.5L18.5,9H13Z"),
        Icon::CheckboxBlankOutline => svg_icon!("M19,3H5C3.89,3 3,3.89 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V5C21,3.89 20.1,3 19,3M19,5V19H5V5H19Z"),
        Icon::Plus => svg_icon!("M19,13H13V19H11V13H5V11H11V5H13V11H19V13Z"),
        Icon::Minus => svg_icon!("M19,13H5V11H19V13Z"),
        Icon::TrashCan => svg_icon!("M9,3V4H4V6H5V19A2,2 0 0,0 7,21H17A2,2 0 0,0 19,19V6H20V4H15V3H9M9,8H11V17H9V8M13,8H15V17H13V8Z"),
        Icon::Eye => svg_icon!("M12,9A3,3 0 0,0 9,12A3,3 0 0,0 12,15A3,3 0 0,0 15,12A3,3 0 0,0 12,9M12,17A5,5 0 0,1 7,12A5,5 0 0,1 12,7A5,5 0 0,1 17,12A5,5 0 0,1 12,17M12,4.5C7,4.5 2.73,7.61 1,12C2.73,16.39 7,19.5 12,19.5C17,19.5 21.27,16.39 23,12C21.27,7.61 17,4.5 12,4.5Z"),
        Icon::EyeOff => svg_icon!("M11.83,9L15,12.16C15,12.11 15,12.05 15,12A3,3 0 0,0 12,9C11.94,9 11.89,9 11.83,9M7.53,9.8L9.08,11.35C9.03,11.56 9,11.77 9,12A3,3 0 0,0 12,15C12.22,15 12.44,14.97 12.65,14.92L14.2,16.47C13.53,16.8 12.79,17 12,17A5,5 0 0,1 7,12C7,11.21 7.2,10.47 7.53,9.8M2,4.27L4.28,6.55L4.73,7C3.08,8.3 1.78,10 1,12C2.73,16.39 7,19.5 12,19.5C13.55,19.5 15.03,19.2 16.38,18.66L16.81,19.08L19.73,22L21,20.73L3.27,3M12,7A5,5 0 0,1 17,12C17,12.64 16.87,13.26 16.64,13.82L19.57,16.75C21.07,15.5 22.27,13.86 23,12C21.27,7.61 17,4.5 12,4.5C10.6,4.5 9.26,4.75 8,5.2L10.17,7.35C10.74,7.13 11.35,7 12,7Z"),
        Icon::SkipNext => svg_icon!("M16,18H18V6H16M6,18L14.5,12L6,6V18Z"),
        Icon::SkipPrevious => svg_icon!("M6,18V6H8V18H6M9.5,12L18,6V18L9.5,12Z"),
        Icon::ContentCopy => svg_icon!("M19,21H8V7H19M19,5H8A2,2 0 0,0 6,7V21A2,2 0 0,0 8,23H19A2,2 0 0,0 21,21V7A2,2 0 0,0 19,5M16,1H4A2,2 0 0,0 2,3V17H4V3H16V1Z"),
        Icon::ContentCut => svg_icon!("M19,3L13,9L15,11L22,4V3M12,12.5A0.5,0.5 0 0,1 11.5,12A0.5,0.5 0 0,1 12,11.5A0.5,0.5 0 0,1 12.5,12A0.5,0.5 0 0,1 12,12.5M6,20A2,2 0 0,1 4,18C4,16.89 4.9,16 6,16A2,2 0 0,1 8,18C8,19.11 7.1,20 6,20M6,8A2,2 0 0,1 4,6C4,4.89 4.9,4 6,4A2,2 0 0,1 8,6C8,7.11 7.1,8 6,8M9.64,7.64C9.87,7.14 10,6.59 10,6A4,4 0 0,0 6,2A4,4 0 0,0 2,6A4,4 0 0,0 6,10C6.59,10 7.14,9.87 7.64,9.64L10,12L7.64,14.36C7.14,14.13 6.59,14 6,14A4,4 0 0,0 2,18A4,4 0 0,0 6,22A4,4 0 0,0 10,18C10,17.41 9.87,16.86 9.64,16.36L12,14L19,21H22V20L9.64,7.64Z"),
        Icon::ContentPaste => svg_icon!("M19,20H5V4H7V7H17V4H19M12,2A1,1 0 0,1 13,3A1,1 0 0,1 12,4A1,1 0 0,1 11,3A1,1 0 0,1 12,2M19,2H14.82C14.4,0.84 13.3,0 12,0C10.7,0 9.6,0.84 9.18,2H5A2,2 0 0,0 3,4V20A2,2 0 0,0 5,22H19A2,2 0 0,0 21,20V4A2,2 0 0,0 19,2Z"),
        Icon::ContentDuplicate => svg_icon!("M11,17H4A2,2 0 0,1 2,15V3A2,2 0 0,1 4,1H16V3H4V15H11V13L15,16L11,19V17M19,21V7H8V13H6V7A2,2 0 0,1 8,5H19A2,2 0 0,1 21,7V21A2,2 0 0,1 19,23H8A2,2 0 0,1 6,21V19H8V21H19Z"),
        Icon::DragVertical => svg_icon!("M9,3H11V5H9V3M13,3H15V5H13V3M9,7H11V9H9V7M13,7H15V9H13V7M9,11H11V13H9V11M13,11H15V13H13V11M9,15H11V17H9V15M13,15H15V17H13V15M9,19H11V21H9V19M13,19H15V21H13V19Z"),
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
            Icon::DockRight,
            Icon::DockBottom,
            Icon::ChevronDown,
            Icon::ChevronUp,
        ] {
            let text = std::str::from_utf8(icon.svg()).expect("svg is utf-8");
            assert!(text.starts_with("<svg"), "{icon:?}");
            assert!(text.ends_with("</svg>"), "{icon:?}");
            assert!(text.contains(" d=\"M"), "{icon:?}: path data missing");
        }
    }
}
