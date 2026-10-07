// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::sync::LazyLock;

use i18n_embed::{
    LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;
use unic_langid::{LanguageIdentifier, langid};

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    let fallback = loader.fallback_language().clone();
    loader
        .load_languages(&Localizations, &[fallback])
        .expect("failed to load the fallback language");
    loader
});

/// Translate a message. Message ids and arguments are checked at compile time.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id)
    }};
    ($message_id:literal, $($args:tt)*) => {{
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id, $($args)*)
    }};
}

/// Switch the active language. The fallback language is always kept.
pub fn select(language: &LanguageIdentifier) {
    let fallback = LOADER.fallback_language().clone();
    let mut languages = vec![language.clone()];
    if *language != fallback {
        languages.push(fallback);
    }
    if let Err(e) = LOADER.load_languages(&Localizations, &languages) {
        log::error!("failed to load language {language}: {e}");
    }
}

/// The app's supported UI languages. A small fixed enum, not a wrapper
/// around an arbitrary [`LanguageIdentifier`]: every caller that needs to
/// show or cycle through "the language" (a menu item today; eventually the
/// settings dialog's language picker, Phase 7) wants a closed set with a
/// display label per entry, not an open-ended identifier.
///
/// Not persisted anywhere yet -- switching is in-memory only for this
/// patch, matching `ThemeMode` before it got a settings-backed home. A
/// `GlobalHostSettings.appearance` field to store this in already exists;
/// wiring it up is left for whichever patch first saves settings for real.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    En,
    Ja,
}

impl Language {
    pub fn toggle(self) -> Self {
        match self {
            Language::En => Language::Ja,
            Language::Ja => Language::En,
        }
    }

    pub fn langid(self) -> LanguageIdentifier {
        match self {
            Language::En => langid!("en"),
            Language::Ja => langid!("ja"),
        }
    }

    /// The label this language's own name should render as -- i.e. always
    /// "English"/"日本語", never translated through the *currently active*
    /// language (a language picker that only showed entries in whichever
    /// language is presently selected would make every other option
    /// unreadable to someone who can't read that one).
    pub fn label(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Ja => "日本語",
        }
    }
}
