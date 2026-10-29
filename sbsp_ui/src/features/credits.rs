// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The credits dialog: the app itself, the icons and fonts it ships, and
//! the licenses of the libraries it is built from. Ported from
//! `CreditsDialog.vue`, which rendered `THIRD_PARTY_NOTICES.md` as one
//! long page.
//!
//! The notices run to hundreds of kilobytes, so they are split into one
//! entry per license and only the opened entry's text is laid out.

use std::sync::OnceLock;

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::fl;
use crate::theme::fonts;

/// What `cargo about` wrote (the generated file is committed). The web
/// build does not carry it (it would double the download); its credits
/// come with Phase 9.
#[cfg(not(target_arch = "wasm32"))]
const NOTICES: &str = include_str!("../../../THIRD_PARTY_NOTICES.md");
#[cfg(target_arch = "wasm32")]
const NOTICES: &str = "";

/// SIL Open Font License 1.1, with the fonts' copyright notice.
const FONT_LICENSE: &str = include_str!("../../assets/fonts/OFL.txt");

const ICON_NOTICE: &str = "Material Design Icons\n\
Copyright (c) Pictogrammers (https://pictogrammers.com/library/mdi/)\n\
\n\
Licensed under the Apache License, Version 2.0\n\
(https://www.apache.org/licenses/LICENSE-2.0). The license text is the\n\
\"Apache License 2.0\" entry below.";

#[derive(Debug, Clone, PartialEq)]
pub struct Crate {
    pub name: String,
    pub version: String,
    pub url: String,
}

/// One license and the crates that use it.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub name: String,
    pub used_by: Vec<Crate>,
    pub text: String,
}

/// `&quot;` and the like, which the template's escaping left in.
fn unescape(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#x3D;", "=")
        .replace("&#x60;", "`")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// `[name version]( url )`, as the template writes a crate.
fn parse_crate(line: &str) -> Option<Crate> {
    let rest = line.trim().strip_prefix("- [")?;
    let (label, url) = rest.split_once("](")?;
    let (name, version) = label.rsplit_once(' ').unwrap_or((label, ""));
    Some(Crate {
        name: name.to_owned(),
        version: version.to_owned(),
        url: url.trim_end_matches(')').trim().to_owned(),
    })
}

/// The licenses in the generated file: a `### name` heading, the crates
/// using it, then the text in a fence. Anything after the first top-level
/// heading that follows them (the old JavaScript part) is left out.
pub fn parse(markdown: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut in_fence = false;
    let mut started = false;

    for line in markdown.lines() {
        if in_fence {
            if line.trim_end() == "```" {
                in_fence = false;
            } else if let Some(section) = sections.last_mut() {
                section.text.push_str(&unescape(line));
                section.text.push('\n');
            }
            continue;
        }
        if line.starts_with("```") {
            in_fence = started;
            continue;
        }
        if line.starts_with("# ") && started {
            break;
        }
        if let Some(name) = line.strip_prefix("### ") {
            started = true;
            sections.push(Section {
                name: unescape(name.trim()),
                used_by: Vec::new(),
                text: String::new(),
            });
        } else if let (Some(section), Some(krate)) = (sections.last_mut(), parse_crate(line)) {
            section.used_by.push(krate);
        }
    }
    sections
}

/// The entries the dialog lists: what is shipped besides code first.
fn entries() -> &'static [Section] {
    static ENTRIES: OnceLock<Vec<Section>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let mut entries = vec![
            Section {
                name: "Material Design Icons".to_owned(),
                used_by: Vec::new(),
                text: ICON_NOTICE.to_owned(),
            },
            Section {
                name: "BIZ UD Gothic / BIZ UDP Gothic".to_owned(),
                used_by: Vec::new(),
                text: FONT_LICENSE.to_owned(),
            },
        ];
        entries.extend(parse(NOTICES));
        entries
    })
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Open or close the entry at this index.
    Toggle(usize),
    Close,
}

#[derive(Debug, Default)]
pub struct Dialog {
    open: Option<usize>,
}

impl Dialog {
    pub fn update(&mut self, message: &Message) {
        if let Message::Toggle(index) = message {
            self.open = (self.open != Some(*index)).then_some(*index);
        }
    }

    pub fn open(&self) -> Option<usize> {
        self.open
    }
}

fn entry_view<'a>(index: usize, section: &'a Section, open: bool) -> Element<'a, Message> {
    let count = if section.used_by.is_empty() {
        String::new()
    } else {
        format!(" ({})", section.used_by.len())
    };
    let marker = if open { "v" } else { ">" };
    let header = button(
        row![text(marker).font(fonts::MONO), text(format!("{}{count}", section.name))]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([4, 8])
    .style(if open { button::secondary } else { button::text })
    .on_press(Message::Toggle(index));

    if !open {
        return header.into();
    }

    let mut body = column![].spacing(8).padding([4, 16]);
    if !section.used_by.is_empty() {
        let crates: Vec<String> = section
            .used_by
            .iter()
            .map(|krate| format!("{} {}", krate.name, krate.version))
            .collect();
        body = body.push(text(crates.join(", ")).size(12));
    }
    body = body.push(text(section.text.as_str()).size(11).font(fonts::MONO));
    column![header, body].into()
}

pub fn view<'a>(dialog: &Dialog) -> Element<'a, Message> {
    let mut list = column![].spacing(2);
    for (index, section) in entries().iter().enumerate() {
        list = list.push(entry_view(index, section, dialog.open == Some(index)));
    }

    container(
        column![
            text("SBS Player").size(20),
            text(format!(
                "{} {} - GNU General Public License v3.0 or later",
                fl!("dialog-credits-version"),
                env!("CARGO_PKG_VERSION")
            ))
            .size(12),
            text(fl!("dialog-credits-third-party")).size(14),
            scrollable(list).height(Length::Fill),
            row![
                Space::new().width(Length::Fill),
                button(text(fl!("general-close")))
                    .style(button::secondary)
                    .on_press(Message::Close),
            ],
        ]
        .spacing(10),
    )
    .width(800)
    .height(560)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# THIRD PARTY NOTICES\n\
\n\
## Rust Dependencies\n\
\n\
### Apache License 2.0\n\
\n\
- Used by\n\
\n\
    - [num_threads 0.1.7]( https://github.com/jhpratt/num_threads )\n\
    - [other 2.0.0]( https://crates.io/crates/other )\n\
\n\
```text\n\
&quot;License&quot; shall mean\n\
# not a heading, inside the text\n\
### nor this\n\
```\n\
\n\
---\n\
\n\
### MIT License\n\
\n\
- Used by\n\
\n\
    - [a 1.0.0]( https://example.com/a )\n\
\n\
```text\n\
Permission is hereby granted\n\
```\n\
\n\
# JavaScript Dependencies\n\
\n\
### ignored\n\
\n\
```text\n\
nope\n\
```\n";

    #[test]
    fn a_license_has_its_name_crates_and_text() {
        let sections = parse(SAMPLE);
        assert_eq!(sections.len(), 2);

        assert_eq!(sections[0].name, "Apache License 2.0");
        assert_eq!(sections[0].used_by.len(), 2);
        assert_eq!(
            sections[0].used_by[0],
            Crate {
                name: "num_threads".to_owned(),
                version: "0.1.7".to_owned(),
                url: "https://github.com/jhpratt/num_threads".to_owned(),
            }
        );
        assert_eq!(
            sections[0].text,
            "\"License\" shall mean\n# not a heading, inside the text\n### nor this\n"
        );
        assert_eq!(sections[1].name, "MIT License");
        assert_eq!(sections[1].text, "Permission is hereby granted\n");
    }

    #[test]
    fn nothing_after_the_next_top_level_heading_is_listed() {
        assert!(parse(SAMPLE).iter().all(|section| section.name != "ignored"));
    }

    #[test]
    fn the_committed_notices_parse_into_licenses() {
        let sections = parse(NOTICES);
        assert!(!sections.is_empty());
        for section in &sections {
            assert!(!section.name.is_empty());
            assert!(!section.text.is_empty(), "{} has no text", section.name);
        }
        assert!(sections.iter().any(|s| s.name.contains("MIT")));
    }

    #[test]
    fn the_entries_start_with_the_icons_and_the_fonts() {
        let entries = entries();
        assert_eq!(entries[0].name, "Material Design Icons");
        assert!(entries[1].text.contains("SIL OPEN FONT LICENSE"));
    }

    #[test]
    fn one_entry_is_open_at_a_time() {
        let mut dialog = Dialog::default();
        dialog.update(&Message::Toggle(2));
        assert_eq!(dialog.open(), Some(2));
        dialog.update(&Message::Toggle(3));
        assert_eq!(dialog.open(), Some(3));
        dialog.update(&Message::Toggle(3));
        assert_eq!(dialog.open(), None);
    }
}
