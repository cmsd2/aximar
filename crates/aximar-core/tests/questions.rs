//! Maxima's questions on the legacy path: each stops its cell with an
//! error explaining how to answer it.  The kernel-events path is in
//! `questions_wired.rs`, a separate binary because it needs an
//! environment variable set before Maxima starts.
//!
//! Run with: `cargo test -p aximar-core --test questions -- --ignored`

mod questions_common;

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn legacy_path_turns_questions_into_errors() {
    questions_common::check_questions().await;
}
