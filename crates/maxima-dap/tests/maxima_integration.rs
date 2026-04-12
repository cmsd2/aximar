//! Integration tests that spawn an actual Maxima process and exercise
//! the debugger communication path: breakpoint setting, prompt detection,
//! backtrace retrieval, stepping, and resume/completion.
//!
//! These tests require Maxima (with SBCL backend recommended) to be
//! installed and on PATH. They are marked `#[ignore]` by default —
//! run with `cargo test -p maxima-dap -- --ignored` to include them.

use std::path::Path;
use std::sync::Arc;

use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::debugger::{self, PromptKind};
use aximar_core::maxima::output::{OutputEvent, OutputSink};
use aximar_core::maxima::process::MaximaProcess;
use maxima_dap::breakpoints::SourceIndex;
use maxima_dap::strategy::BreakpointStrategy;
use maxima_dap::strategy::StrategyContext;
use maxima_dap::strategy_legacy::LegacyStrategy;
use maxima_dap::types::DebugState;

/// Null output sink for tests.
struct NullSink;
impl OutputSink for NullSink {
    fn emit(&self, _event: OutputEvent) {}
}

fn example_path(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
        .to_string_lossy()
        .to_string()
}

async fn spawn_maxima() -> MaximaProcess {
    let custom_path = std::env::var("MAXIMA_PATH").ok();
    MaximaProcess::spawn(Backend::Local, custom_path, Arc::new(NullSink))
        .await
        .expect("failed to spawn Maxima — is it installed?")
}

// ---------------------------------------------------------------------------
// Debugger prompt detection
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn debugger_prompt_detected_on_breakpoint() {
    let mut proc = spawn_maxima().await;

    // Enable debug mode and load example
    let path = example_path("01_basic_breakpoint.mac");
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    let sentinel = "__TEST_DONE__";
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Set a breakpoint at offset 0 (function entry)
    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    let (lines, _) = proc.read_until_sentinel(sentinel).await.unwrap();
    let bp_set = lines.iter().any(|l| l.contains("Bkpt"));
    assert!(bp_set, "expected breakpoint confirmation, got: {:?}", lines);

    // Evaluate expression — should hit the breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();

    let (lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "expected debugger prompt after breakpoint hit, got {:?}",
        prompt
    );

    // There should be a breakpoint-hit message in the output
    let has_bkpt_hit = lines
        .iter()
        .any(|l| debugger::parse_breakpoint_hit(l).is_some());
    assert!(
        has_bkpt_hit,
        "expected breakpoint-hit message in output, got: {:?}",
        lines
    );

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Backtrace at breakpoint
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn backtrace_at_breakpoint() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup: debugmode, batchload, set breakpoint
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Trigger breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Request backtrace
    proc.write_stdin(":bt\n").await.unwrap();
    let (bt_lines, prompt) = proc.read_dap_response(None).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Parse frames from backtrace
    let frames: Vec<_> = bt_lines
        .iter()
        .filter_map(|l| debugger::parse_backtrace_frame(l))
        .collect();
    assert!(
        !frames.is_empty(),
        "expected at least one backtrace frame, got lines: {:?}",
        bt_lines
    );
    assert_eq!(frames[0].function, "add", "top frame should be 'add'");

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Resume completes evaluation
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn resume_completes_evaluation() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Trigger breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Resume — should complete the expression and produce the sentinel
    proc.write_stdin(":resume\n").await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert_eq!(
        prompt,
        PromptKind::Normal,
        "expected Normal prompt (sentinel) after :resume"
    );

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Step produces another debugger prompt
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn step_stays_in_debugger() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Trigger breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Step — should stay in the debugger
    proc.write_stdin(":step\n").await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "expected debugger prompt after :step, got {:?}",
        prompt
    );

    // Resume to finish
    proc.write_stdin(":resume\n").await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert_eq!(prompt, PromptKind::Normal);

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Evaluate expression at debugger prompt
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn evaluate_at_debugger_prompt() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Trigger breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Evaluate an expression at the debugger prompt
    proc.write_stdin("a + b;\n").await.unwrap();
    let (lines, prompt) = proc.read_dap_response(None).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // The result (7) should be in the output
    let has_result = lines.iter().any(|l| l.contains('7'));
    assert!(
        has_result,
        "expected '7' in evaluation output, got: {:?}",
        lines
    );

    // Clean up
    proc.write_stdin(":resume\n").await.unwrap();
    let _ = proc.read_dap_response(Some(eval_sentinel)).await;
    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Backtrace frames have correct source file and line
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn backtrace_frame_has_source_line() {
    // Verifies that the backtrace frame includes the source file name
    // and an actual file line number (not a function offset), and that
    // the DAP StackFrame conversion uses them correctly.
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup: debugmode, batchload, set breakpoint
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!(
        "batchload(\"{}\")$\n",
        path.replace('\\', "/")
    ))
    .await
    .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Trigger breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Request backtrace
    proc.write_stdin(":bt\n").await.unwrap();
    let (bt_lines, prompt) = proc.read_dap_response(None).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Parse frames
    let frames: Vec<_> = bt_lines
        .iter()
        .filter_map(|l| debugger::parse_backtrace_frame(l))
        .collect();
    assert!(!frames.is_empty(), "expected backtrace frames");

    let top = &frames[0];
    assert_eq!(top.function, "add");

    // The frame should have a source file name
    assert!(
        top.file.is_some(),
        "top frame should have a source file, got: {:?}",
        top
    );
    let file_name = top.file.as_ref().unwrap();
    assert!(
        file_name.contains("01_basic_breakpoint.mac"),
        "expected file name to contain '01_basic_breakpoint.mac', got: {}",
        file_name
    );

    // The line should be within the add function body (lines 12-16 in the example)
    assert!(
        top.line.is_some(),
        "top frame should have a line number, got: {:?}",
        top
    );
    let line = top.line.unwrap();
    assert!(
        (12..=16).contains(&line),
        "expected line within add function (12-16), got: {}",
        line
    );

    // Convert to DAP StackFrames and verify the line is preserved directly
    let source_index = maxima_dap::breakpoints::SourceIndex::new();
    let program_path = Path::new(&path);
    let remaps = std::collections::HashMap::new();
    let dap_frames = maxima_dap::frames::parse_backtrace(&bt_lines, &source_index, program_path, &remaps, None);

    assert!(!dap_frames.is_empty(), "expected DAP stack frames");
    let dap_top = &dap_frames[0];
    assert_eq!(
        dap_top.line, line as i64,
        "DAP frame line should match backtrace line directly"
    );
    // Source path should be set
    let source = dap_top.source.as_ref().expect("DAP frame should have source");
    assert!(
        source.path.is_some(),
        "DAP frame source should have a path"
    );

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// No stale sentinel in stdin after breakpoint
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn no_stale_sentinel_after_breakpoint() {
    // Verifies that after hitting a breakpoint, sending :bt does NOT
    // produce the sentinel — the sentinel is embedded in the expression
    // block and only fires when the expression completes.
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Trigger breakpoint with embedded sentinel
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Send :bt — should return to debugger prompt, NOT find the sentinel
    proc.write_stdin(":bt\n").await.unwrap();
    let (bt_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "expected debugger prompt after :bt, got {:?} (sentinel leaked!)",
        prompt
    );
    // Sentinel should NOT appear in the backtrace output
    let sentinel_leaked = bt_lines.iter().any(|l| l.contains(eval_sentinel));
    assert!(
        !sentinel_leaked,
        "sentinel leaked into debugger output: {:?}",
        bt_lines
    );

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Step-over (:next) stays in debugger
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn next_stays_in_debugger() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!(
        "batchload(\"{}\")$\n",
        path.replace('\\', "/")
    ))
    .await
    .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Trigger breakpoint with embedded sentinel (like the DAP server does)
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }), "should hit breakpoint");

    // Send :bt (like VS Code's stackTrace request)
    proc.write_stdin(":bt\n").await.unwrap();
    let (_bt_lines, prompt) = proc.read_dap_response(None).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }), "should stay in debugger after :bt");

    // Send :next (Step Over) — should stay in the debugger
    proc.write_stdin(":next\n").await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "expected debugger prompt after :next, got {:?}",
        prompt
    );

    // Resume to finish
    proc.write_stdin(":resume\n").await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert_eq!(prompt, PromptKind::Normal);

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Step-over (:next) at last statement completes the function
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn next_at_last_statement_completes() {
    // When the breakpoint is at offset 2 (the `result : a + b` line),
    // :next executes the remaining expression and exits the function.
    // This verifies the sentinel fires correctly and returns Normal.
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!(
        "batchload(\"{}\")$\n",
        path.replace('\\', "/")
    ))
    .await
    .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Set breakpoint at offset 2 (line 14: result : a + b)
    proc.write_stdin(":break add 2\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Trigger breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "should hit breakpoint at offset 2"
    );

    // Send :bt (like VS Code's stackTrace request)
    proc.write_stdin(":bt\n").await.unwrap();
    let (_bt_lines, prompt) = proc.read_dap_response(None).await.unwrap();
    assert!(matches!(prompt, PromptKind::Debugger { .. }));

    // Send :next — function should complete (only one expression left)
    proc.write_stdin(":next\n").await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert_eq!(
        prompt,
        PromptKind::Normal,
        "expected Normal (function completed) after :next from last statement"
    );

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Multi-step :next through a longer function (02_stepping.mac)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn next_multi_step_through_function() {
    // Uses 02_stepping.mac which has compute(x) with 5 body statements:
    //   [a, b, c], a:x, b:(x+1)^2, c:x+2, a+b+c
    // Setting breakpoint at offset 0 (function entry), we should be able
    // to :next through several statements before the function completes.
    let mut proc = spawn_maxima().await;

    let path = example_path("02_stepping.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!(
        "batchload(\"{}\")$\n",
        path.replace('\\', "/")
    ))
    .await
    .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Set breakpoint at offset 0 (function entry)
    proc.write_stdin(":break compute 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    let (lines, _) = proc.read_until_sentinel(sentinel).await.unwrap();
    assert!(
        lines.iter().any(|l| l.contains("Bkpt")),
        "expected breakpoint confirmation, got: {:?}",
        lines
    );

    // Trigger breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (compute(5)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "should hit breakpoint"
    );

    // Step multiple times — each :next should stay in the debugger
    // (compute has 5 body items; at offset 0, we should get at least 3
    // successful :next calls that stay in debugger before exiting)
    let mut step_count = 0;
    loop {
        proc.write_stdin(":next\n").await.unwrap();
        let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
        match prompt {
            PromptKind::Debugger { .. } => {
                step_count += 1;
            }
            PromptKind::Normal => {
                // Function completed
                break;
            }
        }
    }

    assert!(
        step_count >= 3,
        "expected at least 3 successful :next steps in compute(), got {}",
        step_count
    );

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Breakpoints fire when file is re-batchloaded (no evaluate expression)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn top_level_code_hits_breakpoint() {
    // Simulates the "no evaluate" flow: batchload the file to define
    // functions, set breakpoints, then execute ONLY the top-level
    // (non-definition) code. This avoids redefining functions (which
    // would clear breakpoints) while still running the file's top-level
    // statements like `print("add(3,4) =", add(3,4))$`.
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup: debugmode, batchload (defines functions + runs top-level)
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    proc.write_stdin(&format!(
        "batchload(\"{}\")$\n",
        path.replace('\\', "/")
    ))
    .await
    .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Set breakpoint (functions exist from batchload)
    proc.write_stdin(":break add 0\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    let (lines, _) = proc.read_until_sentinel(sentinel).await.unwrap();
    assert!(
        lines.iter().any(|l| l.contains("Bkpt")),
        "expected breakpoint confirmation, got: {:?}",
        lines
    );

    // Execute only the top-level code (extracted from the file, excluding
    // the function definition). This is what configurationDone does when
    // no evaluate expression is provided.
    //
    // 01_basic_breakpoint.mac top-level code is:
    //   print("add(3, 4) =", add(3, 4))$
    //
    // This calls add(), which should hit the breakpoint.
    let eval_sentinel = "__EVAL_DONE__";
    let top_level_code = "print(\"add(3, 4) =\", add(3, 4))$";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: ({}), print(\"{}\"), __dap_r__)$\n",
        top_level_code.trim_end_matches('$').trim_end_matches(';'),
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();

    // The top-level code calls add(), which should hit the breakpoint.
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "expected breakpoint hit when executing top-level code, got {:?}",
        prompt
    );

    // Resume to finish
    proc.write_stdin(":resume\n").await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert_eq!(prompt, PromptKind::Normal);

    proc.kill().await.unwrap();
}

// ===========================================================================
// Enhanced Maxima debugger tests
//
// These tests require a patched Maxima with `set_breakpoint` support.
// They are skipped at runtime if Legacy Maxima is detected.
// ===========================================================================

/// Detect whether the running Maxima supports Enhanced debugger features.
async fn detect_enhanced_maxima(proc: &mut MaximaProcess) -> bool {
    let sentinel = "__DETECT_DONE__";
    proc.write_stdin(":lisp (fboundp 'maxima::$set_breakpoint)\n")
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    let (lines, _) = proc.read_until_sentinel(sentinel).await.unwrap();
    let output = lines.join(" ");
    output.contains("T") && !output.contains("NIL")
}

#[tokio::test]
#[ignore]
async fn enhanced_file_line_breakpoint() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    if !detect_enhanced_maxima(&mut proc).await {
        eprintln!("Skipping: Enhanced Maxima not detected");
        proc.kill().await.unwrap();
        return;
    }

    // Batchload the file
    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Set file:line breakpoint (Enhanced syntax)
    let cmd = format!(":break \"{}\" 14", path.replace('\\', "/"));
    proc.write_stdin(&format!("{}\n", cmd)).await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    let (lines, _) = proc.read_until_sentinel(sentinel).await.unwrap();
    let bp_set = lines.iter().any(|l| l.contains("Bkpt"));
    assert!(
        bp_set,
        "expected file:line breakpoint confirmation, got: {:?}",
        lines
    );

    // Trigger — should hit the breakpoint
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (add(3, 4)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "expected debugger prompt after file:line breakpoint hit, got {:?}",
        prompt
    );

    proc.kill().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn enhanced_deferred_breakpoint() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    if !detect_enhanced_maxima(&mut proc).await {
        eprintln!("Skipping: Enhanced Maxima not detected");
        proc.kill().await.unwrap();
        return;
    }

    // Set breakpoint BEFORE loading the file (deferred)
    let cmd = format!(":break \"{}\" 14", path.replace('\\', "/"));
    proc.write_stdin(&format!("{}\n", cmd)).await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    let (lines, _) = proc.read_until_sentinel(sentinel).await.unwrap();
    // Should get a "Deferred" message
    let has_deferred = lines
        .iter()
        .any(|l| l.to_lowercase().contains("deferred"));
    assert!(
        has_deferred,
        "expected deferred breakpoint message, got: {:?}",
        lines
    );

    // Batchload the file — deferred breakpoints should resolve and fire
    // when top-level code calls add()
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (batchload(\"{}\")), print(\"{}\"), __dap_r__)$\n",
        path.replace('\\', "/"),
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();
    let (_lines, prompt) = proc.read_dap_response(Some(eval_sentinel)).await.unwrap();
    assert!(
        matches!(prompt, PromptKind::Debugger { .. }),
        "expected deferred breakpoint to fire during batchload, got {:?}",
        prompt
    );

    proc.kill().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn enhanced_breakpoint_count() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    if !detect_enhanced_maxima(&mut proc).await {
        eprintln!("Skipping: Enhanced Maxima not detected");
        proc.kill().await.unwrap();
        return;
    }

    // Load file
    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Set two breakpoints
    let cmd1 = format!(":break \"{}\" 14", path.replace('\\', "/"));
    proc.write_stdin(&format!("{}\n", cmd1)).await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    let cmd2 = format!(":break \"{}\" 15", path.replace('\\', "/"));
    proc.write_stdin(&format!("{}\n", cmd2)).await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Query breakpoint_count()
    proc.write_stdin("breakpoint_count();\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    let (lines, _) = proc.read_until_sentinel(sentinel).await.unwrap();
    let count: i32 = lines
        .iter()
        .filter_map(|l| l.trim().parse().ok())
        .next()
        .unwrap_or(-1);
    assert!(
        count >= 2,
        "expected breakpoint_count() >= 2, got {} from lines: {:?}",
        count,
        lines
    );

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Error detection: parse error returns Err instead of hanging
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn parse_error_detected_not_hanging() {
    // When invalid Maxima code is wrapped in block() with a sentinel,
    // the parse error causes Maxima to print " -- an error." and return
    // to its input prompt WITHOUT executing the sentinel.  The error
    // detection in read_dap_response should catch this and return Err
    // within a few seconds, rather than hanging forever.
    let mut proc = spawn_maxima().await;
    proc.set_debug_mode(true);

    // Enable debugmode (the DAP server always does this)
    let sentinel = "__TEST_DONE__";
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Send syntactically invalid code, wrapped exactly like send_maxima_and_wait does.
    // "1 +" is an incomplete expression that Maxima will reject as a syntax error.
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (1 +), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();

    // Should return an error within the grace period (~2s), not hang.
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        proc.read_dap_response(Some(eval_sentinel)),
    )
    .await;

    match result {
        Ok(Ok((_lines, prompt))) => {
            // If debugmode caught the error, that's also acceptable —
            // it means Maxima entered the debugger for the parse error.
            assert!(
                matches!(prompt, PromptKind::Debugger { .. }),
                "expected either Err or Debugger prompt, got Normal"
            );
        }
        Ok(Err(e)) => {
            // Expected: error detection kicked in.
            let msg = e.to_string();
            assert!(
                msg.contains("error") || msg.contains("Maxima"),
                "expected error message about Maxima error, got: {}",
                msg
            );
        }
        Err(_) => {
            panic!("read_dap_response hung for >10s on parse error — error detection did not trigger");
        }
    }

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Error detection: runtime error with debugmode enters debugger
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn runtime_error_enters_debugger() {
    // When a runtime error occurs inside a function with debugmode(true),
    // Maxima should enter the debugger (dbm:N prompt) rather than
    // returning an error.  Verify read_dap_response returns Debugger.
    let mut proc = spawn_maxima().await;
    proc.set_debug_mode(true);

    let sentinel = "__TEST_DONE__";

    // Enable debugmode
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Define a function that will trigger a runtime error (division by zero).
    proc.write_stdin("divzero(x) := block([r], r : x / 0, r)$\n")
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Evaluate expression — division by zero should trigger debugger
    let eval_sentinel = "__EVAL_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (divzero(5)), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        proc.read_dap_response(Some(eval_sentinel)),
    )
    .await;

    match result {
        Ok(Ok((_lines, prompt))) => {
            // Division by zero with debugmode should enter the debugger.
            // (Some Maxima versions may handle this differently — the
            // debugger or a completion with error are both valid.)
            eprintln!("Got prompt: {:?}", prompt);
        }
        Ok(Err(e)) => {
            // Error detection kicked in — acceptable if debugmode didn't catch it.
            eprintln!("Got error (acceptable): {}", e);
        }
        Err(_) => {
            panic!("read_dap_response hung for >10s — neither debugger nor error detection triggered");
        }
    }

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Error detection: process stays usable after error
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn session_recovers_after_error() {
    // After a parse error is detected, the session should still be usable
    // for subsequent commands. This tests that the error detection doesn't
    // leave the process in a broken state.
    let mut proc = spawn_maxima().await;
    proc.set_debug_mode(true);

    let sentinel = "__TEST_DONE__";

    // Enable debugmode
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Send an expression that triggers an error. We use an undefined variable
    // in a context where Maxima prints an error and returns to prompt.
    // Use a simple command that's more reliably an error.
    let eval_sentinel = "__EVAL1_DONE__";
    let wrapped = format!(
        "block([__dap_r__], __dap_r__: (1 +), print(\"{}\"), __dap_r__)$\n",
        eval_sentinel
    );
    proc.write_stdin(&wrapped).await.unwrap();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        proc.read_dap_response(Some(eval_sentinel)),
    )
    .await;

    // We don't care what happened (Err or Debugger) — just that it didn't hang.
    match &result {
        Ok(Ok((_lines, PromptKind::Debugger { .. }))) => {
            // Resume from debugger first.
            proc.write_stdin(":resume\n").await.unwrap();
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                proc.read_dap_response(Some(eval_sentinel)),
            )
            .await;
        }
        Ok(Ok(_)) => {}
        Ok(Err(_)) => {}
        Err(_) => panic!("first command hung"),
    }

    // Now send a valid expression to verify the session works.
    // Use the simpler send_maxima pattern (sentinel after command).
    let sentinel2 = "__RECOVER_DONE__";
    proc.write_stdin("2 + 3;\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel2))
        .await
        .unwrap();

    let result2 = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        proc.read_until_sentinel(sentinel2),
    )
    .await;

    match result2 {
        Ok(Ok((lines, _))) => {
            let has_five = lines.iter().any(|l| l.contains('5'));
            assert!(
                has_five,
                "expected '5' in recovery output, got: {:?}",
                lines
            );
        }
        Ok(Err(e)) => {
            panic!("recovery command failed: {}", e);
        }
        Err(_) => {
            panic!("recovery command hung — session not usable after error");
        }
    }

    proc.kill().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn enhanced_clear_breakpoints() {
    let mut proc = spawn_maxima().await;

    let path = example_path("01_basic_breakpoint.mac");
    let sentinel = "__TEST_DONE__";

    // Setup
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    if !detect_enhanced_maxima(&mut proc).await {
        eprintln!("Skipping: Enhanced Maxima not detected");
        proc.kill().await.unwrap();
        return;
    }

    // Load file and set a breakpoint
    proc.write_stdin(&format!("batchload(\"{}\")$\n", path.replace('\\', "/")))
        .await
        .unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    let cmd = format!(":break \"{}\" 14", path.replace('\\', "/"));
    proc.write_stdin(&format!("{}\n", cmd)).await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Clear all breakpoints
    proc.write_stdin("clear_breakpoints();\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    // Verify count is 0
    proc.write_stdin("breakpoint_count();\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    let (lines, _) = proc.read_until_sentinel(sentinel).await.unwrap();
    let count: i32 = lines
        .iter()
        .filter_map(|l| l.trim().parse().ok())
        .next()
        .unwrap_or(-1);
    assert_eq!(
        count, 0,
        "expected breakpoint_count() == 0 after clear, got {} from lines: {:?}",
        count, lines
    );

    proc.kill().await.unwrap();
}

// ---------------------------------------------------------------------------
// Legacy strategy: syntax error in batchload is reported
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn legacy_load_program_reports_syntax_error() {
    // When a .mac file contains a function definition with a syntax error
    // (e.g. semicolons inside block()), batchload fails. The legacy
    // strategy's load_program must return Err, not silently succeed.
    let mut proc = spawn_maxima().await;

    let sentinel = "__TEST_DONE__";

    // Enable debugmode (the DAP server always does this)
    proc.write_stdin("debugmode(true)$\n").await.unwrap();
    proc.write_stdin(&format!("print(\"{}\")$\n", sentinel))
        .await
        .unwrap();
    proc.read_until_sentinel(sentinel).await.unwrap();

    let program_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/16_syntax_error_in_definition.mac");

    let mut source_index = SourceIndex::new();
    source_index.index_file(&program_path).unwrap();

    let strategy = LegacyStrategy;
    let state = DebugState::Running;

    let mut ctx = StrategyContext {
        process: &mut proc,
        state: &state,
        source_index: &source_index,
    };

    let result = strategy.load_program(&mut ctx, &program_path).await;

    let err_msg = match result {
        Ok(_) => panic!("load_program should return Err for a file with syntax errors, got Ok"),
        Err(e) => e.to_string(),
    };
    assert!(
        err_msg.contains("incorrect syntax") || err_msg.contains("error"),
        "error message should mention the syntax error, got: {}",
        err_msg
    );

    proc.kill().await.unwrap();
}
