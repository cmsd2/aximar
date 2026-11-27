//! Error text on the kernel-events path.  Needs the kernel-events package
//! installed as well as Maxima.
//!
//! Run with: `cargo test -p aximar-core --test lisp_errors_wired -- --ignored`

mod lisp_common;

#[tokio::test]
#[ignore = "requires Maxima and the kernel-events package"]
async fn kernel_events_path_error_text() {
    // Read when Maxima starts; this binary runs only this test
    unsafe { std::env::set_var("AXIMAR_KERNEL_EVENTS", "1") };
    lisp_common::check_error_text().await;
}
