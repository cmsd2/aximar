//! Maxima's questions on the kernel-events path.  Needs the kernel-events
//! package installed as well as Maxima.
//!
//! Run with: `cargo test -p aximar-core --test questions_wired -- --ignored`

mod questions_common;

#[tokio::test]
#[ignore = "requires Maxima and the kernel-events package"]
async fn kernel_events_path_turns_questions_into_errors() {
    // Read when Maxima starts; this binary runs only this test
    unsafe { std::env::set_var("AXIMAR_KERNEL_EVENTS", "1") };
    questions_common::check_questions().await;
}
