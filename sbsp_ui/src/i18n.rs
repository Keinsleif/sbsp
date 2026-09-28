// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::sync::LazyLock;

use i18n_embed::{
    LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;
use unic_langid::LanguageIdentifier;

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
