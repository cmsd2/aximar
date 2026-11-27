//! Help summaries in another language, from Maxima's manual in that
//! language (`doc/info/<lang>/`), for Aximar's help in the user's language.
//!
//! mxpm's `doc generate-core-docs` makes the English index from
//! `doc/info/`.  It runs unchanged on a copy of the source whose `doc/info`
//! is the translated manual, given the English tree's macro files, which
//! Maxima's own build finds through an include path.  Aximar keeps only
//! each symbol's summary; signatures and the rest come from the English
//! index.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

/// Files the translated manuals include from the English tree
const SHARED_FILES: &[&str] = &[
    "category-macros.texi",
    "category-macros.texi.in",
    "dlmf-ref-macros.texi",
];

#[derive(Serialize)]
struct Summaries {
    language: String,
    source: String,
    summaries: BTreeMap<String, String>,
}

/// Write `<output>` with the summaries of Maxima's manual in `lang`
pub fn generate(maxima_src: &Path, lang: &str, output: &Path) -> Result<usize, String> {
    let info = maxima_src.join("doc/info");
    let translated = info.join(lang);
    if !translated.join("maxima.texi").exists() {
        return Err(format!("no Maxima manual in {}", translated.display()));
    }

    let work = tempfile::tempdir().map_err(|e| e.to_string())?;
    let tree = work.path().join("maxima");
    let doc_info = tree.join("doc/info");
    copy_dir(&translated, &doc_info)?;
    for file in SHARED_FILES {
        let from = info.join(file);
        if from.exists() {
            fs::copy(&from, doc_info.join(file)).map_err(|e| format!("{file}: {e}"))?;
        }
    }
    let figures = info.join("figures");
    if figures.is_dir() {
        copy_dir(&figures, &doc_info.join("figures"))?;
    }
    // mxpm reads Maxima's version from it
    let configure = maxima_src.join("configure.ac");
    if configure.exists() {
        fs::copy(&configure, tree.join("configure.ac")).map_err(|e| e.to_string())?;
    }

    let out = work.path().join("out");
    let status = Command::new("mxpm")
        .args(["doc", "generate-core-docs", "--maxima-src"])
        .arg(&tree)
        .arg("--output")
        .arg(&out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("can't run mxpm (install it from cmsd2/maxima-mxpm): {e}"))?;
    if !status.success() {
        return Err(format!("mxpm doc generate-core-docs failed: {status}"));
    }

    let slim = out.join("doc/maxima-core-docs-slim-doc-index.json");
    let index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&slim).map_err(|e| format!("{}: {e}", slim.display()))?,
    )
    .map_err(|e| e.to_string())?;
    let summaries: BTreeMap<String, String> = index["symbols"]
        .as_object()
        .ok_or("the index has no symbols")?
        .iter()
        .filter_map(|(name, entry)| {
            let summary = entry["summary"].as_str()?.trim();
            (!summary.is_empty()).then(|| (name.clone(), summary.to_string()))
        })
        .collect();
    let count = summaries.len();

    let file = Summaries {
        language: lang.to_string(),
        source: format!("Maxima's manual, doc/info/{lang}/maxima.texi"),
        summaries,
    };
    let mut json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    json.push('\n');
    fs::write(output, json).map_err(|e| format!("{}: {e}", output.display()))?;
    Ok(count)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
