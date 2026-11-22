//! Function search, documentation and completion over both sources Aximar
//! knows about: the documentation catalog and the installed packages.  The
//! desktop app and MCP both answer from here and only format the results,
//! so they find the same functions.

use std::collections::HashSet;

use serde::Serialize;

use super::doc_index::{levenshtein, SymbolEntry};
use super::packages::PackageCatalog;
use super::search::Catalog;
use super::types::{CompletionResult, PackageInfo};

/// A function found by a search.
#[derive(Debug, Clone, Serialize)]
pub struct FunctionHit {
    pub name: String,
    pub signature: String,
    pub summary: String,
    /// The documentation source, or the packages that provide it.
    pub package: String,
    /// The catalog's relevance score, for hits the catalog's search ranked.
    /// Hits found by name in the packages have none; they follow the ranked
    /// hits.
    pub score: Option<f64>,
    /// Whether the catalog has documentation for it.
    pub documented: bool,
}

/// What is known about a function name.
pub enum FunctionDocs<'a> {
    /// Documented in the catalog.
    Documented { package: &'a str, entry: &'a SymbolEntry },
    /// Not documented, but installed packages provide it (usually one).
    InPackages { packages: Vec<&'a PackageInfo>, signature: Option<&'a str> },
    /// Unknown.  Holds names that are close, from both sources.
    NotFound { suggestions: Vec<String> },
}

/// Search both sources.  The catalog's ranked results come first; then
/// functions the packages match by name that the ranking left out, with
/// their documentation when the catalog has it.  Each name appears once.
pub fn search(catalog: &Catalog, packages: &PackageCatalog, query: &str) -> Vec<FunctionHit> {
    let mut seen = HashSet::new();
    let mut hits = Vec::new();
    for found in catalog.search(query) {
        if seen.insert(found.name.to_lowercase()) {
            hits.push(FunctionHit {
                name: found.name,
                signature: found.signature,
                summary: found.summary,
                package: found.package,
                score: Some(found.score),
                documented: true,
            });
        }
    }
    for found in packages.search_functions(query) {
        if !seen.insert(found.function_name.to_lowercase()) {
            continue;
        }
        let hit = match catalog.get(&found.function_name) {
            Some((package, entry)) => FunctionHit {
                name: found.function_name,
                signature: entry.signature.clone(),
                summary: entry.summary.clone(),
                package: package.clone(),
                score: None,
                documented: true,
            },
            None => {
                let providers = providers(packages, &found.function_name);
                FunctionHit {
                    summary: provided_by(&providers),
                    package: package_names(&providers),
                    name: found.function_name,
                    signature: found.signature,
                    score: None,
                    documented: false,
                }
            }
        };
        hits.push(hit);
    }
    hits
}

/// Look up one function by name (case-insensitive).
pub fn docs<'a>(catalog: &'a Catalog, packages: &'a PackageCatalog, name: &str) -> FunctionDocs<'a> {
    if let Some((package, entry)) = catalog.get(name) {
        return FunctionDocs::Documented { package, entry };
    }
    let providing = providers(packages, name);
    if !providing.is_empty() {
        let signature = providing.iter().find_map(|package| {
            package
                .functions
                .iter()
                .find(|f| f.eq_ignore_ascii_case(name))
                .and_then(|f| package.signatures.get(f))
                .map(String::as_str)
        });
        return FunctionDocs::InPackages { packages: providing, signature };
    }
    FunctionDocs::NotFound { suggestions: suggestions(catalog, packages, name) }
}

/// Complete a name prefix from both sources.  Catalog completions come
/// first.  A name listed by the catalog and by packages, or by several
/// packages, appears once, and says every package that provides it.
pub fn complete(catalog: &Catalog, packages: &PackageCatalog, prefix: &str) -> Vec<CompletionResult> {
    let mut seen = HashSet::new();
    let mut results: Vec<CompletionResult> =
        catalog.complete(prefix).into_iter().filter(|c| seen.insert(c.name.to_lowercase())).collect();
    for mut completion in packages.complete_functions(prefix) {
        if !seen.insert(completion.name.to_lowercase()) {
            continue;
        }
        let providing = providers(packages, &completion.name);
        if providing.len() > 1 {
            completion.description = load_hint(&providing);
        }
        results.push(completion);
    }
    results
}

/// One line on the packages that provide a function and how to load them:
/// "Load it with load(\"a\")$ or load(\"b\")$."
pub fn load_hint(providing: &[&PackageInfo]) -> String {
    if providing.iter().any(|p| p.builtin) {
        return "It is loaded automatically.".to_string();
    }
    let loads: Vec<String> = providing.iter().map(|p| format!("load(\"{}\")$", p.name)).collect();
    format!("Load it with {}.", loads.join(" or "))
}

/// "From the distrib package: Probability distributions (...)", leaving out
/// a description that only repeats the name ("macbasic package").
pub fn provided_by(providing: &[&PackageInfo]) -> String {
    let names = package_names(providing);
    match providing {
        [only] => match description(only) {
            Some(text) => format!("From the {names} package: {text}"),
            None => format!("From the {names} package."),
        },
        _ => format!("From the {names} packages."),
    }
}

/// A package's description, unless it only repeats the package's name.
fn description(package: &PackageInfo) -> Option<&str> {
    let text = package.description.trim();
    let last = package.name.rsplit('/').next().unwrap_or(&package.name);
    let placeholder = text.is_empty()
        || text.eq_ignore_ascii_case(&format!("{last} package"))
        || text.eq_ignore_ascii_case(&format!("{} package", package.name));
    (!placeholder).then_some(text)
}

fn package_names(providing: &[&PackageInfo]) -> String {
    providing.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" or ")
}

/// Every package that lists `name` (case-insensitive), in catalog order.
fn providers<'a>(packages: &'a PackageCatalog, name: &str) -> Vec<&'a PackageInfo> {
    packages
        .all()
        .iter()
        .filter(|p| p.functions.iter().any(|f| f.eq_ignore_ascii_case(name)))
        .collect()
}

/// Up to five names within three edits of `name`, nearest first, from the
/// catalog and the packages.
fn suggestions(catalog: &Catalog, packages: &PackageCatalog, name: &str) -> Vec<String> {
    let lower = name.to_lowercase();
    let mut near: Vec<(usize, String)> = catalog
        .find_similar(name, 3)
        .into_iter()
        .map(|n| (levenshtein(&lower, &n.to_lowercase()), n))
        .collect();
    for package in packages.all() {
        for function in &package.functions {
            let distance = levenshtein(&lower, &function.to_lowercase());
            if distance > 0 && distance <= 3 {
                near.push((distance, function.clone()));
            }
        }
    }
    near.sort();
    let mut seen = HashSet::new();
    near.into_iter()
        .filter(|(_, n)| seen.insert(n.to_lowercase()))
        .map(|(_, n)| n)
        .take(5)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;

    static CATALOG: LazyLock<Catalog> = LazyLock::new(Catalog::load);
    static PACKAGES: LazyLock<PackageCatalog> = LazyLock::new(PackageCatalog::load);

    fn count(names: impl Iterator<Item = String>, name: &str) -> usize {
        names.filter(|n| n.eq_ignore_ascii_case(name)).count()
    }

    #[test]
    fn a_name_in_both_sources_appears_once() {
        // nusum is documented and listed by algebra/nusum
        assert!(CATALOG.get("nusum").is_some());
        assert!(PACKAGES.package_for_function("nusum").is_some());
        let completions = complete(&CATALOG, &PACKAGES, "nusu");
        assert_eq!(count(completions.into_iter().map(|c| c.name), "nusum"), 1);
        let hits = search(&CATALOG, &PACKAGES, "nusum");
        assert_eq!(count(hits.into_iter().map(|h| h.name), "nusum"), 1);
    }

    #[test]
    fn a_name_in_several_packages_appears_once() {
        // gradient is listed by more than one package
        let listing = PACKAGES
            .all()
            .iter()
            .filter(|p| p.functions.iter().any(|f| f == "gradient"))
            .count();
        assert!(listing > 1, "gradient is in {listing} package(s)");
        let completions = complete(&CATALOG, &PACKAGES, "gradien");
        assert_eq!(count(completions.into_iter().map(|c| c.name), "gradient"), 1);
    }

    #[test]
    fn search_finds_undocumented_package_functions_after_documented_ones() {
        let hits = search(&CATALOG, &PACKAGES, "jcubic");
        let jcubic = hits.iter().find(|h| h.name == "jcubic").expect("jcubic is found");
        assert!(!jcubic.documented);
        // Ranked hits first, then the ones found by name
        let first_unranked = hits.iter().position(|h| h.score.is_none()).unwrap();
        assert!(hits[first_unranked..].iter().all(|h| h.score.is_none()));
    }

    #[test]
    fn docs_for_each_kind_of_name() {
        assert!(matches!(docs(&CATALOG, &PACKAGES, "Integrate"), FunctionDocs::Documented { .. }));
        match docs(&CATALOG, &PACKAGES, "jcubic") {
            FunctionDocs::InPackages { packages, .. } => assert_eq!(packages[0].name, "affine/macbasic"),
            _ => panic!("jcubic should come from a package"),
        }
    }

    #[test]
    fn a_miss_suggests_near_names_from_both_sources() {
        match docs(&CATALOG, &PACKAGES, "integrat") {
            FunctionDocs::NotFound { suggestions } => assert_eq!(suggestions[0], "integrate"),
            _ => panic!("integrat is not a function"),
        }
        match docs(&CATALOG, &PACKAGES, "jcubik") {
            FunctionDocs::NotFound { suggestions } => assert!(suggestions.contains(&"jcubic".to_string()), "{suggestions:?}"),
            _ => panic!("jcubik is not a function"),
        }
    }

    #[test]
    fn a_documented_function_found_by_name_keeps_its_docs() {
        // The catalog's ranking for "pdf_" leaves pdf_student_t out of its top
        // results; the package search finds it, and its docs still apply
        let hits = search(&CATALOG, &PACKAGES, "pdf_");
        let student = hits.iter().find(|h| h.name == "pdf_student_t").expect("pdf_student_t is found");
        assert!(student.documented, "{student:?}");
        assert!(student.summary.contains("Student"), "{student:?}");
    }

    #[test]
    fn every_package_that_provides_a_name_is_named() {
        let completions = complete(&CATALOG, &PACKAGES, "gradien");
        let gradient = completions.iter().find(|c| c.name == "gradient").unwrap();
        assert!(gradient.description.contains("calculus/optmiz"), "{gradient:?}");
        assert!(gradient.description.contains("calculus/qual"), "{gradient:?}");
    }

    #[test]
    fn placeholder_descriptions_are_left_out() {
        let macbasic = PACKAGES.get("affine/macbasic").unwrap();
        assert_eq!(provided_by(&[macbasic]), "From the affine/macbasic package.");
        let distrib = PACKAGES.get("distrib").unwrap();
        assert!(provided_by(&[distrib]).starts_with("From the distrib package: Probability"));
        assert_eq!(load_hint(&[macbasic]), "Load it with load(\"affine/macbasic\")$.");
    }
}
