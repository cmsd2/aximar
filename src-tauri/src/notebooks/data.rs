use crate::notebooks::types::{Notebook, TemplateSummary};

const WELCOME: &str = include_str!("welcome.json");
const CALCULUS: &str = include_str!("calculus.json");
const LINEAR_ALGEBRA: &str = include_str!("linear-algebra.json");
const EQUATIONS: &str = include_str!("equations.json");
const PROGRAMMING: &str = include_str!("programming.json");

fn load_all() -> Vec<Notebook> {
    [WELCOME, CALCULUS, LINEAR_ALGEBRA, EQUATIONS, PROGRAMMING]
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

    #[test]
    fn test_templates_load() {
        let templates = list_templates();
        assert_eq!(templates.len(), 5);
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
