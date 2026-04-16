use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use probly_search::score::bm25;
use probly_search::Index;
use serde::{Deserialize, Serialize};

use super::types::CompletionResult;

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
// DocIndexStore — aggregates symbols from all loaded doc-index files
// ---------------------------------------------------------------------------

/// Wrapper for a (name, package) pair that the BM25 index references by key.
struct IndexedSymbol {
    name: String,
    package: String,
    summary: String,
}

fn idx_name_extract(d: &IndexedSymbol) -> Vec<&str> {
    vec![d.name.as_str()]
}

fn idx_summary_extract(d: &IndexedSymbol) -> Vec<&str> {
    vec![d.summary.as_str()]
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
    /// BM25 index over name + summary
    index: Index<usize>,
}

impl DocIndexStore {
    /// Scan `~/.maxima/*/doc/*-doc-index.json` and load all doc-index files.
    pub fn load() -> Self {
        let mut store = DocIndexStore {
            symbols: HashMap::new(),
            names: Vec::new(),
            indexed: Vec::new(),
            index: Index::new(2),
        };

        let userdir = match maxima_userdir() {
            Some(d) => d,
            None => {
                eprintln!("[doc-index] Could not determine Maxima user directory");
                return store;
            }
        };

        if !userdir.is_dir() {
            return store;
        }

        let mut file_count = 0u32;

        let entries = match std::fs::read_dir(&userdir) {
            Ok(e) => e,
            Err(_) => return store,
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
                        let pkg_name = index.package.clone();
                        for (sym_name, entry) in index.symbols {
                            let key = sym_name.to_lowercase();
                            if !store.symbols.contains_key(&key) {
                                store
                                    .names
                                    .push((sym_name.clone(), pkg_name.clone()));
                                store.indexed.push(IndexedSymbol {
                                    name: sym_name.clone(),
                                    package: pkg_name.clone(),
                                    summary: entry.summary.clone(),
                                });
                                store
                                    .symbols
                                    .insert(key, (pkg_name.clone(), entry));
                            }
                        }
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

        // Build BM25 index: 2 fields — name (weight 3.0) and summary (weight 1.0)
        for (i, sym) in store.indexed.iter().enumerate() {
            store.index.add_document(
                &[idx_name_extract, idx_summary_extract],
                idx_tokenizer,
                i,
                sym,
            );
        }

        if file_count > 0 {
            eprintln!(
                "[doc-index] Loaded {} symbols from {} doc-index file(s)",
                store.symbols.len(),
                file_count
            );
        }

        store
    }

    /// BM25 search across symbol names and summaries.
    pub fn search(&self, query: &str) -> Vec<DocSearchResult> {
        if query.is_empty() || self.indexed.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<DocSearchResult> = self
            .index
            .query(query, &mut bm25::new(), idx_tokenizer, &[3.0, 1.0])
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
    pub fn hover_markdown(&self, name: &str) -> Option<String> {
        let (pkg_name, entry) = self.get(name)?;

        let mut md = format!("```maxima\n{}\n```\n\n", entry.signature);

        if !entry.body_md.is_empty() {
            md.push_str(&entry.body_md);
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
                    results.push(CompletionResult {
                        name: name.clone(),
                        signature: entry.signature.clone(),
                        description: format!("requires load(\"{}\")", pkg_name),
                        insert_text: format!("{}(", name),
                        package: Some(pkg_name.clone()),
                    });
                }
            }
        }

        results.truncate(20);
        results
    }

    /// Extract signature info for signature help: (signature, params, summary).
    pub fn signature_info(
        &self,
        func_name: &str,
    ) -> Option<(String, Vec<String>, String)> {
        let (_, entry) = self.get(func_name)?;

        if entry.symbol_type != "Function" {
            return None;
        }

        let params = extract_params(&entry.signature);
        Some((entry.signature.clone(), params, entry.summary.clone()))
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

    #[test]
    fn test_store_hover_and_complete() {
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
                    "see_also": []
                }
            },
            "sections": []
        }"#;

        let index: DocIndex = serde_json::from_str(json).unwrap();
        let mut store = DocIndexStore {
            symbols: HashMap::new(),
            names: Vec::new(),
            indexed: Vec::new(),
            index: Index::new(2),
        };
        for (name, entry) in index.symbols {
            let key = name.to_lowercase();
            store.names.push((name.clone(), index.package.clone()));
            store.indexed.push(IndexedSymbol {
                name: name.clone(),
                package: index.package.clone(),
                summary: entry.summary.clone(),
            });
            store.symbols.insert(key, (index.package.clone(), entry));
        }
        for (i, sym) in store.indexed.iter().enumerate() {
            store.index.add_document(
                &[idx_name_extract, idx_summary_extract],
                idx_tokenizer,
                i,
                sym,
            );
        }

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

        // Signature info
        let (sig, params, summary) = store.signature_info("test_func").unwrap();
        assert_eq!(sig, "test_func(x, y)");
        assert_eq!(params, vec!["x", "y"]);
        assert_eq!(summary, "A test function.");
    }
}
