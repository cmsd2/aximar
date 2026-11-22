//! Maxima's `%` after an evaluation, on the kernel-events path.  Needs the
//! kernel-events package installed as well as Maxima.
//!
//! Run with: `cargo test -p aximar-core --test percent_wired -- --ignored`

mod percent_common;

#[tokio::test]
#[ignore = "requires Maxima and the kernel-events package"]
async fn kernel_events_path_keeps_maxima_s_percent() {
    // Read when Maxima starts; this binary runs only this test
    unsafe { std::env::set_var("AXIMAR_KERNEL_EVENTS", "1") };
    percent_common::check_percent().await;
}
