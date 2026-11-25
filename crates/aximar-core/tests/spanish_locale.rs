//! Aximar under a Spanish locale.  Maxima translates its error messages,
//! and Aximar reads them to tell when a cell failed, so Aximar starts
//! Maxima in English whatever the user's locale.  Without that, errors
//! passed as results and the debugger hung on a syntax error.
//!
//! A separate binary because it sets the locale before Maxima starts.
//!
//! Run with: `cargo test -p aximar-core --test spanish_locale -- --ignored`

mod percent_common;

use std::sync::Arc;
use std::time::Duration;

use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::debugger::PromptKind;
use aximar_core::maxima::output::{OutputEvent, OutputSink};
use aximar_core::maxima::process::MaximaProcess;

struct NullSink;
impl OutputSink for NullSink {
    fn emit(&self, _event: OutputEvent) {}
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn errors_are_still_errors_under_a_spanish_locale() {
    // Every variable Maxima reads its language from.  One test, so no
    // other thread reads the environment while it changes.
    for var in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        unsafe { std::env::set_var(var, "es_ES.UTF-8") };
    }

    // A failed cell is an error, and `%` keeps the result before it
    percent_common::check_percent().await;

    // In debug mode (the debugger's), a syntax error returns rather than
    // waiting for the end marker
    let mut proc = MaximaProcess::spawn(Backend::Local, None, Arc::new(NullSink))
        .await
        .expect("Maxima starts");
    proc.set_debug_mode(true);
    proc.write_stdin("debugmode(true)$\nprint(\"__READY__\")$\n")
        .await
        .unwrap();
    proc.read_until_sentinel("__READY__").await.unwrap();
    proc.write_stdin("block([r], r: (1 +), print(\"__DONE__\"), r)$\n")
        .await
        .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        proc.read_dap_response(Some("__DONE__")),
    )
    .await
    .expect("a syntax error doesn't hang the debugger");
    match result {
        Err(e) => assert!(e.to_string().contains("incorrect syntax"), "{e}"),
        Ok((_, prompt)) => assert!(
            matches!(prompt, PromptKind::Debugger { .. }),
            "expected an error or the debugger's prompt, got {prompt:?}"
        ),
    }
}
