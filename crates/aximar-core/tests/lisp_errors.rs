//! Error text on the legacy path: Lisp errors whole, without the debugger
//! hint.  The kernel-events path is in `lisp_errors_wired.rs`.
//!
//! Run with: `cargo test -p aximar-core --test lisp_errors -- --ignored`

mod lisp_common;

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn legacy_path_error_text() {
    lisp_common::check_error_text().await;
}
