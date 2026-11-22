//! Maxima's `%` after an evaluation, on the legacy path.  The
//! kernel-events path is in `percent_wired.rs`, a separate binary because
//! it needs an environment variable set before Maxima starts.
//!
//! Run with: `cargo test -p aximar-core --test percent -- --ignored`

mod percent_common;

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn legacy_path_keeps_maxima_s_percent() {
    percent_common::check_percent().await;
}
