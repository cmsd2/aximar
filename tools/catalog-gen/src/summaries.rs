//! Help summaries in another language, from Maxima's manual in that
//! language (`doc/info/<lang>/`), for Aximar's help in the user's language.
//!
//! mxpm's `doc generate-core-docs --lang <lang>` builds the translated
//! manual into a language pack; Aximar keeps only each symbol's summary,
//! and the rest comes from the English index or an installed pack.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

#[derive(Serialize)]
struct Summaries {
    language: String,
    source: String,
    summaries: BTreeMap<String, String>,
}

/// Write `<output>` with the summaries of Maxima's manual in `lang`
pub fn generate(maxima_src: &Path, lang: &str, output: &Path) -> Result<usize, String> {
    let work = tempfile::tempdir().map_err(|e| e.to_string())?;
    let out = work.path().join("out");
    let result = Command::new("mxpm")
        .args(["doc", "generate-core-docs", "--lang", lang, "--maxima-src"])
        .arg(maxima_src)
        .arg("--output")
        .arg(&out)
        .output()
        .map_err(|e| format!("can't run mxpm (install it from cmsd2/maxima-mxpm): {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "mxpm doc generate-core-docs --lang {lang} failed ({}); it needs an mxpm with --lang:\n{}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        ));
    }

    let slim = out.join(format!("doc/maxima-core-docs-slim-doc-index.{lang}.json"));
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
