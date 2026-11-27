//! A notebook run and saved with the language set to Spanish is the same
//! file as one run in English, apart from cell IDs and timings: Aximar
//! never writes a translation into a file.
//!
//! Run with: `cargo test -p aximar-core --test same_file_any_language -- --ignored`

use std::sync::Arc;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{NoEffects, NotebookCommand};
use aximar_core::evaluation::evaluate_cell;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::notebook::CellType;
use aximar_core::notebooks::format;
use aximar_core::registry::NotebookRegistry;
use aximar_core::session_ops::{ensure_session, SessionSettings};

/// Cells with results, print text, an error Maxima translates, one it
/// doesn't, and a note
const CELLS: &[&str] = &[
    "print(\"hello\")$ x^2/2;",
    "1/0;",
    "diff();",
    "error(\"my own message\");",
    "rat(0.5*x);",
    "integrate(sin(x), x);",
];

/// Run the cells in a fresh session in `language` and return the saved
/// file, and how many cells held a translated message in memory
async fn saved_file(language: Option<&str>) -> (serde_json::Value, usize) {
    let mut registry = NotebookRegistry::new();
    registry.set_language(language.map(str::to_string));
    let id = registry.create();
    let ctx = registry.get(&id).unwrap();
    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());
    let settings = SessionSettings {
        backend: Backend::Local,
        maxima_path: None,
        eval_timeout: 30,
    };
    ensure_session(
        &ctx,
        &settings,
        |ctx| ctx.capture_sink.clone() as Arc<dyn OutputSink>,
        |_| None,
        &catalog,
        None,
    )
    .await
    .expect("Maxima starts");

    let mut after = None;
    let mut translated = 0;
    for input in CELLS {
        let cell_id = {
            let mut nb = ctx.notebook.lock().await;
            let effect = nb
                .apply(NotebookCommand::AddCell {
                    cell_type: CellType::Code,
                    input: input.to_string(),
                    after_cell_id: after.clone(),
                    before_cell_id: None,
                })
                .unwrap();
            effect.cell_id().unwrap().to_string()
        };
        let out = evaluate_cell(&ctx, &cell_id, &catalog, &packages, 30, &NoEffects)
            .await
            .unwrap()
            .cell_output;
        translated += usize::from(out.error_translated.is_some());
        after = Some(cell_id);
    }
    let nb = ctx.notebook.lock().await;
    let mut file: serde_json::Value = serde_json::from_str(&format::write(&nb)).unwrap();
    normalise(&mut file);
    (file, translated)
}

/// Drop what differs between any two runs: IDs and timings
fn normalise(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|k, _| !matches!(k.as_str(), "id" | "duration_ms" | "timing" | "execution"));
            map.values_mut().for_each(normalise);
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(normalise),
        _ => {}
    }
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn a_notebook_saves_the_same_in_any_language() {
    let (english, none) = saved_file(None).await;
    let (spanish, translated) = saved_file(Some("es")).await;
    assert_eq!(none, 0);
    // 1/0 and diff(): the window showed Maxima's Spanish
    assert_eq!(translated, 2);
    assert_eq!(
        english, spanish,
        "English:\n{english:#}\nSpanish:\n{spanish:#}"
    );
    let text = english.to_string();
    // The run reached the cells that fail, and wrote nothing in Spanish
    assert!(text.contains("0 to a negative exponent"), "{text}");
    assert!(!text.contains("no definido"), "{text}");
}
