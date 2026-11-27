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
    /// The language of the index's text, a BCP 47 tag such as `es`; an
    /// index without one is English
    #[serde(default = "english")]
    pub language: String,
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

/// The language every chain ends with, and an index's language by default
pub const ENGLISH: &str = "en";

fn english() -> String {
    ENGLISH.to_string()
}

/// A language tag as the store keys it: `pt-BR` and `pt_br` are one language
fn language_key(tag: &str) -> String {
    tag.trim().replace('_', "-").to_lowercase()
}

// ---------------------------------------------------------------------------
// Languages: the chain a lookup answers in, and what it found
// ---------------------------------------------------------------------------

/// The languages to answer in, in order, ending with English: the reader's
/// language, then English where there's no translation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageChain(Vec<String>);

impl LanguageChain {
    /// `languages` in order, then English, without repeats
    pub fn new<S: AsRef<str>>(languages: &[S]) -> Self {
        let mut chain: Vec<String> = Vec::new();
        for tag in languages.iter().map(|l| language_key(l.as_ref())) {
            if !tag.is_empty() && !chain.contains(&tag) {
                chain.push(tag);
            }
        }
        if !chain.iter().any(|l| l == ENGLISH) {
            chain.push(ENGLISH.to_string());
        }
        LanguageChain(chain)
    }

    /// The chain for an editor's or system's locale, such as `es-ES`: the
    /// locale, then its language alone (`es`), then English
    pub fn for_locale(locale: &str) -> Self {
        let tag = language_key(locale);
        let base = tag.split('-').next().unwrap_or_default().to_string();
        LanguageChain::new(&[tag, base])
    }

    /// English alone, as lookups answered before languages
    pub fn english() -> Self {
        LanguageChain(vec![ENGLISH.to_string()])
    }

    /// The reader's language: the first in the chain
    pub fn first(&self) -> &str {
        &self.0[0]
    }

    pub fn languages(&self) -> &[String] {
        &self.0
    }
}

/// A symbol's documentation in one language other than English
#[derive(Debug, Clone)]
enum Translation {
    /// A full entry, from an installed `<stem>-doc-index.<lang>.json`
    Full {
        package: String,
        entry: Box<SymbolEntry>,
    },
    /// A summary alone, such as the summaries Aximar embeds
    Summary(String),
}

/// A symbol's documentation in a language chain
#[derive(Debug, Clone)]
pub struct Resolved {
    pub package: String,
    /// The entry to show: a translation where there is one, with its
    /// summary from the first language that has one
    pub entry: SymbolEntry,
    /// The language of `entry.summary`
    pub summary_language: String,
    /// The language of the rest: the body, signatures and examples
    pub body_language: String,
    /// The chain's first language, when the symbol has nothing in it
    pub fallback_from: Option<String>,
    /// A translation whose signatures differ from the English entry's, so
    /// it may describe an older version
    pub stale: bool,
    /// The English signatures, for a stale translation
    pub english_signatures: Vec<String>,
}

/// A symbol's signatures as a set, the primary one included
fn signature_set(entry: &SymbolEntry) -> std::collections::BTreeSet<String> {
    let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    std::iter::once(&entry.signature)
        .chain(entry.signatures.iter())
        .filter(|s| !s.is_empty())
        .map(|s| squash(s))
        .collect()
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
    /// The summary in other languages, so search finds a symbol by either
    translated: String,
    keywords: String,
}

fn idx_name_extract(d: &IndexedSymbol) -> Vec<&str> {
    vec![d.name.as_str()]
}

fn idx_summary_extract(d: &IndexedSymbol) -> Vec<&str> {
    vec![d.summary.as_str(), d.translated.as_str()]
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
    /// symbol_name (lowercase) → (package_name, SymbolEntry), in English
    symbols: HashMap<String, (String, SymbolEntry)>,
    /// language → symbol_name (lowercase) → its documentation in that
    /// language, for every language but English
    translations: HashMap<String, HashMap<String, Translation>>,
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
            translations: HashMap::new(),
            names: Vec::new(),
            indexed: Vec::new(),
            index: Index::new(3),
        }
    }

    /// Load the embedded slim core doc-index as baseline, with the
    /// summaries Aximar embeds in other languages, then overlay runtime
    /// doc-index files from `~/.maxima/`.
    pub fn load() -> Self {
        let mut store = Self::new_empty();
        store.load_embedded_core();
        store.load_embedded_summaries();
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

    /// The help summaries Aximar embeds in other languages, from Maxima's
    /// manual in each (`core-doc-summaries.<lang>.json`), as a layer under
    /// any full documentation installed in that language
    pub fn load_embedded_summaries(&mut self) {
        for language in super::summaries::languages() {
            for (name, summary) in super::summaries::all(language) {
                self.ingest_summary(language, name, summary);
            }
        }
    }

    /// One symbol's summary in a language other than English.  A full
    /// translation already loaded keeps its own summary.
    pub fn ingest_summary(&mut self, language: &str, name: &str, summary: &str) {
        let lang = language_key(language);
        if lang == ENGLISH {
            return;
        }
        let entries = self.translations.entry(lang).or_default();
        entries
            .entry(name.to_lowercase())
            .or_insert_with(|| Translation::Summary(summary.to_string()));
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
        self.load_runtime_dir(&userdir);
    }

    /// Load the doc-index files of every package in a Maxima user
    /// directory: `<package>/doc/<stem>-doc-index.json` in English, and
    /// `<stem>-doc-index.<lang>.json` in another language, preferring each
    /// full index over its `-slim-` variant.
    pub fn load_runtime_dir(&mut self, userdir: &std::path::Path) {
        if !userdir.is_dir() {
            return;
        }

        use std::collections::HashMap as StdHashMap;

        // Collect doc-index files, pairing slim variants with their full
        // counterparts. For each base stem we load the full file if present,
        // falling back to the slim variant.
        //
        // Stem is derived by stripping `-slim-doc-index.json` or
        // `-doc-index.json` from the filename.
        let mut by_stem: StdHashMap<
            (String, String),
            (Option<std::path::PathBuf>, Option<std::path::PathBuf>),
        > = StdHashMap::new();

        let entries = match std::fs::read_dir(userdir) {
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
                let name = file_name.to_string_lossy().to_string();
                let Some((stem, slim, lang)) = doc_index_file_name(&name) else {
                    continue;
                };

                let path = file_entry.path();
                let pair = by_stem.entry((stem, lang)).or_default();
                if slim {
                    pair.0 = Some(path);
                } else {
                    pair.1 = Some(path);
                }
            }
        }

        let mut file_count = 0u32;
        for ((_, lang), (slim, full)) in &by_stem {
            // Prefer full doc-index; fall back to slim
            let path = full.as_ref().or(slim.as_ref());
            let path = match path {
                Some(p) => p,
                None => continue,
            };
            match load_doc_index(path) {
                Ok(index) if language_key(&index.language) != *lang => {
                    eprintln!(
                        "[doc-index] Skipped {}: its name says {} but it holds {}",
                        path.display(),
                        lang,
                        index.language
                    );
                }
                Ok(index) => {
                    self.ingest_index(index);
                    file_count += 1;
                }
                Err(e) => {
                    eprintln!("[doc-index] Failed to load {}: {}", path.display(), e);
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
    /// for the same symbol name in the same language.
    pub fn ingest_index(&mut self, index: DocIndex) {
        let lang = language_key(&index.language);
        if lang != ENGLISH {
            let entries = self.translations.entry(lang).or_default();
            for (sym_name, entry) in index.symbols {
                entries.insert(
                    sym_name.to_lowercase(),
                    Translation::Full {
                        package: index.package.clone(),
                        entry: Box::new(entry),
                    },
                );
            }
            return;
        }
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
                translated: String::new(),
                keywords: entry.keywords.join(" "),
            });
            self.symbols.insert(key, (pkg_name.clone(), entry));
        }
    }

    /// Build the BM25 index from the current indexed symbols, with each
    /// symbol's text in every other language, so search finds it by either.
    pub fn build_bm25_index(&mut self) {
        for sym in &mut self.indexed {
            let key = sym.name.to_lowercase();
            let mut languages: Vec<&String> = self.translations.keys().collect();
            languages.sort();
            sym.translated = languages
                .into_iter()
                .filter_map(|lang| match self.translations[lang].get(&key)? {
                    Translation::Full { entry, .. } => {
                        Some(format!("{} {}", entry.summary, entry.keywords.join(" ")))
                    }
                    Translation::Summary(summary) => Some(summary.clone()),
                })
                .collect::<Vec<_>>()
                .join(" ");
        }
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
                    score: qr.score,
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.total_cmp(&a.score));
        results.truncate(20);
        results
    }

    /// Look up a symbol by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&(String, SymbolEntry)> {
        self.symbols.get(&name.to_lowercase())
    }

    /// A symbol's documentation in a language chain, resolved for this
    /// symbol alone: a full translation in the first language that has
    /// one, else the English entry with the summary from the first
    /// language that has one.  None when the symbol has no English entry
    /// and no full translation.
    pub fn resolve(&self, name: &str, chain: &LanguageChain) -> Option<Resolved> {
        let key = name.to_lowercase();
        let english = self.symbols.get(&key);
        let mut summary: Option<(&str, &str)> = None;
        for lang in chain.languages() {
            if lang == ENGLISH {
                break;
            }
            match self.translations.get(lang).and_then(|t| t.get(&key)) {
                Some(Translation::Full { package, entry }) => {
                    let stale =
                        english.is_some_and(|(_, e)| signature_set(e) != signature_set(entry));
                    let english_signatures = match english {
                        Some((_, e)) if stale => signature_set(e).into_iter().collect(),
                        _ => Vec::new(),
                    };
                    let mut entry = (**entry).clone();
                    let mut summary_language = lang.clone();
                    // A translation without a summary of its own
                    if entry.summary.is_empty() {
                        if let Some((l, s)) = summary {
                            entry.summary = s.to_string();
                            summary_language = l.to_string();
                        }
                    }
                    return Some(Resolved {
                        package: package.clone(),
                        entry,
                        summary_language,
                        body_language: lang.clone(),
                        fallback_from: (lang != chain.first()).then(|| chain.first().to_string()),
                        stale,
                        english_signatures,
                    });
                }
                Some(Translation::Summary(s)) if summary.is_none() => summary = Some((lang, s)),
                _ => {}
            }
        }
        let (package, english) = english?;
        let mut entry = english.clone();
        let summary_language = match summary {
            Some((lang, s)) => {
                entry.summary = s.to_string();
                lang.to_string()
            }
            None => ENGLISH.to_string(),
        };
        Some(Resolved {
            package: package.clone(),
            entry,
            fallback_from: (summary_language != chain.first()).then(|| chain.first().to_string()),
            summary_language,
            body_language: ENGLISH.to_string(),
            stale: false,
            english_signatures: Vec::new(),
        })
    }

    /// BM25 search, matching every language's text, with each result's
    /// summary in the chain
    pub fn search_in(&self, query: &str, chain: &LanguageChain) -> Vec<DocSearchResult> {
        let mut results = self.search(query);
        for r in &mut results {
            if let Some(resolved) = self.resolve(&r.name, chain) {
                r.summary = resolved.entry.summary;
                r.signature = resolved.entry.signature;
            }
        }
        results
    }

    /// Prefix completion, with summaries in the chain
    pub fn complete_in(&self, prefix: &str, chain: &LanguageChain) -> Vec<CompletionResult> {
        let mut results = self.complete(prefix);
        for r in results.iter_mut().filter(|r| r.package.is_none()) {
            if let Some(resolved) = self.resolve(&r.name, chain) {
                r.description = resolved.entry.summary;
            }
        }
        results
    }

    /// Signature help in the chain: the summary in the reader's language,
    /// and the English signatures when a translation's differ
    pub fn signature_info_in(
        &self,
        func_name: &str,
        chain: &LanguageChain,
    ) -> Option<Vec<(String, Vec<String>, String)>> {
        let mut sigs = self.signature_info(func_name)?;
        if let Some(resolved) = self.resolve(func_name, chain) {
            for (_, _, summary) in &mut sigs {
                summary.clone_from(&resolved.entry.summary);
            }
        }
        Some(sigs)
    }

    /// Hover markdown in the chain.  A translated summary above
    /// documentation in another language comes first, so the reader sees
    /// their language.
    pub fn hover_markdown_in(&self, name: &str, chain: &LanguageChain) -> Option<String> {
        let resolved = self.resolve(name, chain)?;
        let mut md = hover_markdown_for(&resolved.package, &resolved.entry);
        let summary = &resolved.entry.summary;
        if resolved.summary_language != resolved.body_language
            && !resolved.entry.body_md.is_empty()
            && !summary.is_empty()
        {
            // After the signature block
            let at = md.find("```\n\n").map_or(0, |i| i + 5);
            md.insert_str(at, &format!("{summary}\n\n---\n\n"));
        }
        Some(md)
    }

    /// Build hover markdown for a symbol.
    ///
    /// Images are stripped since VS Code hover tooltips can't render them.
    pub fn hover_markdown(&self, name: &str) -> Option<String> {
        let (pkg_name, entry) = self.get(name)?;
        Some(hover_markdown_for(pkg_name, entry))
    }

    /// The languages, other than English, the store has any text in
    pub fn languages(&self) -> Vec<String> {
        let mut languages: Vec<String> = self.translations.keys().cloned().collect();
        languages.sort();
        languages
    }

    /// Whether the store has full documentation, not only summaries, in a
    /// language
    pub fn has_full_documentation(&self, language: &str) -> bool {
        let lang = language_key(language);
        lang == ENGLISH
            || self
                .translations
                .get(&lang)
                .is_some_and(|t| t.values().any(|e| matches!(e, Translation::Full { .. })))
    }
}

/// Hover markdown for one entry: signature, documentation, examples
fn hover_markdown_for(pkg_name: &str, entry: &SymbolEntry) -> String {
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
        md.push_str(&format!("\n**See also:** {}", entry.see_also.join(", ")));
    }

    md.push_str(&format!("\n\n*From package `{}`*", pkg_name));

    md
}

impl DocIndexStore {
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
    pub fn signature_info(&self, func_name: &str) -> Option<Vec<(String, Vec<String>, String)>> {
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
                let cat = entry.category.clone().unwrap_or_else(|| {
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

/// A doc-index file's stem, whether it's the slim variant, and the
/// language its name gives: `core-doc-index.json` is `("core", false,
/// "en")`, `core-slim-doc-index.es.json` is `("core", true, "es")`
fn doc_index_file_name(name: &str) -> Option<(String, bool, String)> {
    let base = name.strip_suffix(".json")?;
    let (rest, lang) = match base.rsplit_once('.') {
        Some((rest, lang)) if rest.ends_with("-doc-index") && !lang.is_empty() => {
            (rest, language_key(lang))
        }
        _ => (base, ENGLISH.to_string()),
    };
    if let Some(stem) = rest.strip_suffix("-slim-doc-index") {
        Some((stem.to_string(), true, lang))
    } else {
        rest.strip_suffix("-doc-index")
            .map(|stem| (stem.to_string(), false, lang))
    }
}

fn load_doc_index(path: &std::path::Path) -> Result<DocIndex, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
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

pub(crate) fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
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
mod language_tests {
    use super::*;
    use std::path::Path;

    /// A doc index for `package` in `language` (None for no field), with
    /// entries `(name, signatures, summary, body)`
    fn index_json(
        package: &str,
        language: Option<&str>,
        entries: &[(&str, &[&str], &str, &str)],
    ) -> String {
        let symbols: serde_json::Map<String, serde_json::Value> = entries
            .iter()
            .map(|(name, sigs, summary, body)| {
                (
                    name.to_string(),
                    serde_json::json!({
                        "type": "Function",
                        "signature": sigs[0],
                        "signatures": sigs,
                        "summary": summary,
                        "body_md": body,
                    }),
                )
            })
            .collect();
        let mut index = serde_json::json!({ "version": 1, "package": package, "symbols": symbols });
        if let Some(lang) = language {
            index["language"] = lang.into();
        }
        index.to_string()
    }

    fn write(userdir: &Path, package: &str, file: &str, json: &str) {
        let doc = userdir.join(package).join("doc");
        std::fs::create_dir_all(&doc).unwrap();
        std::fs::write(doc.join(file), json).unwrap();
    }

    const INTEGRATE_EN: (&str, &[&str], &str, &str) = (
        "integrate",
        &["integrate(expr, x)", "integrate(expr, x, a, b)"],
        "Attempts to symbolically compute the integral.",
        "The full English documentation.",
    );
    const FFT_EN: (&str, &[&str], &str, &str) =
        ("fft", &["fft(x)"], "Fast Fourier transform.", "About fft.");

    /// A store with English core docs, as maxima-core-docs installs them
    fn store_with_english(userdir: &Path) -> DocIndexStore {
        write(
            userdir,
            "maxima-core-docs",
            "maxima-core-docs-doc-index.json",
            &index_json("maxima-core-docs", None, &[INTEGRATE_EN, FFT_EN]),
        );
        let mut store = DocIndexStore::new_empty();
        store.load_runtime_dir(userdir);
        store
    }

    fn spanish() -> LanguageChain {
        LanguageChain::new(&["es"])
    }

    #[test]
    fn a_chain_ends_with_english_once() {
        assert_eq!(LanguageChain::new(&["es"]).languages(), ["es", "en"]);
        assert_eq!(
            LanguageChain::new(&["pt_BR", "en"]).languages(),
            ["pt-br", "en"]
        );
        assert_eq!(LanguageChain::new::<&str>(&[]).languages(), ["en"]);
        assert_eq!(LanguageChain::english().first(), "en");
    }

    #[test]
    fn file_names_give_stem_variant_and_language() {
        let parse = doc_index_file_name;
        assert_eq!(
            parse("core-doc-index.json"),
            Some(("core".into(), false, "en".into()))
        );
        assert_eq!(
            parse("core-slim-doc-index.json"),
            Some(("core".into(), true, "en".into()))
        );
        assert_eq!(
            parse("core-doc-index.es.json"),
            Some(("core".into(), false, "es".into()))
        );
        assert_eq!(
            parse("core-slim-doc-index.pt-BR.json"),
            Some(("core".into(), true, "pt-br".into()))
        );
        assert_eq!(parse("notes.json"), None);
    }

    #[test]
    fn an_index_without_a_language_is_english() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_with_english(dir.path());
        let integrate = store
            .resolve("integrate", &LanguageChain::english())
            .unwrap();
        assert_eq!(integrate.body_language, "en");
        assert_eq!(integrate.entry.body_md, "The full English documentation.");
        assert!(store.languages().is_empty());
    }

    #[test]
    fn a_translated_index_loads_by_name() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "maxima-core-docs-es",
            "maxima-core-docs-doc-index.es.json",
            &index_json(
                "maxima-core-docs-es",
                Some("es"),
                &[(
                    "integrate",
                    &["integrate(expr, x)", "integrate(expr, x, a, b)"],
                    "Calcula la integral.",
                    "La documentación en español.",
                )],
            ),
        );
        let store = store_with_english(dir.path());
        assert_eq!(store.languages(), ["es"]);
        assert!(store.has_full_documentation("es"));
        // English lookups are as before
        assert_eq!(
            store.get("integrate").unwrap().1.body_md,
            "The full English documentation."
        );
    }

    #[test]
    fn a_mismatched_index_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "maxima-core-docs-es",
            "maxima-core-docs-doc-index.es.json",
            &index_json(
                "maxima-core-docs-es",
                Some("de"),
                &[(
                    "integrate",
                    &["integrate(expr, x)"],
                    "Integral.",
                    "Deutsch.",
                )],
            ),
        );
        let store = store_with_english(dir.path());
        assert!(store.languages().is_empty());
        assert_eq!(
            store
                .resolve("integrate", &spanish())
                .unwrap()
                .body_language,
            "en"
        );
    }

    #[test]
    fn a_translated_symbol_comes_first() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "maxima-core-docs-es",
            "maxima-core-docs-doc-index.es.json",
            &index_json(
                "maxima-core-docs-es",
                Some("es"),
                &[(
                    "integrate",
                    &["integrate(expr, x)", "integrate(expr, x, a, b)"],
                    "Calcula la integral.",
                    "La documentación en español.",
                )],
            ),
        );
        let store = store_with_english(dir.path());
        let integrate = store.resolve("integrate", &spanish()).unwrap();
        assert_eq!(integrate.entry.body_md, "La documentación en español.");
        assert_eq!(
            (
                integrate.summary_language.as_str(),
                integrate.body_language.as_str()
            ),
            ("es", "es")
        );
        assert_eq!(integrate.fallback_from, None);
        assert!(!integrate.stale);
    }

    #[test]
    fn a_missing_translation_falls_back_to_english() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "maxima-core-docs-es",
            "maxima-core-docs-doc-index.es.json",
            &index_json(
                "maxima-core-docs-es",
                Some("es"),
                &[("integrate", &["integrate(expr, x)"], "Calcula.", "Español.")],
            ),
        );
        let store = store_with_english(dir.path());
        let fft = store.resolve("fft", &spanish()).unwrap();
        assert_eq!(fft.entry.body_md, "About fft.");
        assert_eq!(fft.body_language, "en");
        assert_eq!(fft.fallback_from.as_deref(), Some("es"));
    }

    #[test]
    fn differing_signatures_mark_a_translation_stale() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "maxima-core-docs-es",
            "maxima-core-docs-doc-index.es.json",
            &index_json(
                "maxima-core-docs-es",
                Some("es"),
                &[(
                    "integrate",
                    &["integrate(expr, x)"],
                    "Calcula.",
                    "Español antiguo.",
                )],
            ),
        );
        let store = store_with_english(dir.path());
        let integrate = store.resolve("integrate", &spanish()).unwrap();
        assert!(integrate.stale);
        assert_eq!(
            integrate.english_signatures,
            ["integrate(expr, x)", "integrate(expr, x, a, b)"]
        );
    }

    #[test]
    fn search_matches_either_language() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "maxima-core-docs-es",
            "maxima-core-docs-doc-index.es.json",
            &index_json(
                "maxima-core-docs-es",
                Some("es"),
                &[(
                    "integrate",
                    &["integrate(expr, x)", "integrate(expr, x, a, b)"],
                    "Calcula la integral definida.",
                    "Español.",
                )],
            ),
        );
        let mut store = store_with_english(dir.path());
        store.build_bm25_index();
        let hits = store.search_in("integral definida", &spanish());
        let integrate = hits
            .iter()
            .find(|h| h.name == "integrate")
            .expect("integrate found by its Spanish text");
        assert_eq!(integrate.summary, "Calcula la integral definida.");
        let english = store.search_in("integral", &LanguageChain::english());
        assert_eq!(
            english
                .iter()
                .find(|h| h.name == "integrate")
                .unwrap()
                .summary,
            "Attempts to symbolically compute the integral."
        );
    }

    #[test]
    fn a_summary_layer_sits_above_english_documentation() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = DocIndexStore::new_empty();
        store.ingest_summary("es", "integrate", "Calcula simbólicamente la integral.");
        // maxima-core-docs installed: full English documentation
        write(
            dir.path(),
            "maxima-core-docs",
            "maxima-core-docs-doc-index.json",
            &index_json("maxima-core-docs", None, &[INTEGRATE_EN, FFT_EN]),
        );
        store.load_runtime_dir(dir.path());
        let integrate = store.resolve("integrate", &spanish()).unwrap();
        assert_eq!(
            integrate.entry.summary,
            "Calcula simbólicamente la integral."
        );
        assert_eq!(integrate.entry.body_md, "The full English documentation.");
        assert_eq!(
            (
                integrate.summary_language.as_str(),
                integrate.body_language.as_str()
            ),
            ("es", "en")
        );
        assert_eq!(integrate.fallback_from, None);
        assert!(!store.has_full_documentation("es"));
        // A full translation, installed later, takes over from the summary
        write(
            dir.path(),
            "maxima-core-docs-es",
            "maxima-core-docs-doc-index.es.json",
            &index_json(
                "maxima-core-docs-es",
                Some("es"),
                &[(
                    "integrate",
                    &["integrate(expr, x)", "integrate(expr, x, a, b)"],
                    "Calcula la integral.",
                    "Español.",
                )],
            ),
        );
        store.load_runtime_dir(dir.path());
        let integrate = store.resolve("integrate", &spanish()).unwrap();
        assert_eq!(integrate.entry.body_md, "Español.");
        assert_eq!(integrate.body_language, "es");
    }

    #[test]
    fn the_embedded_spanish_summaries_load() {
        let mut store = DocIndexStore::new_empty();
        store.load_embedded_core();
        store.load_embedded_summaries();
        let integrate = store.resolve("integrate", &spanish()).unwrap();
        assert!(
            integrate
                .entry
                .summary
                .starts_with("Calcula simbólicamente"),
            "{}",
            integrate.entry.summary
        );
        assert_eq!(integrate.summary_language, "es");
    }

    #[test]
    fn a_locale_falls_back_to_its_language() {
        assert_eq!(
            LanguageChain::for_locale("es-ES").languages(),
            ["es-es", "es", "en"]
        );
        assert_eq!(LanguageChain::for_locale("es").languages(), ["es", "en"]);
        assert_eq!(
            LanguageChain::for_locale("en-US").languages(),
            ["en-us", "en"]
        );
    }

    #[test]
    fn hover_puts_a_translated_summary_above_english_documentation() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store_with_english(dir.path());
        store.ingest_summary("es", "integrate", "Calcula la integral.");
        let md = store.hover_markdown_in("integrate", &spanish()).unwrap();
        let summary = md
            .find("Calcula la integral.")
            .expect("the Spanish summary");
        let body = md.find("The full English documentation.").unwrap();
        assert!(
            md.starts_with("```maxima\nintegrate(expr, x)\n```\n\n"),
            "{md}"
        );
        assert!(summary < body, "{md}");
        // English alone is as before
        let english = store
            .hover_markdown_in("integrate", &LanguageChain::english())
            .unwrap();
        assert_eq!(english, store.hover_markdown("integrate").unwrap());
    }
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
        assert_eq!(
            entry.signatures,
            vec!["diff(expr, var)", "diff(expr, var, n)"]
        );
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
            translations: HashMap::new(),
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
            translations: HashMap::new(),
            names: Vec::new(),
            indexed: Vec::new(),
            index: Index::new(3),
        };

        let base: DocIndex = serde_json::from_str(
            r#"{
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
        }"#,
        )
        .unwrap();

        let overlay: DocIndex = serde_json::from_str(
            r#"{
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
        }"#,
        )
        .unwrap();

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
