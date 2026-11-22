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
    /// The documentation source or package the function comes from.
    pub package: String,
    pub score: f64,
    /// True when only a package lists it: there are no docs to show.
    pub undocumented: bool,
}

/// What is known about a function name.
pub enum FunctionDocs<'a> {
    /// Documented in the catalog.
    Documented { package: &'a str, entry: &'a SymbolEntry },
    /// Not documented, but an installed package provides it.
    InPackage { package: &'a PackageInfo, signature: Option<&'a str> },
    /// Unknown.  Holds names that are close, from both sources.
    NotFound { suggestions: Vec<String> },
}

/// Search both sources.  Documented functions come first, in the catalog's
/// ranking; package functions with no documentation follow.  Each name
/// appears once.
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
                score: found.score,
                undocumented: false,
            });
        }
    }
    for found in packages.search_functions(query) {
        if seen.insert(found.function_name.to_lowercase()) {
            hits.push(FunctionHit {
                summary: format!("From the {} package: {}", found.package_name, found.package_description),
                name: found.function_name,
                signature: found.signature,
                package: found.package_name,
                score: found.score,
                undocumented: true,
            });
        }
    }
    hits
}

/// Look up one function by name (case-insensitive).
pub fn docs<'a>(catalog: &'a Catalog, packages: &'a PackageCatalog, name: &str) -> FunctionDocs<'a> {
    if let Some((package, entry)) = catalog.get(name) {
        return FunctionDocs::Documented { package, entry };
    }
    if let Some(package) = packages.package_for_function(name).and_then(|p| packages.get(p)) {
        let signature = package
            .functions
            .iter()
            .find(|f| f.eq_ignore_ascii_case(name))
            .and_then(|f| package.signatures.get(f))
            .map(String::as_str);
        return FunctionDocs::InPackage { package, signature };
    }
    FunctionDocs::NotFound { suggestions: suggestions(catalog, packages, name) }
}

/// Complete a name prefix from both sources.  Catalog completions come
/// first; a name listed by the catalog and by packages, or by several
/// packages, appears once.
pub fn complete(catalog: &Catalog, packages: &PackageCatalog, prefix: &str) -> Vec<CompletionResult> {
    let mut seen = HashSet::new();
    catalog
        .complete(prefix)
        .into_iter()
        .chain(packages.complete_functions(prefix))
        .filter(|c| seen.insert(c.name.to_lowercase()))
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
        assert!(jcubic.undocumented);
        let first_undocumented = hits.iter().position(|h| h.undocumented).unwrap();
        assert!(hits[first_undocumented..].iter().all(|h| h.undocumented));
    }

    #[test]
    fn docs_for_each_kind_of_name() {
        assert!(matches!(docs(&CATALOG, &PACKAGES, "Integrate"), FunctionDocs::Documented { .. }));
        match docs(&CATALOG, &PACKAGES, "jcubic") {
            FunctionDocs::InPackage { package, .. } => assert!(package.functions.contains(&"jcubic".to_string())),
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
}
