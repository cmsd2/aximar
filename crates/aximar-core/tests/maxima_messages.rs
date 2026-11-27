//! Aximar's explanations of Maxima's text against the catalog: every
//! message the code can produce is in `locales/messages.pot`, every entry
//! there is one the code produces, and `docs/maxima-messages.md` is the
//! list generated from the template.
//!
//! `scripts/i18n-extract` regenerates the list; by hand:
//! `AXIMAR_UPDATE_DOCS=1 cargo test -p aximar-core --test maxima_messages`

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::error::AppError;
use aximar_core::maxima::errors::{enhance_error_with_packages, explain_failure};
use aximar_core::maxima::notes::notes;
use regex::Regex;
use rspolib::{pofile, POEntry};

/// Raw errors that reach every explanation and every variant of its text
const SAMPLES: &[&str] = &[
    "expt: undefined: 0 to a negative exponent.",
    "incorrect syntax: Premature termination of input at ;.",
    "incorrect syntax: Missing )",
    "incorrect syntax: ! is not a prefix operator",
    "incorrect syntax: y is not an infix operator",
    "incorrect syntax: Too many )'s",
    "Too many arguments supplied to f(x); found: [1,2]",
    "Too few arguments supplied to integrate; found: 0",
    "Too few arguments supplied to myfunc",
    "Too many arguments supplied to myfunc",
    "unbound variable foo",
    "funcall: no such function: pdf_normal",
    "funcall: no such function: intgrate",
    "funcall: no such function: zzqqxx",
    "Maxima encountered a Lisp error: The value 1 is not of type LIST when binding LIST",
    "Maxima encountered a Lisp error: Control stack exhausted (no more space for function call frames).",
    "Maxima encountered a Lisp error: arithmetic error FLOATING-POINT-OVERFLOW signalled",
    "Maxima encountered a Lisp error: Heap exhausted",
    "defint: integral is divergent.",
    "solve: inconsistent equations",
    "Maxima needs to know: Is x positive, negative or zero?",
    "Maxima needs to know: Is x positive or zero?",
    "Maxima needs to know: Is x negative or zero?",
    "Maxima needs to know: Is x positive or negative?",
    "Maxima needs to know: Is x zero or nonzero?",
    "Maxima needs to know: Is a equal to -1?",
    "Maxima needs to know: Is n an integer?",
    "Maxima needs to know: Answer 1, 2, 3 or 4 :",
    "all rows must be the same length",
    "loadfile: failed to load mypackage",
    "loadfile: failed to load",
    "log: encountered log(0).",
    "part: fell off the end.",
    "part: argument must be a non-atomic expression; found x",
    "assignment: cannot assign to 2",
    "first: empty argument.",
    "allroots: expected a polynomial; found sin(x)",
    "algsys: Couldn't reduce system to a polynomial in one variable.",
];

/// Text output that reaches every note
const NOTE_SAMPLES: &[&str] = &[
    "rat: replaced 0.5 by 1/2 = 0.5",
    "rat: replaced 0.1 by 1/10 = 0.1\nrat: replaced 0.2 by 1/5 = 0.2\nrat: replaced 0.3 by 3/10 = 0.3\nrat: replaced 0.4 by 2/5 = 0.4",
    "solve: using arc-trig functions to get a solution.\nSome solutions will be lost.",
    "solve: dependent equations eliminated: (2)",
    "Principal Value",
    "plot2d: expression evaluates to non-numeric value somewhere in plotting range.",
    "quad_qags: Numerical evaluation of f at 0.0\nis not a float or is not defined",
    "expt: undefined: 0 to a negative exponent.",
    "tellsimp: warning: rule will treat '+' as noncommutative and nonassociative.",
];

/// Contexts of explanations' messages, as opposed to window text
const PREFIXES: &[&str] = &["error.", "question.", "note."];

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(\w+)\}").unwrap());

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The template's explanation entries, by context
fn template() -> BTreeMap<String, POEntry> {
    let po = pofile(root().join("locales/messages.pot").as_path()).expect("the template");
    po.entries
        .into_iter()
        .filter_map(|e| {
            let ctx = e.msgctxt.clone()?;
            PREFIXES
                .iter()
                .any(|p| ctx.starts_with(p))
                .then_some((ctx, e))
        })
        .collect()
}

fn placeholders(text: &str) -> BTreeSet<String> {
    PLACEHOLDER
        .captures_iter(text)
        .map(|c| c[1].to_string())
        .collect()
}

#[test]
fn the_catalog_and_the_code_agree() {
    let template = template();
    assert!(!template.is_empty());
    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());

    // Raw errors, then evaluations that didn't finish
    let mut explanations: Vec<(String, _)> = SAMPLES
        .iter()
        .map(|raw| {
            let info = enhance_error_with_packages(raw, &catalog, Some(&packages))
                .unwrap_or_else(|| panic!("no explanation for {raw:?}"));
            (raw.to_string(), info)
        })
        .collect();
    for failure in [
        AppError::Timeout(30),
        AppError::EvalCancelled(String::new()),
    ] {
        explanations.push((failure.to_string(), explain_failure(&failure).unwrap()));
    }

    // Every message, with what produced it, its explanation's ID and field
    let mut messages = Vec::new();
    for (raw, info) in explanations {
        for (field, m) in info.messages() {
            messages.push((raw.clone(), info.id.clone(), field, m.clone()));
        }
    }
    for text in NOTE_SAMPLES {
        let found = notes(text, true);
        assert_eq!(found.len(), 1, "one note for {text:?}");
        let note = &found[0];
        messages.push((
            text.to_string(),
            note.id.clone(),
            "text",
            note.message.clone(),
        ));
    }

    let mut produced = BTreeSet::new();
    for (raw, id, field, m) in &messages {
        assert!(
            m.id.starts_with(&format!("{id}.{field}")),
            "{raw:?}: {field}'s ID {} isn't under {id}",
            m.id
        );
        let entry = template
            .get(&m.id)
            .unwrap_or_else(|| panic!("{} isn't in the template: run scripts/i18n-extract", m.id));
        assert_eq!(entry.msgid, m.template, "{}'s English", m.id);
        let params: BTreeSet<String> = m.params.keys().cloned().collect();
        assert_eq!(placeholders(&m.template), params, "{}'s parameters", m.id);
        produced.insert(m.id.clone());
    }

    let unused: Vec<_> = template.keys().filter(|k| !produced.contains(*k)).collect();
    assert!(
        unused.is_empty(),
        "entries no sample produces (add a sample, or remove the message): {unused:?}"
    );
}

#[test]
fn each_explanation_says_what_maxima_text_it_covers() {
    for (ctx, entry) in template() {
        // An explanation's title, or a note's one message
        if ctx.ends_with(".title") || (ctx.starts_with("note.") && ctx.ends_with(".text")) {
            assert!(
                entry.comment.as_deref().is_some_and(|c| !c.is_empty()),
                "{ctx} has no `// i18n:` note saying what Maxima prints"
            );
        }
    }
}

/// A translator note without its tag, on one line
fn note(entry: &POEntry) -> String {
    let text = entry.comment.as_deref().unwrap_or("");
    text.lines()
        .map(|l| l.trim().trim_start_matches("i18n:").trim())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Text for a Markdown table cell
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

fn generate() -> String {
    let mut by_id: BTreeMap<String, Vec<(String, POEntry)>> = BTreeMap::new();
    for (ctx, entry) in template() {
        let mut parts = ctx.splitn(3, '.');
        let id = format!("{}.{}", parts.next().unwrap(), parts.next().unwrap());
        let field = parts.next().unwrap_or("").to_string();
        by_id.entry(id).or_default().push((field, entry));
    }

    let mut out = String::from(
        "<!-- Generated from locales/messages.pot by scripts/i18n-extract. Do not edit. -->\n\n\
         # Aximar's messages about Maxima's output\n\n\
         Each explanation Aximar gives for something Maxima printed, by its ID. The ID \
         is stable: MCP results carry it as `error_info.id`, and each field's text is a \
         catalog entry whose context is the ID and the field. Parameters fill the \
         placeholders in braces. See [i18n.md](i18n.md).\n",
    );
    for (id, mut entries) in by_id {
        // In reading order: the title, explanations, then suggestions
        let order = |field: &str| {
            ["title", "explanation", "suggestion"]
                .iter()
                .position(|f| {
                    field.split('.').next() == Some(*f) || (*f == "title" && field == "text")
                })
                .unwrap_or(3)
        };
        entries.sort_by(|(a, _), (b, _)| order(a).cmp(&order(b)).then(a.cmp(b)));
        let params: BTreeSet<String> = entries
            .iter()
            .flat_map(|(_, e)| placeholders(&e.msgid))
            .collect();
        writeln!(out, "\n## `{id}`\n").unwrap();
        if let Some((_, title)) = entries
            .iter()
            .find(|(f, _)| f == "title")
            .or_else(|| entries.iter().find(|(f, _)| f == "text"))
        {
            let covers = note(title);
            let covers = covers.strip_prefix("Maxima: ").unwrap_or(&covers);
            writeln!(out, "Covers: {covers}\n").unwrap();
        }
        if !params.is_empty() {
            let list: Vec<_> = params.iter().map(|p| format!("`{p}`")).collect();
            writeln!(out, "Parameters: {}\n", list.join(", ")).unwrap();
        }
        writeln!(out, "| Field | English |\n|---|---|").unwrap();
        for (field, entry) in &entries {
            writeln!(out, "| {field} | {} |", cell(&entry.msgid)).unwrap();
        }
    }
    out
}

#[test]
fn the_list_of_messages_is_current() {
    let path = root().join("docs/maxima-messages.md");
    let generated = generate();
    if std::env::var_os("AXIMAR_UPDATE_DOCS").is_some() {
        std::fs::write(&path, &generated).unwrap();
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        current == generated,
        "docs/maxima-messages.md is out of date: run scripts/i18n-extract"
    );
}

/// Every explanation and note the samples give, as the window receives
/// them: the fixtures for the Storybook gallery (src/stories)
fn fixtures() -> String {
    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());
    let mut explanations = Vec::new();
    for raw in SAMPLES {
        let info = enhance_error_with_packages(raw, &catalog, Some(&packages)).unwrap();
        explanations.push(serde_json::json!({ "error": raw, "error_info": info }));
    }
    for failure in [
        AppError::Timeout(30),
        AppError::EvalCancelled(String::new()),
    ] {
        explanations.push(serde_json::json!({
            "error": failure.to_string(),
            "error_info": explain_failure(&failure).unwrap(),
        }));
    }
    let notes: Vec<_> = NOTE_SAMPLES
        .iter()
        .map(|text| serde_json::json!({ "text_output": text, "notes": notes(text, true) }))
        .collect();
    let mut out = serde_json::to_string_pretty(&serde_json::json!({
        "generated_by": "crates/aximar-core/tests/maxima_messages.rs",
        "explanations": explanations,
        "notes": notes,
    }))
    .unwrap();
    out.push('\n');
    out
}

#[test]
fn the_gallery_fixtures_are_current() {
    let path = root().join("src/stories/messages.generated.json");
    let generated = fixtures();
    if std::env::var_os("AXIMAR_UPDATE_DOCS").is_some() {
        std::fs::write(&path, &generated).unwrap();
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        current == generated,
        "src/stories/messages.generated.json is out of date: run scripts/i18n-extract"
    );
}
