use tauri::State;

use crate::i18n::docs_chain;
use crate::state::AppState;
use aximar_core::catalog::doc_index::{LanguageChain, Resolved};
use aximar_core::catalog::queries::{self, FunctionDocs};
use aximar_core::catalog::search::Catalog;
use aximar_core::catalog::types::*;

/// A documented function in a language chain, for the window.  Its
/// description is the full documentation where there is some, and the
/// summary otherwise.  A summary in another language than the full
/// documentation, such as a Spanish summary above English documentation,
/// comes separately, in `languages.summary_text`.
pub fn function_in(catalog: &Catalog, name: &str, chain: &LanguageChain) -> Option<MaximaFunction> {
    catalog
        .resolve(name, chain)
        .map(|r| resolved_to_function(name, r))
}

fn resolved_to_function(name: &str, resolved: Resolved) -> MaximaFunction {
    let entry = &resolved.entry;
    let mut sigs = vec![entry.signature.clone()];
    for alt in &entry.signatures {
        if !sigs.contains(alt) {
            sigs.push(alt.clone());
        }
    }
    let (description, summary_text) = if entry.body_md.is_empty() {
        (entry.summary.clone(), None)
    } else if resolved.summary_language != resolved.body_language && !entry.summary.is_empty() {
        (entry.body_md.clone(), Some(entry.summary.clone()))
    } else {
        (entry.body_md.clone(), None)
    };
    MaximaFunction {
        name: name.to_string(),
        signatures: sigs,
        description,
        category: entry
            .category
            .as_deref()
            .and_then(parse_category)
            .unwrap_or(FunctionCategory::Other),
        examples: entry
            .examples
            .iter()
            .map(|e| FunctionExample {
                input: e.input.clone(),
                description: if e.description.is_empty() {
                    None
                } else {
                    Some(e.description.clone())
                },
            })
            .collect(),
        see_also: entry.see_also.clone(),
        search_keywords: entry.keywords.join(" "),
        languages: Some(DocLanguages {
            summary: resolved.summary_language.clone(),
            body: resolved.body_language.clone(),
            fallback_from: resolved.fallback_from.clone(),
            stale: resolved.stale,
            english_signatures: resolved.english_signatures.clone(),
            summary_text,
        }),
    }
}

/// Documented functions only: the window lists package functions in a
/// section of their own, from `search_package_functions`.  Search matches
/// every language's text, and shows each result in the help language.
#[tauri::command]
pub fn search_functions(state: State<AppState>, query: String) -> Vec<SearchResult> {
    let chain = docs_chain();
    queries::search(&state.catalog, &state.packages, &query, &chain)
        .into_iter()
        .filter_map(|hit| {
            // The catalog's ranked hits only
            let score = hit.score?;
            if !documented(&state, &hit.name, &chain) {
                return None;
            }
            function_in(&state.catalog, &hit.name, &chain)
                .map(|function| SearchResult { function, score })
        })
        .collect()
}

#[tauri::command]
pub fn complete_function(state: State<AppState>, prefix: String) -> Vec<CompletionResult> {
    let chain = docs_chain();
    let mut results = queries::complete(&state.catalog, &state.packages, &prefix);
    // Core functions carry their summary, in the help language; package
    // ones a load() hint
    for r in results.iter_mut().filter(|r| r.package.is_none()) {
        if let Some(resolved) = state.catalog.resolve(&r.name, &chain) {
            r.description = resolved.entry.summary;
        }
    }
    results
}

/// Documented functions only: the window shows a package's page for a
/// function only a package provides (`package_for_function`).
#[tauri::command]
pub fn get_function(state: State<AppState>, name: String) -> Option<MaximaFunction> {
    let chain = docs_chain();
    if !documented(&state, &name, &chain) {
        return None;
    }
    function_in(&state.catalog, &name, &chain)
}

fn documented(state: &AppState, name: &str, chain: &LanguageChain) -> bool {
    matches!(
        queries::docs(&state.catalog, &state.packages, name, chain),
        FunctionDocs::Documented(_)
    )
}

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> Vec<CategoryGroup> {
    let chain = docs_chain();
    let doc_cats = state.catalog.by_category();
    doc_cats
        .into_iter()
        .map(|g| {
            let category = parse_category(&g.category).unwrap_or(FunctionCategory::Other);
            CategoryGroup {
                label: g.category,
                category,
                functions: g
                    .symbols
                    .into_iter()
                    .filter_map(|(name, _sig)| {
                        state
                            .catalog
                            .resolve(&name, &chain)
                            .map(|resolved| MaximaFunction {
                                description: resolved.entry.summary.clone(),
                                signatures: vec![resolved.entry.signature.clone()],
                                name,
                                category,
                                examples: Vec::new(),
                                see_also: Vec::new(),
                                search_keywords: String::new(),
                                languages: None,
                            })
                    })
                    .collect(),
            }
        })
        .collect()
}

/// A language the help has documentation in
#[derive(Debug, serde::Serialize)]
pub struct DocsLanguage {
    pub tag: String,
    /// Full documentation, not only summaries
    pub full: bool,
    /// The package that adds full documentation in it, when there is one
    pub pack: Option<String>,
}

/// The help's languages: which is chosen, which is in use, and which have
/// documentation
#[derive(Debug, serde::Serialize)]
pub struct DocsLanguages {
    /// The language chosen for the help, or null to follow the app's
    pub choice: Option<String>,
    /// The language the help is in now
    pub effective: String,
    /// English, then every language with documentation
    pub languages: Vec<DocsLanguage>,
}

pub fn docs_languages_in(
    catalog: &Catalog,
    choice: Option<String>,
    effective: String,
) -> DocsLanguages {
    let mut tags = vec![aximar_core::catalog::doc_index::ENGLISH.to_string()];
    tags.extend(catalog.languages());
    if !tags.contains(&effective) {
        tags.push(effective.clone());
    }
    let languages = tags
        .into_iter()
        .map(|tag| DocsLanguage {
            full: catalog.has_full_documentation(&tag),
            pack: aximar_core::catalog::doc_index::core_docs_pack(&tag),
            tag,
        })
        .collect();
    DocsLanguages {
        choice,
        effective,
        languages,
    }
}

#[tauri::command]
pub fn docs_languages(app: tauri::AppHandle, state: State<AppState>) -> DocsLanguages {
    let choice = crate::commands::config::read_docs_choice(&app);
    let effective = crate::commands::config::read_docs_language(&app);
    docs_languages_in(&state.catalog, choice, effective)
}

/// A function's full documentation in the help language, where there is any
#[tauri::command]
pub fn get_function_docs(state: State<AppState>, name: String) -> Option<String> {
    let chain = docs_chain();
    if !documented(&state, &name, &chain) {
        return None;
    }
    state
        .catalog
        .resolve(&name, &chain)
        .map(|r| r.entry.body_md)
        .filter(|body| !body.is_empty())
}

// ── Package commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn search_packages(state: State<AppState>, query: String) -> Vec<PackageSearchResult> {
    state.packages.search(&query)
}

#[tauri::command]
pub fn complete_packages(state: State<AppState>, prefix: String) -> Vec<PackageCompletionResult> {
    state.packages.complete(&prefix)
}

#[tauri::command]
pub fn get_package(state: State<AppState>, name: String) -> Option<PackageInfo> {
    state.packages.get(&name).cloned()
}

#[tauri::command]
pub fn package_for_function(state: State<AppState>, name: String) -> Option<String> {
    state
        .packages
        .package_for_function(&name)
        .map(|s| s.to_string())
}

#[tauri::command]
pub fn search_package_functions(
    state: State<AppState>,
    query: String,
) -> Vec<PackageFunctionSearchResult> {
    state.packages.search_functions(&query)
}

/// Parse a string category into the FunctionCategory enum.
fn parse_category(s: &str) -> Option<FunctionCategory> {
    match s {
        "Calculus" => Some(FunctionCategory::Calculus),
        "Algebra" => Some(FunctionCategory::Algebra),
        "LinearAlgebra" | "Linear Algebra" => Some(FunctionCategory::LinearAlgebra),
        "Simplification" => Some(FunctionCategory::Simplification),
        "Solving" => Some(FunctionCategory::Solving),
        "Plotting" => Some(FunctionCategory::Plotting),
        "Trigonometry" => Some(FunctionCategory::Trigonometry),
        "NumberTheory" | "Number Theory" => Some(FunctionCategory::NumberTheory),
        "Polynomials" => Some(FunctionCategory::Polynomials),
        "Series" => Some(FunctionCategory::Series),
        "Combinatorics" => Some(FunctionCategory::Combinatorics),
        "Programming" => Some(FunctionCategory::Programming),
        "IO" | "I/O" => Some(FunctionCategory::IO),
        "Other" => Some(FunctionCategory::Other),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aximar_core::catalog::doc_index::DocIndexStore;

    /// The usual Spanish setup: Aximar's embedded Spanish summaries, and
    /// maxima-core-docs giving full English documentation
    fn catalog_with_english_docs() -> (tempfile::TempDir, Catalog) {
        let dir = tempfile::tempdir().unwrap();
        let doc = dir.path().join("maxima-core-docs").join("doc");
        std::fs::create_dir_all(&doc).unwrap();
        std::fs::write(
            doc.join("maxima-core-docs-doc-index.json"),
            serde_json::json!({
                "version": 1,
                "package": "maxima-core-docs",
                "symbols": {
                    "integrate": {
                        "type": "Function",
                        "signature": "integrate(expr, x)",
                        "summary": "Attempts to symbolically compute the integral.",
                        "body_md": "The full English documentation.",
                        "category": "Calculus"
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        let mut store = DocIndexStore::new_empty();
        store.load_embedded_core();
        store.load_embedded_summaries();
        store.load_runtime_dir(dir.path());
        (dir, Catalog::from_store(store))
    }

    #[test]
    fn spanish_help_shows_the_spanish_summary_above_english_documentation() {
        let (_dir, catalog) = catalog_with_english_docs();
        let integrate = function_in(&catalog, "integrate", &LanguageChain::new(&["es"])).unwrap();
        assert_eq!(integrate.description, "The full English documentation.");
        let languages = integrate.languages.unwrap();
        assert!(languages
            .summary_text
            .clone()
            .unwrap()
            .starts_with("Calcula simbólicamente"));
        assert_eq!(
            (languages.summary.as_str(), languages.body.as_str()),
            ("es", "en")
        );
        assert_eq!(languages.fallback_from, None);
    }

    #[test]
    fn the_help_lists_its_languages() {
        let (_dir, catalog) = catalog_with_english_docs();
        let info = docs_languages_in(&catalog, None, "es".into());
        let tags: Vec<_> = info
            .languages
            .iter()
            .map(|l| (l.tag.as_str(), l.full))
            .collect();
        assert_eq!(tags, [("en", true), ("es", false)]);
        // Full Spanish documentation comes in a pack
        assert_eq!(info.languages[0].pack, None);
        assert_eq!(
            info.languages[1].pack.as_deref(),
            Some("maxima-core-docs-es")
        );
        assert_eq!(info.effective, "es");
        // A language with no pack yet
        let german = docs_languages_in(&catalog, None, "de".into());
        let de = german.languages.iter().find(|l| l.tag == "de").unwrap();
        assert_eq!(de.pack, None);
    }

    #[test]
    fn english_help_is_as_before() {
        let (_dir, catalog) = catalog_with_english_docs();
        let integrate = function_in(&catalog, "integrate", &LanguageChain::english()).unwrap();
        assert_eq!(integrate.description, "The full English documentation.");
        assert_eq!(integrate.languages.unwrap().summary, "en");
    }
}
