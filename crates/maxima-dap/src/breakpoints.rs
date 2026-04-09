//! Breakpoint mapping between DAP (file:line) and Maxima (function+offset).
//!
//! Maxima's debugger uses `:break func N` where `N` is an offset from the
//! start of the function body. This module translates between VS Code's
//! file:line breakpoints and Maxima's function+offset breakpoints using
//! the `.mac` parser.

use maxima_mac_parser::{MacFile, MacItem};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Result of mapping a source line to a Maxima breakpoint location.
#[derive(Debug, Clone)]
pub enum BreakpointMapping {
    /// The line maps to a function body.
    Mapped {
        function_name: String,
        /// Offset from the function body start (0-based).
        offset: u32,
    },
    /// The line is not inside any function definition.
    NotInFunction { message: String },
}

/// Cache of parsed `.mac` files for breakpoint mapping.
pub struct SourceIndex {
    files: HashMap<PathBuf, MacFile>,
}

impl SourceIndex {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Parse and cache a `.mac` file. Returns `Ok` even if the file has
    /// parse errors (the parser is fault-tolerant).
    pub fn index_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let source = std::fs::read_to_string(path)?;
        let parsed = maxima_mac_parser::parse(&source);
        self.files.insert(path.to_path_buf(), parsed);
        Ok(())
    }

    /// Get the cached parse result for a file.
    pub fn get(&self, path: &Path) -> Option<&MacFile> {
        self.files.get(path)
    }

    /// Re-index a file (e.g. after reload).
    pub fn reindex_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        self.index_file(path)
    }
}

/// Map a 1-based source line to a Maxima breakpoint location.
///
/// Finds which function definition contains the line, then computes the
/// offset from the function's `body_start_line`.
pub fn map_line_to_breakpoint(file: &MacFile, line_1based: u64) -> BreakpointMapping {
    // Parser uses 0-based lines
    let line_0based = line_1based.saturating_sub(1) as u32;

    // Find all function/macro definitions
    for item in &file.items {
        let func_def = match item {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => f,
            _ => continue,
        };

        // Check if the line falls within this function's span
        if line_0based >= func_def.span.start.line && line_0based <= func_def.span.end.line {
            // Compute offset from body start
            let offset = line_0based.saturating_sub(func_def.body_start_line);
            return BreakpointMapping::Mapped {
                function_name: func_def.name.clone(),
                offset,
            };
        }
    }

    BreakpointMapping::NotInFunction {
        message: format!(
            "Line {} is not inside a function definition. \
             Maxima only supports breakpoints inside functions.",
            line_1based
        ),
    }
}

/// Reverse mapping: given a function name and offset, find the 1-based
/// source line.
pub fn function_offset_to_source_line(
    file: &MacFile,
    function_name: &str,
    offset: u32,
) -> Option<u64> {
    for item in &file.items {
        let func_def = match item {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => f,
            _ => continue,
        };

        if func_def.name == function_name {
            let line_0based = func_def.body_start_line + offset;
            return Some(line_0based as u64 + 1); // Convert to 1-based
        }
    }
    None
}

/// Build a bitmap marking which lines are function/macro definitions.
fn definition_bitmap(source: &str, mac_file: &MacFile) -> Vec<bool> {
    let total_lines = source.lines().count();
    let mut is_definition = vec![false; total_lines];
    for item in &mac_file.items {
        let (start, end) = match item {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => {
                (f.span.start.line as usize, f.span.end.line as usize)
            }
            _ => continue,
        };
        for i in start..=end.min(total_lines.saturating_sub(1)) {
            is_definition[i] = true;
        }
    }
    is_definition
}

/// Extract top-level executable code from a source file, excluding
/// function and macro definitions.
///
/// Returns the concatenated non-definition lines, or an empty string if
/// there are no top-level statements.
pub fn extract_top_level_code(source: &str, mac_file: &MacFile) -> String {
    let is_definition = definition_bitmap(source, mac_file);
    let mut result = String::new();
    for (i, line) in source.lines().enumerate() {
        if !is_definition[i] {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// Extract only function and macro definitions from a source file,
/// replacing top-level executable code with blank lines.
///
/// Blank lines preserve the original line numbering so that Maxima's
/// `:bt` output reports correct line numbers matching the original file.
/// Used to load definitions into Maxima without executing top-level
/// statements, so that breakpoints can be set before any user code runs.
pub fn extract_definitions(source: &str, mac_file: &MacFile) -> String {
    let is_definition = definition_bitmap(source, mac_file);
    let mut result = String::new();
    for (i, line) in source.lines().enumerate() {
        if is_definition[i] {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_test_file() -> MacFile {
        let source = r#"
/* A test file */
x : 42$

foo(a, b) :=
    block([result],
        result : a + b,
        result
    )$

bar(x) := x * 2$
"#;
        maxima_mac_parser::parse(source)
    }

    #[test]
    fn map_line_inside_function() {
        let file = parse_test_file();
        // Line 7 should be inside foo's body (result : a + b)
        match map_line_to_breakpoint(&file, 7) {
            BreakpointMapping::Mapped {
                function_name,
                offset,
            } => {
                assert_eq!(function_name, "foo");
                assert!(offset > 0, "offset should be > 0 for a body line");
            }
            other => panic!("expected Mapped, got {:?}", other),
        }
    }

    #[test]
    fn map_line_outside_function() {
        let file = parse_test_file();
        // Line 3 is the `x : 42$` assignment — not inside any function
        match map_line_to_breakpoint(&file, 3) {
            BreakpointMapping::NotInFunction { .. } => {}
            other => panic!("expected NotInFunction, got {:?}", other),
        }
    }

    #[test]
    fn map_line_at_function_def() {
        let file = parse_test_file();
        // Line 5 is where `foo(a, b) :=` starts
        match map_line_to_breakpoint(&file, 5) {
            BreakpointMapping::Mapped { function_name, .. } => {
                assert_eq!(function_name, "foo");
            }
            other => panic!("expected Mapped, got {:?}", other),
        }
    }

    #[test]
    fn reverse_mapping() {
        let file = parse_test_file();
        // Map line 7 to function+offset, then reverse it
        if let BreakpointMapping::Mapped {
            ref function_name,
            offset,
        } = map_line_to_breakpoint(&file, 7)
        {
            let line = function_offset_to_source_line(&file, function_name, offset);
            assert_eq!(line, Some(7));
        }
    }

    #[test]
    fn reverse_mapping_not_found() {
        let file = parse_test_file();
        assert_eq!(function_offset_to_source_line(&file, "nonexistent", 0), None);
    }

    #[test]
    fn single_line_function() {
        let file = parse_test_file();
        // bar(x) := x * 2$ is on line 11
        match map_line_to_breakpoint(&file, 11) {
            BreakpointMapping::Mapped {
                function_name,
                offset,
            } => {
                assert_eq!(function_name, "bar");
                assert_eq!(offset, 0, "single-line function body should have offset 0");
            }
            other => panic!("expected Mapped, got {:?}", other),
        }
    }

    #[test]
    fn extract_top_level_excludes_definitions() {
        let source = r#"/* header */
x : 42$

foo(a, b) :=
    block([result],
        result : a + b,
        result
    )$

print("result =", foo(3, 4))$
"#;
        let file = maxima_mac_parser::parse(source);
        let top_level = extract_top_level_code(source, &file);

        // Should include: comment, x:42, blank lines, print(...)
        assert!(
            top_level.contains("print("),
            "top-level should include print statement, got: {:?}",
            top_level
        );
        assert!(
            top_level.contains("x : 42"),
            "top-level should include variable assignment, got: {:?}",
            top_level
        );
        // Should NOT include function definition lines
        assert!(
            !top_level.contains("foo(a, b) :="),
            "top-level should NOT include function def, got: {:?}",
            top_level
        );
        assert!(
            !top_level.contains("result : a + b"),
            "top-level should NOT include function body, got: {:?}",
            top_level
        );
    }

    #[test]
    fn extract_top_level_empty_when_only_definitions() {
        let source = "foo(x) := x + 1$\nbar(x) := x * 2$\n";
        let file = maxima_mac_parser::parse(source);
        let top_level = extract_top_level_code(source, &file);
        // Only whitespace/empty lines should remain
        assert!(
            top_level.trim().is_empty(),
            "expected empty top-level for definitions-only file, got: {:?}",
            top_level
        );
    }
}
