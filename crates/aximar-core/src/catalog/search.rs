use std::collections::{BTreeMap, HashMap};

use crate::catalog::doc_index::{
    DocCategoryGroup, DocIndex, DocIndexStore, DocSearchResult, ExampleEntry, SymbolEntry,
};
use crate::catalog::types::*;

/// Catalog wraps a [`DocIndexStore`] and provides the unified API for
/// searching, completing, and looking up Maxima function documentation.
///
/// On load it ingests ax-plotting functions (with full docs), the embedded
/// slim core doc-index, and any runtime doc-index files from `~/.maxima/`.
pub struct Catalog {
    store: DocIndexStore,
}

impl Catalog {
    pub fn load() -> Self {
        let mut store = DocIndexStore::new_empty();

        // 1. Ingest Aximar-specific plotting functions (with full body docs)
        let ax_json = include_str!("../maxima/ax_plotting_catalog.json");
        let ax_functions: Vec<MaximaFunction> =
            serde_json::from_str(ax_json).expect("embedded ax_plotting_catalog.json must be valid");
        let ax_docs_json = include_str!("../maxima/ax_plotting_docs.json");
        let ax_docs: HashMap<String, String> =
            serde_json::from_str(ax_docs_json).expect("embedded ax_plotting_docs.json must be valid");
        let ax_index = functions_to_doc_index("ax-plotting", &ax_functions, &ax_docs);
        store.ingest_index(ax_index);

        // 2. Load embedded core doc-index
        store.load_embedded_core();

        // 3. Load runtime doc-index files from ~/.maxima/ (overrides everything)
        store.load_runtime_files();

        // 4. Build BM25 search index
        store.build_bm25_index();

        Catalog { store }
    }

    /// Access the underlying [`DocIndexStore`].
    pub fn doc_index(&self) -> &DocIndexStore {
        &self.store
    }

    // ── Forwarded methods ────────────────────────────────────────────

    pub fn search(&self, query: &str) -> Vec<DocSearchResult> {
        self.store.search(query)
    }

    pub fn get(&self, name: &str) -> Option<&(String, SymbolEntry)> {
        self.store.get(name)
    }

    pub fn hover_markdown(&self, name: &str) -> Option<String> {
        self.store.hover_markdown(name)
    }

    pub fn complete(&self, prefix: &str) -> Vec<CompletionResult> {
        self.store.complete(prefix)
    }

    pub fn signature_info(
        &self,
        func_name: &str,
    ) -> Option<Vec<(String, Vec<String>, String)>> {
        self.store.signature_info(func_name)
    }

    pub fn find_similar(&self, name: &str, max_distance: usize) -> Vec<String> {
        self.store.find_similar(name, max_distance)
    }

    pub fn find_deprecated(&self) -> Vec<DeprecationInfo> {
        self.store.find_deprecated()
    }

    pub fn by_category(&self) -> Vec<DocCategoryGroup> {
        self.store.by_category()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }
}

/// Convert a list of [`MaximaFunction`] into a [`DocIndex`] for ingestion.
/// The `docs` map provides optional full markdown body for each function.
fn functions_to_doc_index(
    package: &str,
    functions: &[MaximaFunction],
    docs: &HashMap<String, String>,
) -> DocIndex {
    let mut symbols = BTreeMap::new();
    for f in functions {
        let primary_sig = f.signatures.first().cloned().unwrap_or_default();
        let alt_sigs: Vec<String> = f.signatures.iter().skip(1).cloned().collect();
        symbols.insert(
            f.name.clone(),
            SymbolEntry {
                symbol_type: "Function".to_string(),
                signature: primary_sig,
                summary: f.description.clone(),
                body_md: docs.get(&f.name).cloned().unwrap_or_default(),
                body_html: String::new(),
                examples: f
                    .examples
                    .iter()
                    .map(|e| ExampleEntry {
                        input: e.input.clone(),
                        output: String::new(),
                        description: e.description.clone().unwrap_or_default(),
                    })
                    .collect(),
                see_also: f.see_also.clone(),
                category: Some(f.category.label().to_string()),
                section: None,
                keywords: if f.search_keywords.is_empty() {
                    vec![f.category.label().to_lowercase()]
                } else {
                    f.search_keywords
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect()
                },
                signatures: alt_sigs,
            },
        );
    }
    DocIndex {
        version: 1,
        package: package.to_string(),
        source: String::new(),
        symbols,
        sections: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_loads() {
        let catalog = Catalog::load();
        assert!(catalog.len() > 100);
    }

    #[test]
    fn test_search_exact() {
        let catalog = Catalog::load();
        let results = catalog.search("integrate");
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "integrate");
    }

    #[test]
    fn test_search_prefix() {
        let catalog = Catalog::load();
        let results = catalog.search("integ");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.name == "integrate"));
    }

    #[test]
    fn test_search_description() {
        let catalog = Catalog::load();
        let results = catalog.search("differential");
        assert!(!results.is_empty());
        assert!(
            results.iter().any(|r| r.name == "diff"),
            "expected diff in results for 'differential', got: {:?}",
            results.iter().map(|r| &r.name).collect::<Vec<_>>()
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
        let result = catalog.get("solve");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_similar() {
        let catalog = Catalog::load();
        let similar = catalog.find_similar("intgrate", 3);
        assert!(similar.contains(&"integrate".to_string()));
    }

    #[test]
    fn test_find_deprecated() {
        let catalog = Catalog::load();
        let deprecated = catalog.find_deprecated();
        assert!(!deprecated.is_empty(), "expected at least one deprecated function");
        for info in &deprecated {
            assert!(!info.name.is_empty());
            assert!(!info.description.is_empty());
        }
    }

    #[test]
    fn test_by_category() {
        let catalog = Catalog::load();
        let groups = catalog.by_category();
        assert!(!groups.is_empty());
        assert!(groups.iter().any(|g| g.category == "Calculus"));
    }

    #[test]
    fn test_ax_plotting_functions_in_catalog() {
        let catalog = Catalog::load();
        assert!(catalog.get("ax_plot2d").is_some());
        assert!(catalog.get("ax_draw2d").is_some());
        assert!(catalog.get("ax_draw3d").is_some());
    }

    #[test]
    fn test_ax_plotting_functions_complete() {
        let catalog = Catalog::load();
        let results = catalog.complete("ax_");
        assert!(results.iter().any(|c| c.name == "ax_plot2d"));
        assert!(results.iter().any(|c| c.name == "ax_draw2d"));
        assert!(results.iter().any(|c| c.name == "ax_draw3d"));
    }

    #[test]
    fn test_ax_plotting_functions_searchable() {
        let catalog = Catalog::load();
        let results = catalog.search("ax_plot2d");
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "ax_plot2d");
    }
}
