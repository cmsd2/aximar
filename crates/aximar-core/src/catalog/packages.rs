use std::borrow::Cow;
use std::collections::HashMap;

use probly_search::score::bm25;
use probly_search::Index;

use crate::catalog::types::*;

pub struct PackageCatalog {
    packages: Vec<PackageInfo>,
    index: Index<usize>,
    /// function_name (lowercase) → package name
    func_to_package: HashMap<String, String>,
}

fn tokenizer(s: &str) -> Vec<Cow<'_, str>> {
    s.split_whitespace()
        .map(|w| Cow::Owned(w.to_lowercase()))
        .collect()
}

fn name_extract(d: &PackageInfo) -> Vec<&str> {
    vec![d.name.as_str()]
}

fn description_extract(d: &PackageInfo) -> Vec<&str> {
    vec![d.description.as_str()]
}

impl PackageCatalog {
    pub fn load() -> Self {
        let json = include_str!("packages.json");
        let packages: Vec<PackageInfo> =
            serde_json::from_str(json).expect("embedded packages.json must be valid");

        let mut index = Index::<usize>::new(2);
        for (i, p) in packages.iter().enumerate() {
            index.add_document(
                &[name_extract, description_extract],
                tokenizer,
                i,
                p,
            );
        }

        let mut func_to_package = HashMap::new();
        for p in &packages {
            for f in &p.functions {
                func_to_package.insert(f.to_lowercase(), p.name.clone());
            }
        }

        PackageCatalog {
            packages,
            index,
            func_to_package,
        }
    }

    pub fn search(&self, query: &str) -> Vec<PackageSearchResult> {
        if query.is_empty() {
            return self
                .packages
                .iter()
                .map(|p| PackageSearchResult {
                    package: p.clone(),
                    score: 0.0,
                })
                .collect();
        }

        let mut results: Vec<PackageSearchResult> = self
            .index
            .query(query, &mut bm25::new(), tokenizer, &[3.0, 1.0])
            .into_iter()
            .map(|qr| PackageSearchResult {
                package: self.packages[qr.key].clone(),
                score: qr.score as f64,
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);
        results
    }

    pub fn complete(&self, prefix: &str) -> Vec<PackageCompletionResult> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let p = prefix.to_lowercase();
        let mut matches: Vec<(f64, &PackageInfo)> = self
            .packages
            .iter()
            .filter_map(|pkg| {
                let name_lower = pkg.name.to_lowercase();
                if name_lower.starts_with(&p) || name_lower.contains(&p) {
                    // Prefix match scores higher than substring match
                    let score = if name_lower.starts_with(&p) {
                        100.0 - pkg.name.len() as f64
                    } else {
                        50.0 - pkg.name.len() as f64
                    };
                    Some((score, pkg))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        matches.truncate(10);

        matches
            .into_iter()
            .map(|(_, pkg)| PackageCompletionResult {
                name: pkg.name.clone(),
                description: pkg.description.clone(),
            })
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<&PackageInfo> {
        let n = name.to_lowercase();
        self.packages.iter().find(|p| p.name.to_lowercase() == n)
    }

    /// Given a function name, return the package that provides it (if any).
    pub fn package_for_function(&self, name: &str) -> Option<&str> {
        self.func_to_package
            .get(&name.to_lowercase())
            .map(|s| s.as_str())
    }

    /// Complete function names from package functions by prefix.
    /// Returns `CompletionResult`s with the package name in the description.
    pub fn complete_functions(&self, prefix: &str) -> Vec<CompletionResult> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let p = prefix.to_lowercase();
        let mut results: Vec<(f64, CompletionResult)> = Vec::new();

        for pkg in &self.packages {
            for func in &pkg.functions {
                let func_lower = func.to_lowercase();
                if func_lower.starts_with(&p) {
                    let score = 50.0 - func.len() as f64; // lower priority than catalog
                    results.push((
                        score,
                        CompletionResult {
                            name: func.clone(),
                            signature: String::new(),
                            description: format!("requires load(\"{}\")", pkg.name),
                            insert_text: format!("{}(", func),
                            package: Some(pkg.name.clone()),
                        },
                    ));
                }
            }
        }

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(20);
        results.into_iter().map(|(_, r)| r).collect()
    }

    /// Search package function names by substring match.
    /// Prefix matches score higher than substring matches.
    pub fn search_functions(&self, query: &str) -> Vec<PackageFunctionSearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let q = query.to_lowercase();
        let mut results: Vec<PackageFunctionSearchResult> = Vec::new();

        for (func_lower, pkg_name) in &self.func_to_package {
            if func_lower.contains(&q) {
                let score = if func_lower.starts_with(&q) {
                    100.0 - func_lower.len() as f64
                } else {
                    50.0 - func_lower.len() as f64
                };
                // Look up the original-case function name and package description
                if let Some(pkg) = self.packages.iter().find(|p| p.name == *pkg_name) {
                    let original_name = pkg
                        .functions
                        .iter()
                        .find(|f| f.to_lowercase() == *func_lower)
                        .cloned()
                        .unwrap_or_else(|| func_lower.clone());
                    results.push(PackageFunctionSearchResult {
                        function_name: original_name,
                        package_name: pkg_name.clone(),
                        package_description: pkg.description.clone(),
                        score,
                    });
                }
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(20);
        results
    }

    pub fn all(&self) -> &[PackageInfo] {
        &self.packages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packages_load() {
        let catalog = PackageCatalog::load();
        assert!(
            catalog.packages.len() > 10,
            "expected at least 10 packages, got {}",
            catalog.packages.len()
        );
    }

    #[test]
    fn test_search_packages() {
        let catalog = PackageCatalog::load();
        let results = catalog.search("distrib");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.package.name == "distrib"));
    }

    #[test]
    fn test_complete_packages() {
        let catalog = PackageCatalog::load();
        let results = catalog.complete("dist");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.name == "distrib"));
    }

    #[test]
    fn test_package_for_function() {
        let catalog = PackageCatalog::load();
        let pkg = catalog.package_for_function("pdf_normal");
        assert_eq!(pkg, Some("distrib"));
    }

    #[test]
    fn test_get_package() {
        let catalog = PackageCatalog::load();
        let pkg = catalog.get("distrib");
        assert!(pkg.is_some());
        assert!(!pkg.unwrap().functions.is_empty());
    }

    #[test]
    fn test_search_functions() {
        let catalog = PackageCatalog::load();
        let results = catalog.search_functions("pdf_nor");
        assert!(!results.is_empty());
        assert!(
            results.iter().any(|r| r.function_name == "pdf_normal"),
            "Expected pdf_normal in results: {:?}",
            results.iter().map(|r| &r.function_name).collect::<Vec<_>>()
        );
        assert_eq!(results[0].package_name, "distrib");
        // Prefix matches should score higher
        let prefix_score = results
            .iter()
            .find(|r| r.function_name == "pdf_normal")
            .unwrap()
            .score;
        assert!(prefix_score > 50.0, "Prefix match should score > 50");
    }

    #[test]
    fn test_search_functions_substring() {
        let catalog = PackageCatalog::load();
        let results = catalog.search_functions("normal");
        assert!(!results.is_empty());
        // Substring matches should have score <= 50
        for r in &results {
            if !r.function_name.to_lowercase().starts_with("normal") {
                assert!(r.score <= 50.0, "Substring match should score <= 50");
            }
        }
    }

    #[test]
    fn test_search_functions_empty() {
        let catalog = PackageCatalog::load();
        let results = catalog.search_functions("");
        assert!(results.is_empty());
    }

    #[test]
    fn test_complete_functions() {
        let catalog = PackageCatalog::load();
        let results = catalog.complete_functions("pdf_");
        assert!(!results.is_empty());
        assert!(
            results.iter().any(|r| r.name == "pdf_normal"),
            "Expected pdf_normal in results: {:?}",
            results.iter().map(|r| &r.name).collect::<Vec<_>>()
        );
        // Should mention the package in the description
        assert!(results[0].description.contains("load("));
    }
}
