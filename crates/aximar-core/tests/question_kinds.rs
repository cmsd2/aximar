//! Each kind of question Maxima really asks, against a real Maxima on the
//! legacy path: Aximar names it, and the line it suggests answers it.
//!
//! Run with: `cargo test -p aximar-core --test question_kinds -- --ignored`

mod failures_common;

use aximar_core::notebook::CellOutput;
use failures_common::Session;

/// Run a cell and return its output
async fn run(s: &Session, input: &str) -> CellOutput {
    let (_, output) = s.run(input, 30).await;
    output.unwrap_or_else(|| panic!("{input} left no output"))
}

/// A cell that asks a question, the question's ID, and whether answering
/// it leads Maxima to ask something else next
const CASES: &[(&str, &str, bool)] = &[
    ("integrate(exp(-a*x), x, 0, inf);", "question.sign", false),
    ("integrate(1/(x^2+k), x);", "question.sign_pn", false),
    ("integrate(x^n, x);", "question.equal", false),
    ("askinteger(n);", "question.property", false),
    ("askinteger(n, even);", "question.property", false),
    ("askinteger(n, odd);", "question.property", false),
    ("integrate(1/(x-c), x, 0, 1);", "question.sign", false),
    // Declared complex: Maxima asks about 'realpart(x)
    (
        "declare(x, complex)$ integrate(exp(-x*t), t, 0, inf);",
        "question.sign",
        false,
    ),
    // Nonzero, then Maxima asks for the sign
    (
        "integrate(1/(x^2+a^2), x, minf, inf);",
        "question.zero",
        true,
    ),
    // One exponent's sign, then the other's
    ("integrate(x^m*(1-x)^k, x, 0, 1);", "question.sign", true),
];

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn every_kind_of_question_is_named_and_answered() {
    let s = Session::start().await;
    let mut failures = Vec::new();
    for (input, id, asks_again) in CASES {
        run(&s, "kill(all)$").await;
        let out = run(&s, input).await;
        let info = match out.error_info.as_ref() {
            Some(info) if out.is_error && info.id == *id => info.clone(),
            _ => {
                failures.push(format!("{input}: expected {id}, got {out:?}"));
                continue;
            }
        };
        let Some(line) = info.example.clone() else {
            failures.push(format!("{input}: no suggested line"));
            continue;
        };

        // The suggested line first, then the same cell
        run(&s, "kill(all)$").await;
        let answered = run(&s, &format!("{line}\n{input}")).await;
        let same_question = answered.is_error && answered.error.as_deref() == out.error.as_deref();
        if same_question {
            failures.push(format!("{input}: {line} doesn't answer it"));
        } else if answered.is_error && !asks_again {
            failures.push(format!("{input}: after {line}, {:?}", answered.error));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
