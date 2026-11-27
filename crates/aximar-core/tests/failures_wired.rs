//! A cancelled cell keeps an error output saying so.  Cancelling needs
//! the kernel-events path; this injects the cancel envelope kernel-events
//! sends, as `events_smoke.rs` does.
//!
//! Run with: `cargo test -p aximar-core --test failures_wired -- --ignored`

mod failures_common;

use aximar_core::error::AppError;
use failures_common::Session;

#[tokio::test]
#[ignore = "requires Maxima and the kernel-events package"]
async fn a_cancel_leaves_an_error_output() {
    // Read when Maxima starts; this binary runs only this test
    unsafe { std::env::set_var("AXIMAR_KERNEL_EVENTS", "1") };
    let s = Session::start().await;
    let (result, output) = s
        .run("emit_error(\"cancelled\", \"stopped by the test\");", 30)
        .await;
    assert!(
        matches!(result, Err(AppError::EvalCancelled(_))),
        "{result:?}"
    );
    let output = output.expect("an output");
    assert!(output.is_error, "{output:?}");
    assert_eq!(
        output.error_info.expect("an explanation").id,
        "error.cancelled"
    );
}
