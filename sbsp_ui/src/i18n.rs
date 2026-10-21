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

/// The language the system (the OS on desktop, the browser on the web)
/// asks for first, if it says.
pub fn system_language() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    let requested = i18n_embed::DesktopLanguageRequester::requested_languages();
    #[cfg(target_arch = "wasm32")]
    let requested = i18n_embed::WebLanguageRequester::requested_languages();
    requested.first().map(ToString::to_string)
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
/// Stored in the settings' `appearance.language` (see
/// [`Language::from_setting`]).
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

    /// The language a stored setting names: `"ja"` (or `"ja-JP"`, ...) is
    /// Japanese and any other value is English. With no setting the
    /// system's language decides (`system`, e.g. `"ja-JP"`), English when
    /// that is unknown too.
    pub fn from_setting(setting: Option<&str>, system: Option<&str>) -> Self {
        match setting.or(system) {
            Some(tag) if tag.to_lowercase().starts_with("ja") => Language::Ja,
            _ => Language::En,
        }
    }

    /// What goes into the settings for this language.
    pub fn setting(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Ja => "ja",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_setting_wins_over_the_system() {
        assert_eq!(Language::from_setting(Some("ja"), Some("en-US")), Language::Ja);
        assert_eq!(Language::from_setting(Some("en"), Some("ja-JP")), Language::En);
        assert_eq!(Language::from_setting(Some("ja-JP"), None), Language::Ja);
        assert_eq!(Language::from_setting(Some("fr"), Some("ja-JP")), Language::En);
    }

    #[test]
    fn without_a_setting_the_system_decides() {
        assert_eq!(Language::from_setting(None, Some("ja-JP")), Language::Ja);
        assert_eq!(Language::from_setting(None, Some("en-GB")), Language::En);
        assert_eq!(Language::from_setting(None, None), Language::En);
    }

    #[test]
    fn what_is_stored_is_read_back() {
        for language in [Language::En, Language::Ja] {
            assert_eq!(Language::from_setting(Some(language.setting()), None), language);
        }
    }
}
