//! Opening and saving notebook files, for every front end: the window,
//! MCP and batch runs all go through these.

use std::fs;
use std::path::Path;

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::notebook::Notebook;
use crate::notebooks::format::{self, NotebookFile};

/// Read a notebook file, with friendly explanations for its errors.
pub fn open(
    path: &Path,
    catalog: &Catalog,
    packages: &PackageCatalog,
) -> Result<NotebookFile, String> {
    let json =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let mut file =
        format::read(&json).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
    format::explain_errors(&mut file, catalog, packages);
    Ok(file)
}

/// Write a notebook to a file, creating its directory if needed.
pub fn save(path: &Path, notebook: &Notebook) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    fs::write(path, format::write(notebook))
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::NotebookCommand;
    use crate::notebook::{CellOutput, CellType};

    #[test]
    fn a_saved_notebook_opens_with_its_ids_errors_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/calc.macnb");

        let mut nb = Notebook::new();
        let cell_id = nb.cells()[0].id.clone();
        nb.apply(NotebookCommand::UpdateCellInput {
            cell_id: cell_id.clone(),
            input: "1/0;".into(),
        })
        .unwrap();
        let error = "expt: undefined: 0 to a negative exponent.".to_string();
        let output = CellOutput {
            error: Some(error.clone()),
            is_error: true,
            execution_count: Some(1),
            ..Default::default()
        };
        nb.apply(NotebookCommand::SetCellOutput {
            cell_id: cell_id.clone(),
            output: Box::new(output),
            raw_output: vec![],
        })
        .unwrap();
        save(&path, &nb).unwrap();

        let file = open(&path, &Catalog::load(), &PackageCatalog::load()).unwrap();
        let cell = &file.cells[0];
        assert_eq!(
            (cell.id.as_str(), cell.cell_type, cell.input.as_str()),
            (cell_id.as_str(), CellType::Code, "1/0;")
        );
        let out = cell.output.as_ref().unwrap();
        assert!(out.is_error && out.stale);
        assert_eq!(out.error.as_deref(), Some(error.as_str()));
        assert_eq!(
            out.error_info.as_ref().unwrap().title.text,
            "Division by Zero"
        );
        assert!(file.metadata.contains_key("kernelspec"));
    }

    #[test]
    fn opening_a_missing_or_bad_file_says_which() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.macnb");
        let err = open(&missing, &Catalog::load(), &PackageCatalog::load()).unwrap_err();
        assert!(err.contains("missing.macnb"), "{err}");

        let bad = dir.path().join("bad.macnb");
        fs::write(&bad, r#"{"nbformat": 3, "cells": []}"#).unwrap();
        let err = open(&bad, &Catalog::load(), &PackageCatalog::load()).unwrap_err();
        assert!(
            err.contains("bad.macnb") && err.contains("nbformat 3"),
            "{err}"
        );
    }
}
