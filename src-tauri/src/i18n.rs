//! The app's language: which languages have a catalog, the setting's
//! default from the system language, and translating the native menus'
//! text.  The window translates its own text with Lingui; see docs/i18n.md.

use std::collections::{BTreeMap, HashMap};

use aximar_core::message::Message;
use fluent_langneg::{negotiate_languages, LanguageIdentifier, NegotiationStrategy};

mod embedded {
    // `CATALOGS`: each `locales/<lang>.po` but `pseudo`, from build.rs
    include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));
}

/// The source language, which every message falls back to
pub const ENGLISH: &str = "en";

/// The languages with a catalog, English first
pub fn languages() -> Vec<&'static str> {
    let mut langs = vec![ENGLISH];
    langs.extend(
        embedded::CATALOGS
            .iter()
            .map(|(lang, _)| *lang)
            .filter(|lang| *lang != ENGLISH),
    );
    langs
}

/// The system's language, such as `es-CO`
pub fn system_language() -> Option<String> {
    sys_locale::get_locale()
}

/// The language to use until the user chooses one: the system's, when a
/// catalog covers it (`es_CO` takes `es`), and English otherwise.
pub fn default_language(system: Option<&str>, available: &[&str]) -> String {
    let Some(requested) = system.and_then(parse_language) else {
        return ENGLISH.to_string();
    };
    let available: Vec<LanguageIdentifier> =
        available.iter().filter_map(|l| parse_language(l)).collect();
    let english = parse_language(ENGLISH).unwrap();
    negotiate_languages(
        &[requested],
        &available,
        Some(&english),
        NegotiationStrategy::Lookup,
    )
    .first()
    .map(|l| l.to_string())
    .unwrap_or_else(|| ENGLISH.to_string())
}

/// A language tag from a POSIX or BCP 47 name: `es_CO.UTF-8` or `es-CO`
fn parse_language(name: &str) -> Option<LanguageIdentifier> {
    let name = name.split(['.', '@']).next()?.replace('_', "-");
    name.parse().ok()
}

/// One language's translations of Aximar's Rust-side text
#[derive(Default)]
pub struct Translations {
    /// By context (empty for none) and English template
    entries: HashMap<(String, String), String>,
}

impl Translations {
    /// The translations built into the app for `language`; none for
    /// English or a language without a catalog
    pub fn for_language(language: &str) -> Self {
        embedded::CATALOGS
            .iter()
            .find(|(lang, _)| *lang == language && language != ENGLISH)
            .map(|(_, po)| Self::parse(po))
            .unwrap_or_default()
    }

    /// The translated entries of a catalog's text.  Fuzzy entries are
    /// guesses waiting for review, so they don't count.
    pub fn parse(po: &str) -> Self {
        let Ok(file) = rspolib::pofile(po) else {
            return Self::default();
        };
        let entries = file
            .entries
            .into_iter()
            .filter(|e| !e.obsolete && !e.fuzzy() && e.msgid_plural.is_none())
            .filter_map(|e| {
                let translation = e.msgstr.filter(|t| !t.is_empty())?;
                Some(((e.msgctxt.unwrap_or_default(), e.msgid), translation))
            })
            .collect();
        Translations { entries }
    }

    /// The message in this language, or its English text when the catalog
    /// doesn't translate it
    pub fn translate(&self, message: &Message) -> String {
        let key = (message.id.clone(), message.template.to_string());
        match self.entries.get(&key) {
            Some(translation) => fill(translation, &message.params),
            None => message.text.clone(),
        }
    }
}

/// A translation with each `{name}` replaced by its parameter
fn fill(template: &str, params: &BTreeMap<String, String>) -> String {
    let mut out = template.to_string();
    for (name, value) in params {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    /// A message built by hand: `msg!` here would put test text in the
    /// catalog, since extraction reads `src/`
    fn message(id: &str, template: &'static str, params: &[(&str, &str)]) -> Message {
        let params: BTreeMap<String, String> = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Message {
            id: id.to_string(),
            text: fill(template, &params),
            template: Cow::Borrowed(template),
            params,
        }
    }

    #[test]
    fn a_regional_system_language_takes_its_catalog() {
        let available = ["en", "es"];
        assert_eq!(default_language(Some("es_CO"), &available), "es");
        assert_eq!(default_language(Some("es-CO"), &available), "es");
        assert_eq!(default_language(Some("es_CO.UTF-8"), &available), "es");
    }

    #[test]
    fn a_system_language_without_a_catalog_is_english() {
        let available = ["en", "es"];
        assert_eq!(default_language(Some("ja"), &available), "en");
        assert_eq!(default_language(Some("ja_JP"), &available), "en");
        assert_eq!(default_language(None, &available), "en");
        assert_eq!(default_language(Some(""), &available), "en");
        assert_eq!(default_language(Some("es"), &["en"]), "en");
    }

    #[test]
    fn english_comes_first() {
        assert_eq!(languages()[0], "en");
        assert!(!languages().contains(&"pseudo"));
    }

    #[test]
    fn spanish_is_offered() {
        assert!(languages().contains(&"es"));
    }

    const ES: &str = r#"msgid ""
msgstr ""
"Language: es\n"
"Content-Type: text/plain; charset=UTF-8\n"

msgid "File"
msgstr "Archivo"

msgctxt "error.arg_count.explanation.too_few"
msgid "Too few arguments were passed to {func}."
msgstr "Se pasaron muy pocos argumentos a {func}."

#, fuzzy
msgid "Save"
msgstr "Salvar"

msgid "Edit"
msgstr ""
"#;

    #[test]
    fn translations_fill_parameters_and_fall_back_to_english() {
        let es = Translations::parse(ES);
        assert_eq!(es.translate(&message("", "File", &[])), "Archivo");
        let m = message(
            "error.arg_count.explanation.too_few",
            "Too few arguments were passed to {func}.",
            &[("func", "diff")],
        );
        assert_eq!(es.translate(&m), "Se pasaron muy pocos argumentos a diff.");
        // Fuzzy, untranslated and missing entries are English
        assert_eq!(es.translate(&message("", "Save", &[])), "Save");
        assert_eq!(es.translate(&message("", "Edit", &[])), "Edit");
        assert_eq!(es.translate(&message("", "Window", &[])), "Window");
        // The context counts
        assert_eq!(es.translate(&message("menu.other", "File", &[])), "File");
    }
}
