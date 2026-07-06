//! Fluent-based localization (`en` / `fr`).
//!
//! The active locale is **`DEFAULT_LANGUAGE`** in `.env` (see `.env.example`). Only `en` and `fr`
//! are supported (`locales/*.ftl`). If the variable is unset, empty, or invalid, English is used
//! (with a log warning when invalid).

use std::str::FromStr;

use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentResource, FluentValue};
use log::warn;
use unic_langid::LanguageIdentifier;

const EN_FTL: &str = include_str!("../../locales/en.ftl");
const FR_FTL: &str = include_str!("../../locales/fr.ftl");

/// Shared localization state: one Fluent bundle per supported locale.
pub struct L10n {
    /// Resolved once at startup from `DEFAULT_LANGUAGE` in the environment.
    locale: LanguageIdentifier,
    en: FluentBundle<FluentResource>,
    fr: FluentBundle<FluentResource>,
}

impl L10n {
    /// Builds bundles and reads the `DEFAULT_LANGUAGE` env var (`.env` loaded via `dotenvy` in `main`).
    pub fn new() -> Self {
        let locale = default_from_env();

        let en_res = FluentResource::try_new(EN_FTL.to_string()).expect("parse en.ftl");
        let fr_res = FluentResource::try_new(FR_FTL.to_string()).expect("parse fr.ftl");

        // `new_concurrent` so `L10n` can live in `web::Data` across Actix worker threads.
        let mut en = FluentBundle::new_concurrent(vec![lid("en")]);
        en.add_resource(en_res)
            .expect("add en resource");

        let mut fr = FluentBundle::new_concurrent(vec![lid("fr")]);
        fr.add_resource(fr_res)
            .expect("add fr resource");

        Self {
            locale,
            en,
            fr,
        }
    }

    /// Locale from `DEFAULT_LANGUAGE` (`.env`); see module docs.
    pub fn locale(&self) -> LanguageIdentifier {
        self.locale.clone()
    }

    /// Selects the bundle for API responses (`fr` → French; anything else → English).
    fn bundle(&self, locale: &LanguageIdentifier) -> &FluentBundle<FluentResource> {
        if locale.language == "fr" {
            &self.fr
        } else {
            &self.en
        }
    }

    /// Formats a Fluent message id using the bundle for `locale`.
    pub fn format_message(
        &self,
        locale: &LanguageIdentifier,
        id: &str,
        args: Option<FluentArgs>,
    ) -> String {
        let bundle = self.bundle(locale);
        let Some(msg) = bundle.get_message(id) else {
            return id.to_string();
        };
        let Some(pattern) = msg.value() else {
            return id.to_string();
        };
        let mut errors = vec![];
        let out = bundle.format_pattern(&pattern, args.as_ref(), &mut errors);
        if !errors.is_empty() {
            log::warn!("fluent format errors for {id}: {errors:?}");
        }
        out.into_owned()
    }

    /// Helper for a single string variable (e.g. transaction number in 404 text).
    pub fn format_with_n(
        &self,
        locale: &LanguageIdentifier,
        id: &str,
        n: u32,
    ) -> String {
        let mut args = FluentArgs::new();
        args.set("n", FluentValue::from(i64::from(n)));
        self.format_message(locale, id, Some(args))
    }
}

/// Parses a fixed tag we control (`en` / `fr`).
fn lid(tag: &str) -> LanguageIdentifier {
    LanguageIdentifier::from_str(tag).unwrap_or_else(|_| panic!("invalid language tag {tag:?}"))
}

/// Reads `DEFAULT_LANGUAGE` from the environment; empty or invalid → English with a warning.
fn default_from_env() -> LanguageIdentifier {
    const KEY: &str = "DEFAULT_LANGUAGE";
    const SUPPORTED: &[&str] = &["en", "fr"];

    let raw = std::env::var(KEY).unwrap_or_default();
    let tag = raw.trim().to_lowercase();
    if tag.is_empty() {
        return lid("en");
    }
    if SUPPORTED.contains(&tag.as_str()) {
        return lid(&tag);
    }
    warn!("{KEY}={raw:?} is not one of {SUPPORTED:?}; using \"en\"");
    lid("en")
}
