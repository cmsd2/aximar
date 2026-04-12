//! Stack trace and variable parsing for the DAP server.
//!
//! Converts Maxima's `:bt` output into DAP `StackFrame` and `Variable` types.

use aximar_core::maxima::debugger;
use emmy_dap_types::types::{Source, StackFrame, Variable};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::breakpoints::SourceIndex;

/// Parse Maxima `:bt` output lines into DAP `StackFrame`s.
///
/// Resolves source locations using the `SourceIndex` when the backtrace
/// frame names a file that has been indexed. `path_remaps` maps temp
/// file paths back to original source paths (e.g. the definitions temp
/// file → the user's `.mac` file).
pub fn parse_backtrace(
    lines: &[String],
    source_index: &SourceIndex,
    program_path: &Path,
    path_remaps: &HashMap<PathBuf, PathBuf>,
    cwd: Option<&Path>,
) -> Vec<StackFrame> {
    let program_dir = program_path.parent();
    let mut frames = Vec::new();

    for line in lines {
        let Some(bt_frame) = debugger::parse_backtrace_frame(line) else {
            continue;
        };

        let (source, dap_line) = resolve_frame_source(
            &bt_frame,
            source_index,
            program_dir,
            cwd,
            path_remaps,
        );

        frames.push(StackFrame {
            id: bt_frame.index as i64,
            name: bt_frame.function.clone(),
            source,
            line: dap_line,
            column: 1,
            end_line: None,
            end_column: None,
            can_restart: None,
            instruction_pointer_reference: None,
            module_id: None,
            presentation_hint: None,
        });
    }

    frames
}

/// Resolve source location for a backtrace frame.
///
/// If the frame has a file name and it's been indexed, try to map the
/// function+line back to a source position. Otherwise, use the raw file
/// name from the backtrace. Applies `path_remaps` to translate temp
/// file paths back to original source paths.
fn resolve_frame_source(
    bt_frame: &debugger::BacktraceFrame,
    _source_index: &SourceIndex,
    program_dir: Option<&Path>,
    cwd: Option<&Path>,
    path_remaps: &HashMap<PathBuf, PathBuf>,
) -> (Option<Source>, i64) {
    let Some(ref file_name) = bt_frame.file else {
        return (None, 0);
    };
    let Some(bt_line) = bt_frame.line else {
        return (make_source(file_name, cwd, program_dir), 0);
    };

    // Check if the file name (bare or full path) matches a remapped path.
    // Maxima may report just the basename or the full temp path.
    if let Some(original) = find_remap(file_name, path_remaps) {
        let display_name = original
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file_name.clone());
        let source = Source {
            name: Some(display_name),
            path: Some(original.to_string_lossy().to_string()),
            ..Default::default()
        };
        return (Some(source), bt_line as i64);
    }

    // No remap — resolve relative paths, trying cwd first (where user
    // files live), then program_dir (where the temp file lives).
    let file_path = resolve_file_path(file_name, cwd, program_dir);
    let source = file_path.as_ref().map(|p| Source {
        name: Some(file_name.clone()),
        path: Some(p.to_string_lossy().to_string()),
        ..Default::default()
    });
    (source.or_else(|| make_source(file_name, cwd, program_dir)), bt_line as i64)
}

/// Check if a file name from Maxima output matches any remapped path.
///
/// Maxima may report the full temp path or just the basename, so we
/// check both the literal path and filename-only matching.
fn find_remap(file_name: &str, remaps: &HashMap<PathBuf, PathBuf>) -> Option<PathBuf> {
    let file_path = PathBuf::from(file_name);

    // Direct match on the full path.
    if let Some(original) = remaps.get(&file_path) {
        return Some(original.clone());
    }

    // Match by filename — Maxima often reports just the basename.
    let query_name = file_path.file_name()?;
    for (temp_path, original) in remaps {
        if temp_path.file_name() == Some(query_name) {
            return Some(original.clone());
        }
    }

    None
}

/// Resolve a file name from a backtrace frame to a full path.
///
/// Tries `cwd` first (where user files are loaded from), then
/// `program_dir` (where the temp/program file lives). Prefers the
/// directory where the file actually exists on disk.
fn resolve_file_path(file_name: &str, cwd: Option<&Path>, program_dir: Option<&Path>) -> Option<PathBuf> {
    let path = Path::new(file_name);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    // Try cwd first, then program_dir. Prefer whichever actually exists.
    // Canonicalize so paths like "../foo.mac" resolve cleanly for VS Code.
    let candidates = [cwd, program_dir];
    for dir in candidates.into_iter().flatten() {
        let candidate = dir.join(file_name);
        if let Ok(canonical) = candidate.canonicalize() {
            return Some(canonical);
        }
    }
    // Neither exists — return cwd-based path as the default.
    cwd.or(program_dir).map(|dir| dir.join(file_name))
}

/// Create a DAP `Source` from a file name.
fn make_source(file_name: &str, cwd: Option<&Path>, program_dir: Option<&Path>) -> Option<Source> {
    let path = resolve_file_path(file_name, cwd, program_dir);
    Some(Source {
        name: Some(file_name.to_string()),
        path: path.map(|p| p.to_string_lossy().to_string()),
        ..Default::default()
    })
}

/// Parse a backtrace frame's argument text into DAP `Variable`s.
pub fn frame_args_to_variables(args_text: &str) -> Vec<Variable> {
    debugger::parse_variable_bindings(args_text)
        .into_iter()
        .map(|(name, value)| Variable {
            name,
            value: value.clone(),
            type_field: None,
            presentation_hint: None,
            evaluate_name: None,
            variables_reference: 0, // not expandable for now
            named_variables: None,
            indexed_variables: None,
            memory_reference: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_backtrace_empty() {
        let source_index = SourceIndex::new();
        let remaps = HashMap::new();
        let frames = parse_backtrace(&[], &source_index, Path::new("/test/file.mac"), &remaps, None);
        assert!(frames.is_empty());
    }

    #[test]
    fn parse_backtrace_with_frames() {
        let lines = vec![
            "#0: foo(x = 5) (test.mac line 3)".to_string(),
            "#1: bar(a = 1, b = 2) (test.mac line 10)".to_string(),
            "some other output".to_string(),
        ];
        let source_index = SourceIndex::new();
        let remaps = HashMap::new();
        let frames = parse_backtrace(&lines, &source_index, Path::new("/test/file.mac"), &remaps, None);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].name, "foo");
        assert_eq!(frames[0].id, 0);
        assert_eq!(frames[1].name, "bar");
        assert_eq!(frames[1].id, 1);
    }

    #[test]
    fn frame_args_to_vars() {
        let vars = frame_args_to_variables("x = 5, y = [1, 2, 3]");
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].name, "x");
        assert_eq!(vars[0].value, "5");
        assert_eq!(vars[1].name, "y");
        assert_eq!(vars[1].value, "[1, 2, 3]");
    }

    #[test]
    fn frame_args_empty() {
        let vars = frame_args_to_variables("");
        assert!(vars.is_empty());
    }
}
