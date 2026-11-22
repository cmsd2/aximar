//! The result's one-line form on the kernel-events path.  Needs the
//! kernel-events package installed as well as Maxima.
//!
//! Run with: `cargo test -p aximar-core --test text_plain_wired -- --ignored`

mod plain_common;

#[tokio::test]
#[ignore = "requires Maxima and the kernel-events package"]
async fn kernel_events_path_captures_the_one_line_form() {
    // Read when Maxima starts; this binary runs only this test
    unsafe { std::env::set_var("AXIMAR_KERNEL_EVENTS", "1") };
    plain_common::check_plain_forms().await;
}
