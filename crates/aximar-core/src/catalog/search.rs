use std::collections::BTreeMap;

use crate::catalog::types::*;

pub struct Catalog {
    functions: Vec<MaximaFunction>,
}

impl Catalog {
    pub fn load() -> Self {
        let json = include_str!("catalog.json");
        let functions: Vec<MaximaFunction> =
            serde_json::from_str(json).expect("embedded catalog.json must be valid");
        Catalog { functions }
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

        let q = query.to_lowercase();
        let mut results: Vec<SearchResult> = self
            .functions
            .iter()
            .filter_map(|f| {
                let score = score_match(&f.name, &f.description, &q);
                if score > 0.0 {
                    Some(SearchResult {
                        function: f.clone(),
                        score,
                    })
                } else {
                    None
                }
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

fn score_match(name: &str, description: &str, query: &str) -> f64 {
    let name_lower = name.to_lowercase();
    let desc_lower = description.to_lowercase();

    // Exact name match
    if name_lower == query {
        return 1000.0;
    }

    // Name starts with query — prefer higher coverage ratio (query_len / name_len)
    if name_lower.starts_with(query) {
        return 500.0 + (query.len() as f64 / name.len() as f64) * 100.0;
    }

    // Name contains query
    if name_lower.contains(query) {
        return 200.0 + (100.0 / name.len() as f64);
    }

    // Description contains query
    if desc_lower.contains(query) {
        return 50.0;
    }

    // Fuzzy: check if all query chars appear in order in the name
    if fuzzy_match(&name_lower, query) {
        return 10.0;
    }

    0.0
}

fn fuzzy_match(text: &str, query: &str) -> bool {
    let mut chars = text.chars();
    for qc in query.chars() {
        loop {
            match chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
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
        // All prefix matches should score above 500
        assert!(results[0].score > 500.0);
    }

    #[test]
    fn test_search_description() {
        let catalog = Catalog::load();
        let results = catalog.search("derivative");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.function.name == "diff"));
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
    fn test_by_category() {
        let catalog = Catalog::load();
        let groups = catalog.by_category();
        assert!(!groups.is_empty());
        assert!(groups.iter().any(|g| g.label == "Calculus"));
    }
}
