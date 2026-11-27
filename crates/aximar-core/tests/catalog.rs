//! The translation catalogs in `locales/`: every translation keeps its
//! entry's placeholders and fits the length limit its entry declares.
//! CI's i18n job checks the rest: that the template is up to date
//! (`scripts/i18n-extract --check`) and that each catalog is valid
//! (`msgfmt -c`).  See docs/i18n.md.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use rspolib::{pofile, POEntry, POFile};

/// `{name}`, or the argument of an ICU plural or select
static PLACEHOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\s*([A-Za-z0-9_]+)\s*[,}]").unwrap());

/// A length limit for translations, in a translator note or a flag, as
/// Weblate writes it
static MAX_LENGTH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bmax-length:(\d+)\b").unwrap());

fn placeholders(text: &str) -> BTreeSet<&str> {
    PLACEHOLDER
        .captures_iter(text)
        .map(|c| c.get(1).unwrap().as_str())
        .collect()
}

fn max_length(entry: &POEntry) -> Option<usize> {
    let note = entry.comment.as_deref().unwrap_or("");
    std::iter::once(note)
        .chain(entry.flags.iter().map(String::as_str))
        .find_map(|s| MAX_LENGTH.captures(s))
        .map(|c| c[1].parse().unwrap())
}

fn name(entry: &POEntry) -> String {
    match &entry.msgctxt {
        Some(ctx) => format!("{ctx} {:?}", entry.msgid),
        None => format!("{:?}", entry.msgid),
    }
}

/// What's wrong with a catalog's translations, one line each
fn problems(po: &POFile) -> Vec<String> {
    let mut out = Vec::new();
    for entry in &po.entries {
        // Rust writes a brace as `{{`, the window's ICU messages as `'{'`;
        // and in ICU an apostrophe before a brace quotes it, hiding the value
        if entry.flags.iter().any(|f| f == "rust-format") {
            let id = &entry.msgid;
            if id.contains("{{") || id.contains("}}") {
                out.push(format!(
                    "{}: literal braces can't be translated the same way in Rust and the window",
                    name(entry)
                ));
            }
            if id.contains("'{") || id.contains("'}") {
                out.push(format!(
                    "{}: an apostrophe before a brace hides the placeholder in the window; use double quotes",
                    name(entry)
                ));
            }
        }
        if entry.obsolete || entry.fuzzy() {
            continue;
        }
        let plural = entry.msgid_plural.is_some();
        let source: BTreeSet<&str> = placeholders(&entry.msgid)
            .into_iter()
            .chain(placeholders(entry.msgid_plural.as_deref().unwrap_or("")))
            .collect();
        let translations = entry
            .msgstr
            .iter()
            .chain(entry.msgstr_plural.iter())
            .filter(|t| !t.is_empty());
        for translation in translations {
            let found = placeholders(translation);
            let unknown: Vec<_> = found.difference(&source).collect();
            if !unknown.is_empty() {
                out.push(format!(
                    "{}: translation {translation:?} has placeholders the source doesn't: {unknown:?}",
                    name(entry)
                ));
            }
            // A plural form may leave out the count ("one file")
            let missing: Vec<_> = source.difference(&found).collect();
            if !plural && !missing.is_empty() {
                out.push(format!(
                    "{}: translation {translation:?} drops placeholders: {missing:?}",
                    name(entry)
                ));
            }
            if let Some(limit) = max_length(entry) {
                let len = translation.chars().count();
                if len > limit {
                    out.push(format!(
                        "{}: translation {translation:?} is {len} characters, over its limit of {limit}",
                        name(entry)
                    ));
                }
            }
        }
    }
    out
}

fn parse(po: &str) -> POFile {
    pofile(po).expect("a valid catalog")
}

const HEADER: &str = r#"msgid ""
msgstr ""
"Language: es\n"
"Content-Type: text/plain; charset=UTF-8\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\n"

"#;

#[test]
fn every_catalog_keeps_its_placeholders_and_limits() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../locales");
    let mut found = 0;
    let mut all = Vec::new();
    for file in std::fs::read_dir(&dir).expect("locales/") {
        let path = file.unwrap().path();
        if path.extension().is_some_and(|e| e == "po") {
            found += 1;
            let po = pofile(path.as_path()).expect("a valid catalog");
            // rspolib reads an empty catalog from anything it can't open
            assert!(!po.entries.is_empty(), "{} has no entries", path.display());
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            all.extend(problems(&po).into_iter().map(|p| format!("{name}: {p}")));
        }
    }
    assert!(found > 0, "no catalogs in {}", dir.display());
    assert!(all.is_empty(), "{}", all.join("\n"));
}

#[test]
fn a_renamed_placeholder_fails() {
    let po = parse(&format!(
        "{HEADER}msgid \"{{fn}} expects {{n}} arguments\"\nmsgstr \"{{fn}} espera {{m}} argumentos\"\n"
    ));
    let problems = problems(&po);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].contains("\"{fn} expects {n} arguments\""));
    assert!(problems[0].contains("\"m\""), "{}", problems[0]);
    assert!(
        problems[1].contains("drops placeholders: [\"n\"]"),
        "{}",
        problems[1]
    );
}

#[test]
fn a_kept_placeholder_passes_in_any_order() {
    let po = parse(&format!(
        "{HEADER}msgctxt \"error.arg_count\"\nmsgid \"{{fn}} expects {{n}} arguments\"\nmsgstr \"{{n}} argumentos para {{fn}}\"\n"
    ));
    assert_eq!(problems(&po), Vec::<String>::new());
}

#[test]
fn untranslated_and_fuzzy_entries_are_left_alone() {
    let po = parse(&format!(
        "{HEADER}msgid \"{{a}} one\"\nmsgstr \"\"\n\n#, fuzzy\nmsgid \"{{a}} two\"\nmsgstr \"{{b}} dos\"\n"
    ));
    assert_eq!(problems(&po), Vec::<String>::new());
}

#[test]
fn a_plural_form_may_leave_out_the_count() {
    let po = parse(&format!(
        "{HEADER}msgid \"one file\"\nmsgid_plural \"{{count}} files\"\nmsgstr[0] \"un archivo\"\nmsgstr[1] \"{{count}} archivos\"\n"
    ));
    assert_eq!(problems(&po), Vec::<String>::new());
}

#[test]
fn a_translation_over_its_limit_fails() {
    let po = parse(&format!(
        "{HEADER}#. A menu item. max-length:8\nmsgid \"Run all\"\nmsgstr \"Ejecutar todo\"\n\n#, max-length:20\nmsgid \"Stop\"\nmsgstr \"Detener\"\n"
    ));
    let problems = problems(&po);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("13 characters, over its limit of 8"));
}

#[test]
fn literal_braces_in_rust_text_fail() {
    let po = parse(&format!(
        "{HEADER}#, rust-format\nmsgid \"a set {{{{x}}}}\"\nmsgstr \"\"\n"
    ));
    assert_eq!(problems(&po).len(), 1);
}

#[test]
fn an_apostrophe_before_a_placeholder_fails() {
    let po = parse(&format!(
        "{HEADER}#, rust-format\nmsgid \"'{{func}}' is unknown\"\nmsgstr \"\"\n"
    ));
    let problems = problems(&po);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("apostrophe"));
}
