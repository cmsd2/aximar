use crate::notebooks::types::{Notebook, TemplateSummary};

const WELCOME: &str = include_str!("welcome.json");
const CALCULUS: &str = include_str!("calculus.json");
const LINEAR_ALGEBRA: &str = include_str!("linear-algebra.json");
const EQUATIONS: &str = include_str!("equations.json");
const PROGRAMMING: &str = include_str!("programming.json");
const PLOTTING: &str = include_str!("plotting.json");
const VECTOR_CALCULUS: &str = include_str!("vector-calculus.json");
const COMPLEX_NUMBERS: &str = include_str!("complex-numbers.json");
const OUTPUT_DISPLAY: &str = include_str!("output-display.json");
const INTERACTIVE_PLOTTING: &str = include_str!("interactive-plotting.json");
const THREE_D_PLOTTING: &str = include_str!("3d-plotting.json");
const STATISTICAL_PLOTTING: &str = include_str!("statistical-plotting.json");

fn load_all() -> Vec<Notebook> {
    [WELCOME, CALCULUS, LINEAR_ALGEBRA, EQUATIONS, PROGRAMMING, PLOTTING, INTERACTIVE_PLOTTING, THREE_D_PLOTTING, STATISTICAL_PLOTTING, VECTOR_CALCULUS, COMPLEX_NUMBERS, OUTPUT_DISPLAY]
        .iter()
        .map(|json| serde_json::from_str(json).expect("embedded notebook template must be valid JSON"))
        .collect()
}

fn template_id(nb: &Notebook) -> String {
    nb.metadata
        .aximar
        .as_ref()
        .and_then(|a| a.template_id.clone())
        .unwrap_or_default()
}

fn template_title(nb: &Notebook) -> String {
    nb.metadata
        .aximar
        .as_ref()
        .and_then(|a| a.title.clone())
        .unwrap_or_else(|| "Untitled".into())
}

fn template_description(nb: &Notebook) -> String {
    nb.metadata
        .aximar
        .as_ref()
        .and_then(|a| a.description.clone())
        .unwrap_or_default()
}

pub fn list_templates() -> Vec<TemplateSummary> {
    load_all()
        .iter()
        .map(|nb| TemplateSummary {
            id: template_id(nb),
            title: template_title(nb),
            description: template_description(nb),
            cell_count: nb.cells.len(),
        })
        .collect()
}

pub fn get_template(id: &str) -> Option<Notebook> {
    load_all().into_iter().find(|nb| template_id(nb) == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code cell in the built-in templates and the demo notebook
    /// must get past the Unicode table (docs/unicode-input.md).
    #[test]
    fn shipped_notebooks_use_only_translatable_unicode() {
        use crate::maxima::unicode::unicode_to_maxima;
        use crate::notebooks::types::{CellSource, CellType};

        let demo_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/examples/demo1.macnb");
        let demo: Notebook = serde_json::from_str(&std::fs::read_to_string(demo_path).unwrap()).unwrap();
        let mut notebooks = load_all();
        notebooks.push(demo);
        for nb in &notebooks {
            for cell in nb.cells.iter().filter(|c| c.cell_type == CellType::Code) {
                let input = match &cell.source {
                    CellSource::String(s) => s.clone(),
                    CellSource::Lines(lines) => lines.concat(),
                };
                if let Err(e) = unicode_to_maxima(&input) {
                    panic!("{}: {input:?}: {}", template_id(nb), e.0);
                }
            }
        }
    }

    #[test]
    fn test_templates_load() {
        let templates = list_templates();
        assert_eq!(templates.len(), 12);
        assert_eq!(templates[0].id, "welcome");
    }

    #[test]
    fn test_get_template() {
        let t = get_template("calculus");
        assert!(t.is_some());
        let nb = t.unwrap();
        assert_eq!(nb.nbformat, 4);
        assert!(!nb.cells.is_empty());
    }

    #[test]
    fn test_missing_template() {
        assert!(get_template("nonexistent").is_none());
    }

    #[test]
    fn test_notebook_format() {
        let nb = get_template("welcome").unwrap();
        assert_eq!(nb.metadata.kernelspec.name, "maxima");
        assert!(nb.metadata.aximar.is_some());
        // Cells should be code or markdown
        for cell in &nb.cells {
            assert!(
                cell.cell_type == crate::notebooks::types::CellType::Code
                    || cell.cell_type == crate::notebooks::types::CellType::Markdown,
            );
        }
    }
}
