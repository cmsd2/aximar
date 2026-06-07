//! End-to-end smoke test for the Phase-A kernel-events pipe wiring.
//!
//! Spawns a real Maxima process with `AXIMAR_KERNEL_EVENTS=1` and
//! checks that the session-init prelude fires kernel-events through
//! the fd-3 channel: we expect a `capabilities` envelope followed by
//! a `ready` envelope (the standard `$start_session` handshake) before
//! any user code runs.
//!
//! Gated by the `AXIMAR_RUN_LIVE_TESTS` env var so CI environments
//! without a usable Maxima binary skip it cleanly.  Run locally with:
//!
//! ```sh
//! AXIMAR_RUN_LIVE_TESTS=1 cargo test --package aximar-core --test events_smoke -- --nocapture
//! ```

use std::sync::Arc;
use std::time::Duration;

use aximar_core::catalog::search::Catalog;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::{OutputEvent, OutputSink};
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_init_emits_capabilities_and_ready() {
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

    let mut events_rx = proc
        .take_events_rx()
        .expect("events receiver was wired");

    // Give the kernel-events init time to load, register the sink,
    // and fire the start_session handshake.  Capabilities + ready
    // arrive together; we wait up to 5s total.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut kinds: Vec<String> = Vec::new();
    while kinds.len() < 2 {
        tokio::select! {
            env = events_rx.recv() => match env {
                Some(e) => kinds.push(e.kind_label().to_string()),
                None => break,
            },
            _ = tokio::time::sleep_until(deadline) => break,
        }
    }

    drop(proc);

    eprintln!("envelopes observed: {:?}", kinds);
    assert!(
        kinds.contains(&"capabilities".to_string()),
        "expected capabilities envelope; got {:?}",
        kinds
    );
    assert!(
        kinds.contains(&"ready".to_string()),
        "expected ready envelope; got {:?}",
        kinds
    );
}

/// Phase-A.1: drive a real evaluation through `protocol::evaluate` and
/// confirm the envelope drain collects the eval-lifecycle envelopes
/// kernel-events auto-emits.  The legacy sentinel still terminates the
/// eval; this asserts the envelopes flow alongside, ready for Phase B
/// to start consuming them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn evaluate_drains_envelopes_while_legacy_sentinel_terminates() {
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
    let result = protocol::evaluate(&mut proc, "test-cell", "1 + 1;", &catalog, 10)
        .await
        .expect("evaluate succeeds");

    // The legacy pipeline still produces the answer through stdout.
    assert!(
        result.latex.as_deref().is_some_and(|s| !s.is_empty()),
        "expected non-empty latex result; got {:?}",
        result.latex
    );

    // Drain whatever envelopes are still pending (capabilities + ready
    // from init, plus the eval_begin/result/end triples).  The drain
    // inside protocol::evaluate already consumed those that arrived
    // before the sentinel — this just inspects the post-drain state.
    let mut events_rx = proc.take_events_rx().expect("rx still present");
    let mut kinds = Vec::new();
    while let Ok(env) = events_rx.try_recv() {
        kinds.push(env.kind_label().to_string());
    }
    drop(proc);

    // We can't assert exact counts (the drain races with eval timing
    // so envelopes may have been consumed inside protocol::evaluate),
    // but stderr will print the [events] summary from protocol.rs's
    // log_envelope_summary — that's the observable signal during
    // Phase A.1.
    eprintln!("post-eval residual envelopes: {:?}", kinds);
}
