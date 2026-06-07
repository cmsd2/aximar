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

use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::{OutputEvent, OutputSink};
use aximar_core::maxima::process::MaximaProcess;

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
        .take_events_receiver()
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
