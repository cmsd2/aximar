//! Help summaries in other languages, from Maxima's manual in each
//! (`core-doc-summaries.<lang>.json`, made by `catalog-gen summaries`).
//!
//! The English doc index holds everything else about a symbol; these hold
//! only its one-paragraph summary.  Front ends show a symbol's summary in
//! the user's language when the manual in that language covers it, and in
//! English otherwise.  The core itself stays in English: the search index
//! covers both languages, but results carry the English.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

#[derive(Deserialize)]
struct SummariesFile {
    summaries: HashMap<String, String>,
}

/// The languages with summaries, and their files
const FILES: &[(&str, &str)] = &[("es", include_str!("core-doc-summaries.es.json"))];

/// Each language's summaries, by lower-case symbol name
static SUMMARIES: LazyLock<HashMap<&'static str, HashMap<String, String>>> = LazyLock::new(|| {
    FILES
        .iter()
        .filter_map(|(lang, json)| {
            let file: SummariesFile = serde_json::from_str(json)
                .map_err(|e| eprintln!("[doc-index] Bad summaries for {lang}: {e}"))
                .ok()?;
            let by_name = file
                .summaries
                .into_iter()
                .map(|(name, summary)| (name.to_lowercase(), summary))
                .collect();
            Some((*lang, by_name))
        })
        .collect()
});

/// The languages with summaries, such as `es`
pub fn languages() -> Vec<&'static str> {
    FILES.iter().map(|(lang, _)| *lang).collect()
}

/// A symbol's summary in `language`, if Maxima's manual in it covers it
pub fn summary(language: &str, name: &str) -> Option<&'static str> {
    SUMMARIES
        .get(language)?
        .get(&name.to_lowercase())
        .map(String::as_str)
}

/// A symbol's summary in `language` when there is one, else `english`
pub fn localised<'a>(language: Option<&str>, name: &str, english: &'a str) -> Cow<'a, str> {
    match language.and_then(|lang| summary(lang, name)) {
        Some(translated) => Cow::Borrowed(translated),
        None => Cow::Borrowed(english),
    }
}

/// Every translation of a symbol's summary, for the search index
pub fn translations(name: &str) -> Vec<&'static str> {
    FILES
        .iter()
        .filter_map(|(lang, _)| summary(lang, name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_symbol_has_its_spanish_summary() {
        let integrate = summary("es", "integrate").expect("Spanish summary for integrate");
        assert!(
            integrate.starts_with("Calcula simbólicamente la integral"),
            "{integrate}"
        );
        assert_eq!(
            localised(Some("es"), "integrate", "Attempts to symbolically compute"),
            integrate
        );
    }

    #[test]
    fn a_symbol_the_spanish_manual_lacks_is_in_english() {
        assert_eq!(summary("es", "no_such_symbol"), None);
        assert_eq!(
            localised(Some("es"), "no_such_symbol", "The English summary."),
            "The English summary."
        );
        assert_eq!(
            localised(None, "integrate", "The English summary."),
            "The English summary."
        );
        assert_eq!(summary("fr", "integrate"), None);
    }

    #[test]
    fn spanish_is_listed() {
        assert_eq!(languages(), ["es"]);
    }
}
