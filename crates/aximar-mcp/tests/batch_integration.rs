//! Integration tests for `aximar-mcp run` (batch execution).
//!
//! These tests require Maxima to be installed and on PATH.
//! They are marked `#[ignore]` so they don't run in CI by default.
//! Run with: `cargo test -p aximar-mcp --test batch_integration -- --ignored`

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_BIN_EXE_aximar-mcp"));
    // In case the test binary resolves to a different profile, fall back
    if !path.exists() {
        path = PathBuf::from("target/debug/aximar-mcp");
    }
    path
}

fn write_notebook(dir: &tempfile::TempDir, filename: &str, source: &str) -> PathBuf {
    let path = dir.path().join(filename);
    let notebook = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": {
            "kernelspec": {
                "name": "maxima",
                "display_name": "Maxima",
                "language": "maxima"
            }
        },
        "cells": [
            {
                "cell_type": "code",
                "source": source,
                "metadata": {},
                "execution_count": null,
                "outputs": []
            }
        ]
    });
    fs::write(&path, serde_json::to_string_pretty(&notebook).unwrap()).unwrap();
    path
}

/// Trailing block comments after the last terminator should be stripped
/// and not cause a Maxima parse error.
#[test]
#[ignore]
fn trailing_comment_does_not_cause_error() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_notebook(&dir, "comment.macnb", "x : 42; /* a comment */");
    let output = dir.path().join("out.macnb");

    let result = Command::new(binary_path())
        .args(["run", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
        .output()
        .expect("failed to run aximar-mcp");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "batch run should succeed with trailing comment.\nstderr: {stderr}"
    );

    // Verify the output file has a LaTeX result (x : 42 returns 42)
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
    let outputs = saved["cells"][0]["outputs"].as_array().unwrap();
    assert!(!outputs.is_empty(), "cell should have outputs after execution");
}

/// A cell with only a comment and no statements should be skipped (empty after stripping).
#[test]
#[ignore]
fn comment_only_cell_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_notebook(&dir, "comment_only.macnb", "/* just a comment */");
    let output = dir.path().join("out.macnb");

    let result = Command::new(binary_path())
        .args(["run", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
        .output()
        .expect("failed to run aximar-mcp");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "batch run should succeed with comment-only cell.\nstderr: {stderr}"
    );
}

/// Inline comments (before the terminator) should be preserved and work fine.
#[test]
#[ignore]
fn inline_comment_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let input = write_notebook(&dir, "inline.macnb", "/* setup */ x : 5;");
    let output = dir.path().join("out.macnb");

    let result = Command::new(binary_path())
        .args(["run", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
        .output()
        .expect("failed to run aximar-mcp");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "batch run should succeed with inline comment.\nstderr: {stderr}"
    );

    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
    let outputs = saved["cells"][0]["outputs"].as_array().unwrap();
    assert!(!outputs.is_empty(), "cell should have outputs");
}
