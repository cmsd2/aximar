use std::borrow::Cow;
use std::collections::BTreeMap;

use probly_search::score::bm25;
use probly_search::Index;

use crate::catalog::types::*;

pub struct Catalog {
    functions: Vec<MaximaFunction>,
    index: Index<usize>,
}

fn tokenizer(s: &str) -> Vec<Cow<'_, str>> {
    s.split_whitespace()
        .map(|w| Cow::Owned(w.to_lowercase()))
        .collect()
}

fn name_extract(d: &MaximaFunction) -> Vec<&str> {
    vec![d.name.as_str()]
}

fn description_extract(d: &MaximaFunction) -> Vec<&str> {
    vec![d.description.as_str()]
}

impl Catalog {
    pub fn load() -> Self {
        let json = include_str!("catalog.json");
        let functions: Vec<MaximaFunction> =
            serde_json::from_str(json).expect("embedded catalog.json must be valid");

        let mut index = Index::<usize>::new(2);
        for (i, f) in functions.iter().enumerate() {
            index.add_document(
                &[name_extract, description_extract],
                tokenizer,
                i,
                f,
            );
        }

        Catalog { functions, index }
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return self
                .functions
                .iter()
                .map(|f| SearchResult {
                    function: f.clone(),
                    score: 0.0,
                })
                .collect();
        }

        let mut results: Vec<SearchResult> = self
            .index
            .query(query, &mut bm25::new(), tokenizer, &[3.0, 1.0])
            .into_iter()
            .map(|qr| SearchResult {
                function: self.functions[qr.key].clone(),
                score: qr.score as f64,
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);
        results
    }

    pub fn complete(&self, prefix: &str) -> Vec<CompletionResult> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let p = prefix.to_lowercase();
        let mut matches: Vec<(f64, &MaximaFunction)> = self
            .functions
            .iter()
            .filter_map(|f| {
                let name_lower = f.name.to_lowercase();
                if name_lower.starts_with(&p) {
                    // Exact prefix match gets highest score, shorter names rank higher
                    let score = 100.0 - f.name.len() as f64;
                    Some((score, f))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        matches.truncate(10);

        matches
            .into_iter()
            .map(|(_, f)| CompletionResult {
                name: f.name.clone(),
                signature: f.signatures.first().cloned().unwrap_or_default(),
                description: f.description.clone(),
                insert_text: format!("{}()", f.name),
            })
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<&MaximaFunction> {
        let n = name.to_lowercase();
        self.functions.iter().find(|f| f.name.to_lowercase() == n)
    }

    pub fn by_category(&self) -> Vec<CategoryGroup> {
        let mut map: BTreeMap<String, (FunctionCategory, Vec<MaximaFunction>)> = BTreeMap::new();

        for f in &self.functions {
            let label = f.category.label().to_string();
            map.entry(label.clone())
                .or_insert_with(|| (f.category, Vec::new()))
                .1
                .push(f.clone());
        }

        map.into_values()
            .map(|(cat, functions)| CategoryGroup {
                label: cat.label().to_string(),
                category: cat,
                functions,
            })
            .collect()
    }

    pub fn find_deprecated(&self) -> Vec<DeprecationInfo> {
        self.functions
            .iter()
            .filter_map(|f| {
                let desc_lower = f.description.to_lowercase();
                // Only "obsolete" and "deprecated" reliably indicate a function
                // is deprecated. Other phrases like "replaced by" appear throughout
                // descriptions that merely explain what a function does.
                if desc_lower.contains("obsolete") || desc_lower.contains("deprecated") {
                    let replacement = extract_replacement(&desc_lower);
                    Some(DeprecationInfo {
                        name: f.name.clone(),
                        description: f.description.clone(),
                        replacement,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn find_similar(&self, name: &str, max_distance: usize) -> Vec<String> {
        let n = name.to_lowercase();
        let mut matches: Vec<(usize, String)> = self
            .functions
            .iter()
            .filter_map(|f| {
                let dist = levenshtein(&n, &f.name.to_lowercase());
                if dist > 0 && dist <= max_distance {
                    Some((dist, f.name.clone()))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by_key(|(d, _)| *d);
        matches.truncate(5);
        matches.into_iter().map(|(_, name)| name).collect()
    }
}

/// Extract a replacement function name from deprecation text.
/// Looks for patterns like "replaced by X", "use X instead", "superseded by X".
fn extract_replacement(desc_lower: &str) -> Option<String> {
    // Try "replaced by X" or "superseded by X"
    for prefix in &["replaced by ", "superseded by "] {
        if let Some(pos) = desc_lower.find(prefix) {
            let after = &desc_lower[pos + prefix.len()..];
            let name = extract_word(after);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    // Try "use X instead"
    if let Some(pos) = desc_lower.find("use ") {
        let after = &desc_lower[pos + 4..];
        if let Some(instead_pos) = after.find("instead") {
            let between = after[..instead_pos].trim();
            let name = extract_word(between);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    None
}

/// Extract the first word (alphanumeric + underscore) from a string.
fn extract_word(s: &str) -> String {
    let trimmed = s.trim().trim_start_matches('`').trim_start_matches('\'');
    trimmed
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_loads() {
        let catalog = Catalog::load();
        assert!(catalog.functions.len() > 100);
    }

    #[test]
    fn test_search_exact() {
        let catalog = Catalog::load();
        let results = catalog.search("integrate");
        assert!(!results.is_empty());
        assert_eq!(results[0].function.name, "integrate");
    }

    #[test]
    fn test_search_prefix() {
        let catalog = Catalog::load();
        let results = catalog.search("integ");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.function.name == "integrate"));
    }

    #[test]
    fn test_search_description() {
        let catalog = Catalog::load();
        let results = catalog.search("derivative");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.function.name == "diff"));
    }

    #[test]
    fn test_search_multi_word() {
        let catalog = Catalog::load();

        // Multi-word query that previously returned zero results
        let results = catalog.search("taylor series");
        assert!(!results.is_empty(), "multi-word query 'taylor series' should return results");
        assert!(
            results.iter().any(|r| r.function.name == "taylor"),
            "expected taylor in results for 'taylor series'"
        );

        let results = catalog.search("solve equation");
        assert!(!results.is_empty(), "multi-word query 'solve equation' should return results");
        assert!(
            results.iter().any(|r| r.function.name == "solve"),
            "expected solve in results for 'solve equation'"
        );
    }

    #[test]
    fn test_complete() {
        let catalog = Catalog::load();
        let completions = catalog.complete("int");
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.name == "integrate"));
    }

    #[test]
    fn test_get() {
        let catalog = Catalog::load();
        let f = catalog.get("solve");
        assert!(f.is_some());
        assert_eq!(f.unwrap().name, "solve");
    }

    #[test]
    fn test_find_similar() {
        let catalog = Catalog::load();
        let similar = catalog.find_similar("intgrate", 3);
        assert!(similar.contains(&"integrate".to_string()));
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
    }

    #[test]
    fn test_find_deprecated() {
        let catalog = Catalog::load();
        let deprecated = catalog.find_deprecated();
        // The catalog should have at least one deprecated entry
        assert!(!deprecated.is_empty(), "expected at least one deprecated function");
        // Every entry should have a non-empty name and description
        for info in &deprecated {
            assert!(!info.name.is_empty());
            assert!(!info.description.is_empty());
        }
    }

    #[test]
    fn test_extract_replacement() {
        assert_eq!(
            extract_replacement("this is obsolete. replaced by style"),
            Some("style".to_string())
        );
        assert_eq!(
            extract_replacement("use foo instead"),
            Some("foo".to_string())
        );
        assert_eq!(
            extract_replacement("superseded by bar_baz"),
            Some("bar_baz".to_string())
        );
        assert_eq!(extract_replacement("this is obsolete"), None);
    }

    #[test]
    fn test_by_category() {
        let catalog = Catalog::load();
        let groups = catalog.by_category();
        assert!(!groups.is_empty());
        assert!(groups.iter().any(|g| g.label == "Calculus"));
    }
}
