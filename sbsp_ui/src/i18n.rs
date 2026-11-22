// SPDX-License-Identifier: GPL-3.0-or-later
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
    loader
        .load_languages(&Localizations, &[Language::default().langid()])
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

/// The Languages being requested by the operating system/environment.
pub fn system_language() -> Option<Language> {
    #[cfg(not(target_arch = "wasm32"))]
    let lang = i18n_embed::DesktopLanguageRequester::requested_languages();
    #[cfg(target_arch = "wasm32")]
    let lang = i18n_embed::WebLanguageRequester::requested_languages();
    lang.iter().find_map(|li| Language::from_setting(Some(li)))
}

/// Switch the active language. The fallback language is always kept.
pub fn select(language: Option<Language>) {
    let languages = Vec::from_iter(language.or(system_language()).map(|l| l.langid()));
    if let Err(e) = LOADER.load_languages(&Localizations, &languages) {
        log::error!("failed to load language {languages:?}: {e}");
    }
}

/// The app's supported UI languages. A small fixed enum, not a wrapper
/// around an arbitrary [`LanguageIdentifier`]: every caller that needs to
/// show or cycle through "the language" wants a closed set with a
/// display label per entry, not an open-ended identifier.
///
/// Stored in the settings' `appearance.language` (see
/// [`Language::from_setting`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Ja,
    En,
}

impl Language {
    /// Fallback from other unsupported languages. Not default language.
    /// Same as `i18n.toml`'s `fallback_language`
    pub const FALLBACK_LANGUAGE: Self = Language::En;
    
    pub fn toggle(self) -> Self {
        match self {
            Language::Ja => Language::En,
            Language::En => Language::Ja,
        }
    }

    pub fn from_setting(setting: Option<&LanguageIdentifier>) -> Option<Self> {
        setting.map(|lang| {
            if lang.matches(&Self::Ja.langid(), false, true) {
                Self::Ja
            } else if lang.matches(&Self::En.langid(), false, true) {
                Self::En
            } else {
                Self::FALLBACK_LANGUAGE
            }
        })
    }

    pub fn langid(self) -> LanguageIdentifier {
        match self {
            Language::Ja => langid!("ja"),
            Language::En => langid!("en"),
        }
    }

    /// The label this language's own name should render as -- i.e. always
    /// "English"/"日本語", never translated through the *currently active*
    /// language (a language picker that only showed entries in whichever
    /// language is presently selected would make every other option
    /// unreadable to someone who can't read that one).
    pub fn label(self) -> &'static str {
        match self {
            Language::Ja => "日本語",
            Language::En => "English",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_languid_correctly_parsed() {
        assert_eq!(
            Language::from_setting(Some(&langid!("ja"))),
            Some(Language::Ja)
        );
        assert_eq!(
            Language::from_setting(Some(&langid!("en"))),
            Some(Language::En)
        );
        assert_eq!(Language::from_setting(Some(&langid!("ja-JP"))), Some(Language::Ja));
        assert_eq!(
            Language::from_setting(Some(&langid!("fr"))),
            None
        );
    }

    #[test]
    fn what_is_stored_is_read_back() {
        for language in [Language::En, Language::Ja] {
            assert_eq!(
                Language::from_setting(Some(&language.langid())),
                Some(language)
            );
        }
    }
}
