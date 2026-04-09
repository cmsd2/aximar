//! Verifies that all example `.mac` files parse correctly and contain
//! function definitions with valid breakpoint mapping targets.

use maxima_dap::breakpoints::{map_line_to_breakpoint, BreakpointMapping};
use maxima_mac_parser::MacItem;
use std::path::Path;

fn example_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn parse_example(name: &str) -> maxima_mac_parser::MacFile {
    let path = example_dir().join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", name, e));
    maxima_mac_parser::parse(&source)
}

fn function_names(file: &maxima_mac_parser::MacFile) -> Vec<&str> {
    file.items
        .iter()
        .filter_map(|item| match item {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => Some(f.name.as_str()),
            _ => None,
        })
        .collect()
}

/// Every example file should parse without panicking.
#[test]
fn all_examples_parse() {
    let dir = example_dir();
    for entry in std::fs::read_dir(&dir).expect("examples dir missing") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("mac") {
            let source = std::fs::read_to_string(&path).unwrap();
            let file = maxima_mac_parser::parse(&source);
            assert!(
                !file.items.is_empty(),
                "{} parsed to empty items",
                path.display()
            );
        }
    }
}

#[test]
fn example_01_basic_breakpoint() {
    let file = parse_example("01_basic_breakpoint.mac");
    let fns = function_names(&file);
    assert!(fns.contains(&"add"), "expected function 'add', got {:?}", fns);

    // The "result : a + b" line should map inside add
    // Find the function and pick a line in the body
    for item in &file.items {
        if let MacItem::FunctionDef(f) = item {
            if f.name == "add" {
                let body_line = f.body_start_line as u64 + 1; // 1-based, offset into body
                match map_line_to_breakpoint(&file, body_line + 1) {
                    BreakpointMapping::Mapped { function_name, .. } => {
                        assert_eq!(function_name, "add");
                    }
                    other => panic!("expected Mapped in add body, got {:?}", other),
                }
            }
        }
    }
}

#[test]
fn example_03_step_into_has_two_functions() {
    let file = parse_example("03_step_into.mac");
    let fns = function_names(&file);
    assert!(fns.contains(&"square"), "missing square, got {:?}", fns);
    assert!(
        fns.contains(&"sum_of_squares"),
        "missing sum_of_squares, got {:?}",
        fns
    );
}

#[test]
fn example_04_recursion() {
    let file = parse_example("04_recursion.mac");
    let fns = function_names(&file);
    assert!(fns.contains(&"my_factorial"), "missing my_factorial, got {:?}", fns);
}

#[test]
fn example_08_multiple_functions() {
    let file = parse_example("08_multiple_functions.mac");
    let fns = function_names(&file);
    assert!(fns.contains(&"validate"), "missing validate");
    assert!(fns.contains(&"process"), "missing process");
    assert!(fns.contains(&"run_pipeline"), "missing run_pipeline");
    assert_eq!(fns.len(), 3);
}

#[test]
fn example_13_deep_call_stack() {
    let file = parse_example("13_deep_call_stack.mac");
    let fns = function_names(&file);
    assert_eq!(fns, vec!["layer_d", "layer_c", "layer_b", "layer_a"]);
}

#[test]
fn example_15_unverified_breakpoints_top_level() {
    let file = parse_example("15_unverified_breakpoints.mac");
    let fns = function_names(&file);
    assert!(fns.contains(&"helper"), "missing helper");

    // Line 1-2 area (top-level assignments) should NOT map to a function
    // Find a top-level assignment line
    let source = std::fs::read_to_string(example_dir().join("15_unverified_breakpoints.mac")).unwrap();
    for (i, line) in source.lines().enumerate() {
        let line_1based = (i + 1) as u64;
        if line.trim().starts_with("x :") || line.trim().starts_with("y :") {
            match map_line_to_breakpoint(&file, line_1based) {
                BreakpointMapping::NotInFunction { .. } => { /* expected */ }
                other => panic!(
                    "line {} ('{}') should be NotInFunction, got {:?}",
                    line_1based, line.trim(), other
                ),
            }
        }
    }

    // Lines inside helper's body should map to helper
    for item in &file.items {
        if let MacItem::FunctionDef(f) = item {
            if f.name == "helper" {
                let body_line = f.body_start_line as u64 + 1 + 1; // 1-based
                match map_line_to_breakpoint(&file, body_line) {
                    BreakpointMapping::Mapped { function_name, .. } => {
                        assert_eq!(function_name, "helper");
                    }
                    other => panic!("expected Mapped in helper body, got {:?}", other),
                }
            }
        }
    }
}
