use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use probly_search::score::bm25;
use probly_search::Index;
use serde::{Deserialize, Serialize};

use super::types::{CompletionResult, DeprecationInfo};

// ---------------------------------------------------------------------------
// Types (mirror mxpm::doc_index, deserialize-only)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DocIndex {
    pub version: u32,
    pub package: String,
    #[serde(default)]
    pub source: String,
    pub symbols: BTreeMap<String, SymbolEntry>,
    #[serde(default)]
    pub sections: Vec<SectionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SymbolEntry {
    #[serde(rename = "type")]
    pub symbol_type: String,
    pub signature: String,
    pub summary: String,
    pub body_md: String,
    #[serde(default)]
    pub body_html: String,
    #[serde(default)]
    pub examples: Vec<ExampleEntry>,
    #[serde(default)]
    pub see_also: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExampleEntry {
    pub input: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SectionEntry {
    pub title: String,
    pub body_md: String,
    #[serde(default)]
    pub body_html: String,
}

// ---------------------------------------------------------------------------
// Search result for doc-index BM25 queries
// ---------------------------------------------------------------------------

pub struct DocSearchResult {
    pub name: String,
    pub signature: String,
    pub summary: String,
    pub package: String,
    pub score: f64,
}

// ---------------------------------------------------------------------------
// Category grouping result
// ---------------------------------------------------------------------------

pub struct DocCategoryGroup {
    pub category: String,
    pub symbols: Vec<(String, String)>, // (name, signature)
}

// ---------------------------------------------------------------------------
// DocIndexStore — aggregates symbols from all loaded doc-index files
// ---------------------------------------------------------------------------

/// Wrapper for a (name, package) pair that the BM25 index references by key.
struct IndexedSymbol {
    name: String,
    package: String,
    summary: String,
    keywords: String,
}

fn idx_name_extract(d: &IndexedSymbol) -> Vec<&str> {
    vec![d.name.as_str()]
}

fn idx_summary_extract(d: &IndexedSymbol) -> Vec<&str> {
    vec![d.summary.as_str()]
}

fn idx_keywords_extract(d: &IndexedSymbol) -> Vec<&str> {
    vec![d.keywords.as_str()]
}

fn idx_tokenizer(s: &str) -> Vec<Cow<'_, str>> {
    s.split_whitespace()
        .flat_map(|w| {
            let lower = w.to_lowercase();
            // Also split on underscores for function names
            let parts: Vec<String> = lower
                .split('_')
                .filter(|p| !p.is_empty())
                .map(|p| p.to_string())
                .collect();
            let mut tokens = vec![lower];
            if parts.len() > 1 {
                tokens.extend(parts);
            }
            tokens
        })
        .map(Cow::Owned)
        .collect()
}

pub struct DocIndexStore {
    /// symbol_name (lowercase) → (package_name, SymbolEntry)
    symbols: HashMap<String, (String, SymbolEntry)>,
    /// Original-case symbol names for prefix completion: (name, package)
    names: Vec<(String, String)>,
    /// Indexed symbols for BM25 search (parallel to names)
    indexed: Vec<IndexedSymbol>,
    /// BM25 index over name + summary + keywords (3 fields)
    index: Index<usize>,
}

impl DocIndexStore {
    /// Create an empty store with no symbols.
    pub fn new_empty() -> Self {
        DocIndexStore {
            symbols: HashMap::new(),
            names: Vec::new(),
            indexed: Vec::new(),
            index: Index::new(3),
        }
    }

    /// Load the embedded slim core doc-index as baseline, then overlay
    /// runtime doc-index files from `~/.maxima/`.
    pub fn load() -> Self {
        let mut store = Self::new_empty();
        store.load_embedded_core();
        store.load_runtime_files();
        store.build_bm25_index();
        store
    }

    /// Load the embedded core doc-index (from `core-doc-index.json`).
    pub fn load_embedded_core(&mut self) {
        let core_json = include_str!("core-doc-index.json");
        match serde_json::from_str::<DocIndex>(core_json) {
            Ok(index) => {
                if !index.symbols.is_empty() {
                    eprintln!(
                        "[doc-index] Loaded {} core symbols from embedded index",
                        index.symbols.len()
                    );
                }
                self.ingest_index(index);
            }
            Err(e) => {
                eprintln!("[doc-index] Failed to parse embedded core-doc-index.json: {e}");
            }
        }
    }

    /// Load runtime doc-index files from `~/.maxima/` package directories.
    pub fn load_runtime_files(&mut self) {
        let userdir = match maxima_userdir() {
            Some(d) => d,
            None => {
                eprintln!("[doc-index] Could not determine Maxima user directory");
                return;
            }
        };

        if !userdir.is_dir() {
            return;
        }

        let mut file_count = 0u32;

        let entries = match std::fs::read_dir(&userdir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for pkg_entry in entries.flatten() {
            let pkg_path = pkg_entry.path();
            if !pkg_path.is_dir() {
                continue;
            }

            let doc_dir = pkg_path.join("doc");
            if !doc_dir.is_dir() {
                continue;
            }

            let doc_entries = match std::fs::read_dir(&doc_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for file_entry in doc_entries.flatten() {
                let file_name = file_entry.file_name();
                let name = file_name.to_string_lossy();
                if !name.ends_with("-doc-index.json") {
                    continue;
                }

                let path = file_entry.path();
                match load_doc_index(&path) {
                    Ok(index) => {
                        self.ingest_index(index);
                        file_count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "[doc-index] Failed to load {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        if file_count > 0 {
            eprintln!(
                "[doc-index] Loaded {} symbols total ({} runtime file(s))",
                self.symbols.len(),
                file_count
            );
        }
    }

    /// Ingest all symbols from a doc-index, overwriting existing entries
    /// for the same symbol name.
    pub fn ingest_index(&mut self, index: DocIndex) {
        let pkg_name = index.package;
        for (sym_name, entry) in index.symbols {
            let key = sym_name.to_lowercase();
            // Remove existing entry from names/indexed if overwriting
            if self.symbols.contains_key(&key) {
                self.names.retain(|(n, _)| n.to_lowercase() != key);
                self.indexed.retain(|s| s.name.to_lowercase() != key);
            }
            self.names.push((sym_name.clone(), pkg_name.clone()));
            self.indexed.push(IndexedSymbol {
                name: sym_name.clone(),
                package: pkg_name.clone(),
                summary: entry.summary.clone(),
                keywords: entry.keywords.join(" "),
            });
            self.symbols.insert(key, (pkg_name.clone(), entry));
        }
    }

    /// Build the BM25 index from the current indexed symbols.
    pub fn build_bm25_index(&mut self) {
        self.index = Index::new(3);
        for (i, sym) in self.indexed.iter().enumerate() {
            self.index.add_document(
                &[idx_name_extract, idx_summary_extract, idx_keywords_extract],
                idx_tokenizer,
                i,
                sym,
            );
        }
    }

    /// BM25 search across symbol names, summaries, and keywords.
    pub fn search(&self, query: &str) -> Vec<DocSearchResult> {
        if query.is_empty() || self.indexed.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<DocSearchResult> = self
            .index
            .query(query, &mut bm25::new(), idx_tokenizer, &[3.0, 1.0, 2.0])
            .into_iter()
            .map(|qr| {
                let sym = &self.indexed[qr.key];
                let signature = self
                    .symbols
                    .get(&sym.name.to_lowercase())
                    .map(|(_, e)| e.signature.clone())
                    .unwrap_or_default();
                DocSearchResult {
                    name: sym.name.clone(),
                    signature,
                    summary: sym.summary.clone(),
                    package: sym.package.clone(),
                    score: qr.score as f64,
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);
        results
    }

    /// Look up a symbol by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&(String, SymbolEntry)> {
        self.symbols.get(&name.to_lowercase())
    }

    /// Build hover markdown for a symbol.
    ///
    /// Images are stripped since VS Code hover tooltips can't render them.
    pub fn hover_markdown(&self, name: &str) -> Option<String> {
        let (pkg_name, entry) = self.get(name)?;

        let mut md = format!("```maxima\n{}\n```\n\n", entry.signature);

        if !entry.body_md.is_empty() {
            md.push_str(&strip_markdown_images(&entry.body_md));
        } else if !entry.summary.is_empty() {
            md.push_str(&entry.summary);
        }

        if !entry.examples.is_empty() {
            md.push_str("\n\n**Examples:**\n");
            for ex in &entry.examples {
                md.push_str(&format!("```maxima\n{}\n```\n", ex.input));
                if !ex.description.is_empty() {
                    md.push_str(&format!("{}\n", ex.description));
                }
            }
        }

        if !entry.see_also.is_empty() {
            md.push_str(&format!(
                "\n**See also:** {}",
                entry.see_also.join(", ")
            ));
        }

        md.push_str(&format!("\n\n*From package `{}`*", pkg_name));

        Some(md)
    }

    /// Prefix-match symbol names for completion.
    pub fn complete(&self, prefix: &str) -> Vec<CompletionResult> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let p = prefix.to_lowercase();
        let mut results: Vec<CompletionResult> = Vec::new();

        for (name, pkg_name) in &self.names {
            if name.to_lowercase().starts_with(&p) {
                if let Some((_, entry)) = self.symbols.get(&name.to_lowercase()) {
                    // Core docs don't need a load() hint
                    let description = if pkg_name == "maxima-core-docs" {
                        entry.summary.clone()
                    } else {
                        format!("requires load(\"{}\")", pkg_name)
                    };
                    let package = if pkg_name == "maxima-core-docs" {
                        None
                    } else {
                        Some(pkg_name.clone())
                    };
                    results.push(CompletionResult {
                        name: name.clone(),
                        signature: entry.signature.clone(),
                        description,
                        insert_text: format!("{}(", name),
                        package,
                    });
                }
            }
        }

        results.truncate(20);
        results
    }

    /// Extract signature info for signature help.
    /// Returns all signatures (primary + alternatives) with params and summary.
    pub fn signature_info(
        &self,
        func_name: &str,
    ) -> Option<Vec<(String, Vec<String>, String)>> {
        let (_, entry) = self.get(func_name)?;

        if entry.symbol_type != "Function" {
            return None;
        }

        let mut sigs = Vec::new();
        // Primary signature
        let params = extract_params(&entry.signature);
        sigs.push((entry.signature.clone(), params, entry.summary.clone()));

        // Alternative signatures
        for alt in &entry.signatures {
            if alt != &entry.signature {
                let params = extract_params(alt);
                sigs.push((alt.clone(), params, entry.summary.clone()));
            }
        }

        Some(sigs)
    }

    /// Find similar symbol names using Levenshtein distance.
    pub fn find_similar(&self, name: &str, max_distance: usize) -> Vec<String> {
        let n = name.to_lowercase();
        let mut matches: Vec<(usize, String)> = self
            .names
            .iter()
            .filter_map(|(sym_name, _)| {
                let dist = levenshtein(&n, &sym_name.to_lowercase());
                if dist > 0 && dist <= max_distance {
                    Some((dist, sym_name.clone()))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by_key(|(d, _)| *d);
        matches.truncate(5);
        matches.into_iter().map(|(_, name)| name).collect()
    }

    /// Find deprecated/obsolete symbols by scanning summaries.
    pub fn find_deprecated(&self) -> Vec<DeprecationInfo> {
        self.symbols
            .values()
            .filter_map(|(_, entry)| {
                let summary_lower = entry.summary.to_lowercase();
                if summary_lower.contains("obsolete") || summary_lower.contains("deprecated") {
                    let replacement = extract_replacement(&summary_lower);
                    // Get the original-case name
                    let name = self
                        .names
                        .iter()
                        .find(|(n, _)| {
                            self.symbols
                                .get(&n.to_lowercase())
                                .map(|(_, e)| std::ptr::eq(e, entry))
                                .unwrap_or(false)
                        })
                        .map(|(n, _)| n.clone())
                        .unwrap_or_default();
                    Some(DeprecationInfo {
                        name,
                        description: entry.summary.clone(),
                        replacement,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Group symbols by category.
    pub fn by_category(&self) -> Vec<DocCategoryGroup> {
        let mut map: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for (name, pkg_name) in &self.names {
            if let Some((_, entry)) = self.symbols.get(&name.to_lowercase()) {
                let cat = entry
                    .category
                    .clone()
                    .unwrap_or_else(|| {
                        // Derive category from package name if not set
                        if pkg_name == "maxima-core-docs" {
                            "Other".to_string()
                        } else {
                            pkg_name.clone()
                        }
                    });
                map.entry(cat)
                    .or_default()
                    .push((name.clone(), entry.signature.clone()));
            }
        }
        map.into_iter()
            .map(|(category, symbols)| DocCategoryGroup { category, symbols })
            .collect()
    }

    /// Check if the store has any symbols loaded.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Number of symbols in the store.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_doc_index(path: &std::path::Path) -> Result<DocIndex, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("parse error: {}", e))
}

/// Resolve the Maxima user directory: $MAXIMA_USERDIR, else ~/.maxima/.
pub fn maxima_userdir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MAXIMA_USERDIR") {
        return Some(PathBuf::from(dir));
    }
    dirs::home_dir().map(|h| h.join(".maxima"))
}

/// Remove markdown image references (`![alt](src)`) from text.
///
/// VS Code hover tooltips can't render local or data-URI images, so these
/// would show as broken references. The full docs webview handles images
/// separately via base64 inlining.
fn strip_markdown_images(md: &str) -> String {
    let mut result = String::with_capacity(md.len());
    let mut chars = md.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch == '!' {
            // Peek ahead for ![...](...) pattern
            let mut buf = String::new();
            buf.push(chars.next().unwrap()); // !
            if chars.peek() == Some(&'[') {
                buf.push(chars.next().unwrap()); // [
                let mut depth = 1;
                while let Some(&c) = chars.peek() {
                    buf.push(chars.next().unwrap());
                    if c == '[' {
                        depth += 1;
                    } else if c == ']' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                if chars.peek() == Some(&'(') {
                    buf.push(chars.next().unwrap()); // (
                    let mut depth = 1;
                    while let Some(&c) = chars.peek() {
                        buf.push(chars.next().unwrap());
                        if c == '(' {
                            depth += 1;
                        } else if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                    // Skip trailing newlines left by removed image
                    while chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    // Entire ![...](...) consumed — don't append to result
                } else {
                    result.push_str(&buf);
                }
            } else {
                result.push_str(&buf);
            }
        } else {
            result.push(chars.next().unwrap());
        }
    }
    result
}

/// Extract parameter names from a signature like "func(a, b, c)".
fn extract_params(sig: &str) -> Vec<String> {
    let open = match sig.find('(') {
        Some(i) => i,
        None => return Vec::new(),
    };
    let close = match sig.rfind(')') {
        Some(i) => i,
        None => return Vec::new(),
    };
    if open + 1 >= close {
        return Vec::new();
    }
    let inner = &sig[open + 1..close];
    inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

/// Extract a replacement function name from deprecation text.
fn extract_replacement(desc_lower: &str) -> Option<String> {
    for prefix in &["replaced by ", "superseded by "] {
        if let Some(pos) = desc_lower.find(prefix) {
            let after = &desc_lower[pos + prefix.len()..];
            let name = extract_word(after);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
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

fn extract_word(s: &str) -> String {
    let trimmed = s.trim().trim_start_matches('`').trim_start_matches('\'');
    trimmed
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_params() {
        assert_eq!(
            extract_params("ax_draw2d(obj1, obj2, opt1)"),
            vec!["obj1", "obj2", "opt1"]
        );
        assert_eq!(extract_params("quit()"), Vec::<String>::new());
    }

    #[test]
    fn test_deserialize_symbol_entry() {
        let json = r#"{
            "type": "Function",
            "signature": "ax_draw2d(obj1, obj2)",
            "summary": "Main 2D plotting command.",
            "body_md": "Full docs here.",
            "body_html": "<p>Full docs here.</p>",
            "examples": [{"input": "ax_draw2d(ax_line([1,2],[3,4]))$", "output": "", "description": ""}],
            "see_also": ["ax_draw3d"]
        }"#;
        let entry: SymbolEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.symbol_type, "Function");
        assert_eq!(entry.signature, "ax_draw2d(obj1, obj2)");
        assert_eq!(entry.examples.len(), 1);
        assert_eq!(entry.see_also, vec!["ax_draw3d"]);
    }

    #[test]
    fn test_deserialize_symbol_entry_with_new_fields() {
        let json = r#"{
            "type": "Function",
            "signature": "diff(expr, var)",
            "summary": "Computes the derivative.",
            "body_md": "",
            "examples": [],
            "see_also": [],
            "category": "Calculus",
            "section": "Differentiation",
            "keywords": ["derivative", "differentiation"],
            "signatures": ["diff(expr, var)", "diff(expr, var, n)"]
        }"#;
        let entry: SymbolEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.category.as_deref(), Some("Calculus"));
        assert_eq!(entry.section.as_deref(), Some("Differentiation"));
        assert_eq!(entry.keywords, vec!["derivative", "differentiation"]);
        assert_eq!(entry.signatures, vec!["diff(expr, var)", "diff(expr, var, n)"]);
    }

    #[test]
    fn test_deserialize_doc_index() {
        let json = r#"{
            "version": 1,
            "package": "test-pkg",
            "source": "doc/test-pkg.md",
            "symbols": {
                "test_func": {
                    "type": "Function",
                    "signature": "test_func(x)",
                    "summary": "A test function.",
                    "body_md": "Does testing.",
                    "body_html": "<p>Does testing.</p>",
                    "examples": [],
                    "see_also": []
                }
            },
            "sections": []
        }"#;
        let index: DocIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.package, "test-pkg");
        assert_eq!(index.symbols.len(), 1);
        assert!(index.symbols.contains_key("test_func"));
    }

    fn make_test_store() -> DocIndexStore {
        let json = r#"{
            "version": 1,
            "package": "test-pkg",
            "source": "doc/test-pkg.md",
            "symbols": {
                "test_func": {
                    "type": "Function",
                    "signature": "test_func(x, y)",
                    "summary": "A test function.",
                    "body_md": "Does testing.",
                    "body_html": "",
                    "examples": [],
                    "see_also": [],
                    "keywords": ["testing", "example"],
                    "signatures": ["test_func(x, y)", "test_func(x)"]
                }
            },
            "sections": []
        }"#;

        let index: DocIndex = serde_json::from_str(json).unwrap();
        let mut store = DocIndexStore {
            symbols: HashMap::new(),
            names: Vec::new(),
            indexed: Vec::new(),
            index: Index::new(3),
        };
        store.ingest_index(index);
        store.build_bm25_index();
        store
    }

    #[test]
    fn test_store_hover_and_complete() {
        let store = make_test_store();

        // Hover
        let hover = store.hover_markdown("test_func").unwrap();
        assert!(hover.contains("test_func(x, y)"));
        assert!(hover.contains("Does testing."));
        assert!(hover.contains("test-pkg"));

        // Case-insensitive
        assert!(store.hover_markdown("TEST_FUNC").is_some());

        // Complete
        let results = store.complete("test_");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_func");

        // Signature info — now returns Vec of signatures
        let sigs = store.signature_info("test_func").unwrap();
        assert_eq!(sigs.len(), 2);
        assert_eq!(sigs[0].0, "test_func(x, y)");
        assert_eq!(sigs[0].1, vec!["x", "y"]);
        assert_eq!(sigs[1].0, "test_func(x)");
    }

    #[test]
    fn test_find_similar() {
        let store = make_test_store();
        let similar = store.find_similar("test_fun", 3);
        assert!(similar.contains(&"test_func".to_string()));
    }

    #[test]
    fn test_embedded_core_index_loads() {
        // Verify the embedded JSON is valid
        let json = include_str!("core-doc-index.json");
        let index: DocIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.package, "maxima-core-docs");
    }

    #[test]
    fn test_ingest_overlay_overwrites() {
        let mut store = DocIndexStore {
            symbols: HashMap::new(),
            names: Vec::new(),
            indexed: Vec::new(),
            index: Index::new(3),
        };

        let base: DocIndex = serde_json::from_str(r#"{
            "version": 1,
            "package": "base",
            "source": "",
            "symbols": {
                "foo": {
                    "type": "Function",
                    "signature": "foo(x)",
                    "summary": "Original.",
                    "body_md": "",
                    "examples": [],
                    "see_also": []
                }
            },
            "sections": []
        }"#).unwrap();

        let overlay: DocIndex = serde_json::from_str(r#"{
            "version": 1,
            "package": "overlay",
            "source": "",
            "symbols": {
                "foo": {
                    "type": "Function",
                    "signature": "foo(x, y)",
                    "summary": "Updated.",
                    "body_md": "Full docs.",
                    "examples": [],
                    "see_also": []
                }
            },
            "sections": []
        }"#).unwrap();

        store.ingest_index(base);
        store.ingest_index(overlay);

        let (pkg, entry) = store.get("foo").unwrap();
        assert_eq!(pkg, "overlay");
        assert_eq!(entry.summary, "Updated.");
        assert_eq!(entry.signature, "foo(x, y)");
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
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
        assert_eq!(extract_replacement("this is obsolete"), None);
    }
}
