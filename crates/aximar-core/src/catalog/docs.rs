use std::collections::HashMap;

/// Full documentation store — maps function names to Markdown strings.
pub struct Docs {
    entries: HashMap<String, String>,
}

impl Docs {
    pub fn load() -> Self {
        let json = include_str!("docs.json");
        let mut entries: HashMap<String, String> =
            serde_json::from_str(json).expect("embedded docs.json must be valid");

        // Append Aximar-specific plotting function docs (alongside the .mac file)
        let ax_json = include_str!("../maxima/ax_plotting_docs.json");
        let ax_entries: HashMap<String, String> =
            serde_json::from_str(ax_json).expect("embedded ax_plotting_docs.json must be valid");
        entries.extend(ax_entries);

        Docs { entries }
    }

    /// Get the full Markdown documentation for a function by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        // Try exact match first, then case-insensitive
        if let Some(doc) = self.entries.get(name) {
            return Some(doc.as_str());
        }
        let lower = name.to_lowercase();
        for (k, v) in &self.entries {
            if k.to_lowercase() == lower {
                return Some(v.as_str());
            }
        }
        None
    }
}
