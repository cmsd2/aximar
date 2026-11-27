//! A cell that times out keeps an error output saying so, on the legacy
//! path.  Cancelling needs kernel-events, so it's in `failures_wired.rs`.
//!
//! Run with: `cargo test -p aximar-core --test failures -- --ignored`

mod failures_common;

use aximar_core::error::AppError;
use failures_common::Session;

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn a_timeout_leaves_an_error_output() {
    let s = Session::start().await;
    let (result, output) = s.run("for i:1 thru 10^9 do x: i$", 2).await;
    assert!(matches!(result, Err(AppError::Timeout(2))), "{result:?}");
    let output = output.expect("an output");
    assert!(output.is_error, "{output:?}");
    let info = output.error_info.expect("an explanation");
    assert_eq!(info.id, "error.timeout");
    assert_eq!(info.explanation.params["seconds"], "2");

    // The session carries on
    let (result, output) = s.run("2+2;", 30).await;
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(output.unwrap().text_plain.as_deref(), Some("4"));
}
