//! The result's one-line form, captured next to its LaTeX, against a real
//! Maxima on the legacy path.  The kernel-events path is in
//! `text_plain_wired.rs`, a separate binary because it needs an
//! environment variable set before Maxima starts.
//!
//! Marked `#[ignore]` so they don't run in CI by default.
//! Run with: `cargo test -p aximar-core --test text_plain -- --ignored`

mod plain_common;

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn legacy_path_captures_the_one_line_form() {
    plain_common::check_plain_forms().await;
}
