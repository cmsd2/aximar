//! `msg!` and `msg_ctx!`.  Here rather than in `src/`, so the test
//! strings stay out of the catalog: extraction reads only `src/`.

use aximar_core::message::Message;
use aximar_core::{msg, msg_ctx};

#[test]
fn text_is_the_template_with_its_parameters() {
    let name = String::from("f");
    let m = msg!(
        "{func} expects {n} arguments, found {found}",
        func = name,
        n = 2,
        found = 3
    );
    assert_eq!(m.id, "");
    assert_eq!(m.template, "{func} expects {n} arguments, found {found}");
    assert_eq!(m.text, "f expects 2 arguments, found 3");
    assert_eq!(m.params.len(), 3);
    assert_eq!(m.params["func"], "f");
    assert_eq!(m.to_string(), m.text);
}

#[test]
fn context_is_the_id() {
    let m = msg_ctx!("error.division_by_zero.title", "Division by zero");
    assert_eq!(m.id, "error.division_by_zero.title");
    assert_eq!(m.text, "Division by zero");
    assert!(m.params.is_empty());
}

#[test]
fn values_are_taken_before_any_parameter_is_bound() {
    let (a, b) = ("1", "2");
    let m = msg!("{a} {b}", a = b, b = a);
    assert_eq!(m.text, "2 1");
}

#[test]
fn round_trips_through_json() {
    let m = msg_ctx!(
        "question.sign",
        "Maxima needs to know the sign of {expr}",
        expr = "a"
    );
    let back: Message = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_eq!(back, m);
}
