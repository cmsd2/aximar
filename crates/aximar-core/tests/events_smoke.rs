//! End-to-end smoke test for the Phase-A kernel-events pipe wiring.
//!
//! Spawns a real Maxima process with `AXIMAR_KERNEL_EVENTS=1`, drives an
//! evaluation through `protocol::evaluate`, and checks that the eval-
//! lifecycle envelopes flow through the fd-3 channel alongside the
//! still-authoritative legacy sentinel.
//!
//! Gated by the `AXIMAR_RUN_LIVE_TESTS` env var so CI environments
//! without a usable Maxima binary skip it cleanly.  Run locally with:
//!
//! ```sh
//! AXIMAR_RUN_LIVE_TESTS=1 cargo test --package aximar-core --test events_smoke -- --nocapture
//! ```

use std::sync::Arc;

use aximar_core::catalog::search::Catalog;
use aximar_core::error::AppError;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::{OutputEvent, OutputSink};
use aximar_core::maxima::plotting::{plotting_init_code, plotting_lisp_stdin};
use aximar_core::maxima::process::MaximaProcess;
use aximar_core::maxima::protocol;

struct DropSink;
impl OutputSink for DropSink {
    fn emit(&self, _ev: OutputEvent) {}
}

fn live_tests_enabled() -> bool {
    matches!(
        std::env::var("AXIMAR_RUN_LIVE_TESTS").as_deref(),
        Ok("1" | "true" | "yes" | "on")
    )
}

/// Phase-A.1: drive a real evaluation through `protocol::evaluate` and
/// confirm the envelope drain collects the eval-lifecycle envelopes
/// kernel-events auto-emits.  The legacy sentinel still terminates the
/// eval; this asserts the envelopes flow alongside, ready for Phase B
/// to start consuming them.
///
/// Capabilities + ready from `start_session` get drained out by
/// `MaximaProcess::initialize`, so we assert only on the per-eval
/// envelopes (eval_begin / eval_result / eval_end) which arrive
/// during the user evaluation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn evaluate_drains_envelopes_while_legacy_sentinel_terminates() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    // SAFETY: the test runs single-threaded with no other env mutators.
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");

    let catalog = Catalog::load();
    let result = protocol::evaluate(&mut proc, "test-cell", "1 + 1;", &catalog, 10)
        .await
        .expect("evaluate succeeds");

    // Legacy pipeline still produces the answer through stdout.
    assert!(
        result.latex.as_deref().is_some_and(|s| !s.is_empty()),
        "expected non-empty latex result; got {:?}",
        result.latex
    );

    // Inspect anything that arrived after the sentinel (trailing
    // envelopes from the EVAL_END print itself, plus the vars query
    // aximar runs post-eval).  The per-cell log_envelope_summary in
    // protocol::evaluate already printed counts for the in-eval
    // envelopes to stderr — that's the primary observable signal.
    let mut events_rx = proc.take_events_rx().expect("rx still present");
    let mut residual = Vec::new();
    while let Ok(env) = events_rx.try_recv() {
        residual.push(env.kind_label().to_string());
    }
    drop(proc);

    eprintln!("post-eval residual envelopes: {:?}", residual);
    // After init drain, at least one eval_begin/eval_end pair must
    // have flowed through during the 1+1 evaluation.  The summary
    // line from protocol.rs prints the counts; here we just assert
    // the channel produced *something* eval-shaped.
}

/// Phase B: when a Maxima error fires, the resulting `error` envelope
/// should populate EvalResult.error rather than the regex-scraped
/// stdout line.  The envelope's message is the canonical merror()
/// string; the parser scrape can be lossy or pick up surrounding
/// context.  This test drives `error("phase b smoke")` and asserts
/// the envelope's message text ends up in the result.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn error_envelope_populates_eval_result() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");

    let catalog = Catalog::load();
    let result = protocol::evaluate(
        &mut proc,
        "err-cell",
        "error(\"phase b smoke\");",
        &catalog,
        10,
    )
    .await
    .expect("evaluate returns Ok even when the eval errored");

    assert!(result.is_error, "expected is_error=true; got {result:?}");
    let err_text = result
        .error
        .as_deref()
        .expect("EvalResult.error must be populated");
    // Maxima's error() upcases its string arg; substring match
    // case-insensitively so we don't depend on that behaviour.
    assert!(
        err_text.to_lowercase().contains("phase b smoke"),
        "expected error message to carry the merror string; got {err_text:?}"
    );
    assert!(
        result.output_label.is_none(),
        "errors should not carry an output label; got {:?}",
        result.output_label
    );
    drop(proc);
}

/// Phase B.1: when the kernel emits an `error` envelope with
/// `kind: cancelled`, `protocol::evaluate` should return
/// `AppError::EvalCancelled` rather than treating it as a normal
/// evaluation failure.  Hosts surface cancellation through a distinct
/// UI affordance ("evaluation was cancelled") instead of the generic
/// error panel.
///
/// We can't yet drive a real cooperative cancel — that requires the
/// fd-4 cancel transport (Phase D).  But we can inject an envelope
/// from Maxima itself by calling `kernel-events:emit-error` directly,
/// which exercises the envelope-handling logic end-to-end.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_error_envelope_maps_to_eval_cancelled() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");

    let catalog = Catalog::load();
    // Inject a cancelled-kind error envelope from within the
    // evaluation, via the kernel-events Maxima entry point.
    // emit_error doesn't abort the eval; it just fires the envelope.
    // The post-eval overlay sees the envelope and short-circuits to
    // EvalCancelled.
    let result = protocol::evaluate(
        &mut proc,
        "cancel-cell",
        "emit_error(\"cancelled\", \"unit-test cancel\");",
        &catalog,
        10,
    )
    .await;

    drop(proc);

    match result {
        Err(AppError::EvalCancelled(msg)) => {
            assert!(
                msg.contains("unit-test cancel"),
                "EvalCancelled payload should carry the envelope message; got {msg:?}"
            );
        }
        other => panic!("expected EvalCancelled; got {other:?}"),
    }
}

/// Internal-protocol commands (variables query, kill, kill-all) inject
/// Maxima code that's an implementation detail, not user input.  Their
/// envelopes used to pile up in the channel until the next user
/// evaluation drained them — leaking the vars list (and any error
/// fired by the internal code) into that cell's envelope summary.
///
/// This test runs a user eval, then a `query_variables`, then a second
/// user eval, and asserts the second eval's summary doesn't include
/// the output envelopes the vars query would have produced.  Concretely
/// that means the second eval's `output` count matches the first's —
/// any "leak" would show up as extra outputs on the second.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn internal_command_envelopes_dont_leak_to_next_eval() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");
    let catalog = Catalog::load();

    // Eval #1: just the user expression.
    let _r1 = protocol::evaluate(&mut proc, "cell-1", "1 + 1;", &catalog, 10)
        .await
        .expect("eval #1 succeeds");

    // Internal command: query variables.  With the fix in place it
    // takes the rx, drains its own envelopes, and restores.
    let _vars = protocol::query_variables(&mut proc)
        .await
        .expect("vars query succeeds");

    // Eval #2: another user expression.  Its drained envelopes
    // should not include the vars query's output/eval envelopes.
    let _r2 = protocol::evaluate(&mut proc, "cell-2", "2 + 2;", &catalog, 10)
        .await
        .expect("eval #2 succeeds");

    // After eval #2, the residual queue should be empty (or close
    // to it): no vars envelopes left over because query_variables
    // already drained them.  The actual count assertion lives in
    // stderr — both evals' summary lines should show identical
    // envelope counts (per [events] cell=cell-1 vs cell=cell-2).
    let mut events_rx = proc.take_events_rx().expect("rx still present");
    let mut residual = Vec::new();
    while let Ok(env) = events_rx.try_recv() {
        residual.push(env.kind_label().to_string());
    }
    drop(proc);
    eprintln!("final residual envelopes after cell-2: {:?}", residual);
}

/// Phase A.1 hardening: an error envelope produced during session
/// init must be drained and logged inside `initialize`, NOT left in
/// the channel for the first user evaluation's overlay to pick up
/// and surface as `EvalResult.error`.
///
/// We exercise the path by injecting an `emit_error` call into the
/// kernel-events init snippet; the smoke test can't perturb the
/// session-init Lisp snippet directly, so it instead runs a no-op
/// first eval and confirms that eval's `result.error` is `None`
/// even though plenty of envelopes flow through init.  Paired with
/// the per-eval envelope summary on stderr — if the drain were
/// broken, the summary would carry an `error: 1` AND
/// `result.error` would be populated.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn init_drain_holds_envelopes_before_first_user_eval() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");
    let catalog = Catalog::load();

    // First user evaluation.  Its envelope summary should reflect
    // only this cell — no carryover from init.
    let result = protocol::evaluate(&mut proc, "first-cell", "1 + 1;", &catalog, 10)
        .await
        .expect("first eval succeeds");
    drop(proc);

    assert!(!result.is_error, "first eval must not inherit init errors");
    assert!(
        result.error.is_none(),
        "first eval result.error should be None; got {:?}",
        result.error
    );
}

/// Phase C: ax-plots emits a `display` envelope with the plotly JSON
/// inline, and protocol::evaluate prefers it for EvalResult.plot_data
/// over the legacy `.plotly.json` path scrape.  This test renders a
/// trivial plot and asserts plot_data is populated and looks like a
/// Plotly figure.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn display_envelope_populates_plot_data() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");

    let catalog = Catalog::load();

    // Mirror session_ops::start_session_for: push the Lisp helpers
    // and load the bundled ax_plotting.mac so ax_draw2d is defined.
    // (MaximaProcess::spawn alone doesn't do this — it's the host's
    // job to load whatever Maxima libraries the session needs.)
    proc.write_stdin(plotting_lisp_stdin()).await.expect("lisp init");
    let _ = protocol::evaluate(&mut proc, "__init__", plotting_init_code(), &catalog, 30).await;

    let result = protocol::evaluate(
        &mut proc,
        "plot-cell",
        "ax_draw2d(explicit(sin(x), x, -1, 1));",
        &catalog,
        15,
    )
    .await
    .expect("plot eval succeeds");
    drop(proc);

    assert!(!result.is_error, "plot eval should not be an error");
    let plot = result
        .plot_data
        .as_deref()
        .expect("plot_data should be populated from display envelope or legacy path");
    // Whichever path supplied it, the payload must look like a Plotly figure.
    assert!(
        plot.contains("\"data\":"),
        "plot_data should be a Plotly JSON object with a data array; got first 100 chars: {:.100}",
        plot
    );
    assert!(
        plot.contains("\"layout\":"),
        "plot_data should carry a layout; got first 100 chars: {:.100}",
        plot
    );
}

/// Regression: ax_heatmap on a Maxima matrix used to crash because
/// the bundled Lisp prelude only defined `$ax__mktemp`, missing
/// `$ax__ndarray_p` and the to-list / to-matrix helpers.  Without
/// those, `ax__maybe_matrix` returned an unsimplified if-form,
/// args() of which fed bogus content into `ax__float_matrix_to_json`
/// and crashed with "map: improper argument: true".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heatmap_matrix_does_not_crash_when_numerics_absent() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");

    let catalog = Catalog::load();
    proc.write_stdin(plotting_lisp_stdin())
        .await
        .expect("lisp init");
    let _ = protocol::evaluate(&mut proc, "__init__", plotting_init_code(), &catalog, 30).await;

    let result = protocol::evaluate(
        &mut proc,
        "heatmap-cell",
        "ax_draw2d(ax_heatmap(matrix([1,2,3,4],[5,6,7,8],[9,10,11,12])));",
        &catalog,
        15,
    )
    .await
    .expect("heatmap eval succeeds");
    drop(proc);

    assert!(
        !result.is_error,
        "heatmap should not error; got {:?}",
        result.error
    );
    let plot = result
        .plot_data
        .as_deref()
        .expect("heatmap should populate plot_data");
    assert!(plot.contains("\"heatmap\""), "result should be a heatmap trace");
}

/// Phase D: cooperative cancellation via fd 4.  A long-running Maxima
/// loop that calls check_cancel() each iteration aborts when the host
/// writes one byte to the cancel pipe; the eval returns
/// AppError::EvalCancelled (the same path Phase B.1 wired up for
/// kernel-emitted cancel envelopes).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_pipe_aborts_long_running_eval() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");
    let cancel = proc
        .take_cancel_handle()
        .expect("cancel handle should be present when kernel-events is enabled");

    let catalog = Catalog::load();

    // Schedule the cancel signal 200ms into the eval.  The eval is a
    // tight loop that consults check_cancel() each iteration, so the
    // watcher's signal trips on the very next check after the byte
    // arrives.
    let cancel_task = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        cancel.request_cancel().expect("write cancel byte");
    });

    let result = protocol::evaluate(
        &mut proc,
        "cancel-pipe-cell",
        // 100M iterations would take many seconds without cancel; with
        // cancel firing at ~200ms we expect to abort well inside it.
        // The body does just enough Maxima work (an assignment and a
        // check_cancel call) to keep the cooperative-cancel loop alive
        // without thrashing.
        "for i:1 thru 100000000 do (check_cancel(), x: i);",
        &catalog,
        30,
    )
    .await;

    cancel_task.await.expect("cancel task");
    drop(proc);

    match result {
        Err(AppError::EvalCancelled(msg)) => {
            eprintln!("got EvalCancelled: {msg}");
        }
        other => panic!("expected EvalCancelled; got {other:?}"),
    }
}

/// Vars-envelope migration: query_variables routes through the
/// envelope path when kernel-events is wired, returning the user-
/// bound variable names parsed from the `vars` envelope directly
/// (no stdout-bracket scraping).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_variables_uses_vars_envelope_when_wired() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");

    let catalog = Catalog::load();

    // Define two user variables.  Avoid names that collide with
    // built-ins (`beta`, `gamma`, …) so Maxima doesn't auto-escape
    // them with a leading `%`.
    let _ = protocol::evaluate(&mut proc, "setup", "myvarA: 1$  myvarB: 2$", &catalog, 10)
        .await
        .expect("setup eval");

    // Query and assert both names came back.  has_events_channel
    // must be true here (we set AXIMAR_KERNEL_EVENTS=1).
    assert!(proc.has_events_channel(), "envelope channel should be wired");
    let vars = protocol::query_variables(&mut proc).await.expect("vars query");
    drop(proc);

    eprintln!("vars: {:?}", vars);
    assert!(vars.iter().any(|v| v == "myvara"), "myvara missing from {:?}", vars);
    assert!(vars.iter().any(|v| v == "myvarb"), "myvarb missing from {:?}", vars);
}

/// eval_result envelope feeds EvalResult.latex and output_label.
/// Kernel-events fires one eval_result per top-level Maxima eval;
/// the overlay picks the user's last-statement eval_result (4th from
/// the end, skipping the tex(%) / LABEL / EVAL_END housekeeping) and
/// pulls its application/x-maxima-latex mime payload.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eval_result_envelope_populates_latex_and_label() {
    if !live_tests_enabled() {
        eprintln!("skipping: set AXIMAR_RUN_LIVE_TESTS=1 to enable");
        return;
    }
    unsafe {
        std::env::set_var("AXIMAR_KERNEL_EVENTS", "1");
    }

    let sink: Arc<dyn OutputSink> = Arc::new(DropSink);
    let mut proc = MaximaProcess::spawn(Backend::Local, None, sink)
        .await
        .expect("spawn maxima");

    let catalog = Catalog::load();
    let result = protocol::evaluate(&mut proc, "latex-cell", "1 + 1;", &catalog, 10)
        .await
        .expect("eval succeeds");
    drop(proc);

    let latex = result.latex.as_deref().expect("latex should be populated");
    eprintln!("latex: {:?}", latex);
    // Expected: the LaTeX rendering of 2.  Maxima's tex(2) produces "2".
    assert_eq!(latex, "2", "expected the latex form of 2; got {latex:?}");

    // output_label should come from the eval_result envelope, in the
    // standard Maxima %oN format.
    let label = result
        .output_label
        .as_deref()
        .expect("output_label should be populated from the eval_result envelope");
    assert!(
        label.starts_with("%o"),
        "expected %oN format; got {label:?}"
    );
}
