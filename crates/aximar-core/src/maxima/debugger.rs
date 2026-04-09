//! Maxima debugger output parsing.
//!
//! Maxima's built-in debugger (enabled with `debugmode(true)$`) uses a
//! text-based protocol over stdio. This module provides regex-based parsers
//! for debugger prompts, breakpoint hits, backtrace frames, and variable
//! bindings — shared by the DAP server and any future debugger integrations.

use regex::Regex;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Regex patterns
// ---------------------------------------------------------------------------

/// Matches the debugger prompt `(dbm:N)` where N is the nesting level.
static DEBUGGER_PROMPT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(dbm:(\d+)\)\s*$").unwrap());

/// Matches a breakpoint-hit message.
///
/// Format: `Bkpt N: (file.mac line M, in function $FUNC)`
static BREAKPOINT_HIT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Bkpt\s+(\d+):\s+\(\S+\s+line\s+(\d+),\s+in\s+function\s+\$(\w+)\)").unwrap()
});

/// Matches a backtrace frame: `#N: func(args) (file.mac line M)`
static BACKTRACE_FRAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#(\d+):\s+(\w+)\((.*?)\)\s+\((\S+)\s+line\s+(\d+)\)").unwrap()
});

/// Matches a backtrace frame without source location: `#N: func(args)`
static BACKTRACE_FRAME_NO_SRC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#(\d+):\s+(\w+)\((.*?)\)\s*$").unwrap());

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// What kind of prompt Maxima is showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    /// Normal evaluation prompt (detected by sentinel).
    Normal,
    /// Debugger prompt with a nesting level (1-based).
    Debugger { level: u32 },
}

/// Information about a breakpoint hit event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakpointHit {
    /// Maxima's internal breakpoint ID.
    pub breakpoint_id: u32,
    /// The function where the breakpoint was hit.
    pub function: String,
    /// The line offset within the function.
    pub line: u32,
}

/// A single frame from the Maxima backtrace (`:bt`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacktraceFrame {
    /// Frame index (0 = top of stack).
    pub index: u32,
    /// Function name.
    pub function: String,
    /// Raw argument text (e.g. `x = 5, y = [1, 2, 3]`).
    pub args: String,
    /// Source file name, if available.
    pub file: Option<String>,
    /// Line number in the source file, if available.
    pub line: Option<u32>,
}

// ---------------------------------------------------------------------------
// Parsing functions
// ---------------------------------------------------------------------------

/// Detect whether a line is a debugger prompt (`dbm:N>`).
///
/// Returns the debugger nesting level if it matches.
pub fn detect_debugger_prompt(line: &str) -> Option<u32> {
    DEBUGGER_PROMPT_RE
        .captures(line.trim())
        .and_then(|c| c.get(1)?.as_str().parse().ok())
}

/// Parse a breakpoint-hit message from Maxima output.
pub fn parse_breakpoint_hit(line: &str) -> Option<BreakpointHit> {
    let caps = BREAKPOINT_HIT_RE.captures(line)?;
    Some(BreakpointHit {
        breakpoint_id: caps.get(1)?.as_str().parse().ok()?,
        function: caps.get(3)?.as_str().to_ascii_lowercase(),
        line: caps.get(2)?.as_str().parse().ok()?,
    })
}

/// Parse a single backtrace frame from `:bt` output.
pub fn parse_backtrace_frame(line: &str) -> Option<BacktraceFrame> {
    // Try the full form with source location first
    if let Some(caps) = BACKTRACE_FRAME_RE.captures(line) {
        return Some(BacktraceFrame {
            index: caps.get(1)?.as_str().parse().ok()?,
            function: caps.get(2)?.as_str().to_string(),
            args: caps.get(3)?.as_str().to_string(),
            file: Some(caps.get(4)?.as_str().to_string()),
            line: Some(caps.get(5)?.as_str().parse().ok()?),
        });
    }
    // Fall back to form without source location
    if let Some(caps) = BACKTRACE_FRAME_NO_SRC_RE.captures(line) {
        return Some(BacktraceFrame {
            index: caps.get(1)?.as_str().parse().ok()?,
            function: caps.get(2)?.as_str().to_string(),
            args: caps.get(3)?.as_str().to_string(),
            file: None,
            line: None,
        });
    }
    None
}

/// Parse variable bindings from debugger output.
///
/// Handles nested brackets/parens so that `x = 5, y = [1, 2, 3]` correctly
/// splits into `[("x", "5"), ("y", "[1, 2, 3]")]`.
pub fn parse_variable_bindings(text: &str) -> Vec<(String, String)> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    let mut bindings = Vec::new();
    let mut current_name = String::new();
    let mut current_value = String::new();
    let mut in_value = false;
    let mut depth = 0i32; // bracket/paren nesting depth
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '=' if !in_value && depth == 0 => {
                // Skip leading/trailing whitespace around '='
                in_value = true;
                // Skip whitespace after '='
                while chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            ',' if in_value && depth == 0 => {
                // End of this binding
                let name = current_name.trim().to_string();
                let value = current_value.trim().to_string();
                if !name.is_empty() {
                    bindings.push((name, value));
                }
                current_name.clear();
                current_value.clear();
                in_value = false;
                // Skip whitespace after ','
                while chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            '[' | '(' if in_value => {
                depth += 1;
                current_value.push(ch);
            }
            ']' | ')' if in_value => {
                depth -= 1;
                current_value.push(ch);
            }
            _ => {
                if in_value {
                    current_value.push(ch);
                } else {
                    current_name.push(ch);
                }
            }
        }
    }

    // Don't forget the last binding
    if in_value {
        let name = current_name.trim().to_string();
        let value = current_value.trim().to_string();
        if !name.is_empty() {
            bindings.push((name, value));
        }
    }

    bindings
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- detect_debugger_prompt --

    #[test]
    fn detect_prompt_level_1() {
        assert_eq!(detect_debugger_prompt("(dbm:1)"), Some(1));
    }

    #[test]
    fn detect_prompt_level_3() {
        assert_eq!(detect_debugger_prompt("(dbm:3)"), Some(3));
    }

    #[test]
    fn detect_prompt_with_whitespace() {
        assert_eq!(detect_debugger_prompt("  (dbm:2)  "), Some(2));
    }

    #[test]
    fn detect_prompt_trailing_space() {
        // Maxima outputs "(dbm:1) " with a trailing space and no newline
        assert_eq!(detect_debugger_prompt("(dbm:1) "), Some(1));
    }

    #[test]
    fn detect_prompt_not_matching() {
        assert_eq!(detect_debugger_prompt("(%i1)"), None);
        assert_eq!(detect_debugger_prompt("(dbm:)"), None);
        assert_eq!(detect_debugger_prompt("some output (dbm:1) here"), None);
    }

    // -- parse_breakpoint_hit --

    #[test]
    fn parse_breakpoint_hit_basic() {
        let hit = parse_breakpoint_hit(
            "Bkpt 0: (test_debug.mac line 3, in function $ADD)",
        )
        .unwrap();
        assert_eq!(hit.breakpoint_id, 0);
        assert_eq!(hit.function, "add");
        assert_eq!(hit.line, 3);
    }

    #[test]
    fn parse_breakpoint_hit_underscore_name() {
        let hit = parse_breakpoint_hit(
            "Bkpt  2: (file.mac line 10, in function $BAR_BAZ)",
        )
        .unwrap();
        assert_eq!(hit.breakpoint_id, 2);
        assert_eq!(hit.function, "bar_baz");
        assert_eq!(hit.line, 10);
    }

    #[test]
    fn parse_breakpoint_hit_no_match() {
        assert!(parse_breakpoint_hit("some other output").is_none());
    }

    // -- parse_backtrace_frame --

    #[test]
    fn parse_frame_with_source() {
        let frame =
            parse_backtrace_frame("#0: foo(x = 5, y = 10) (test.mac line 7)").unwrap();
        assert_eq!(frame.index, 0);
        assert_eq!(frame.function, "foo");
        assert_eq!(frame.args, "x = 5, y = 10");
        assert_eq!(frame.file.as_deref(), Some("test.mac"));
        assert_eq!(frame.line, Some(7));
    }

    #[test]
    fn parse_frame_without_source() {
        let frame = parse_backtrace_frame("#1: bar(a = 3)").unwrap();
        assert_eq!(frame.index, 1);
        assert_eq!(frame.function, "bar");
        assert_eq!(frame.args, "a = 3");
        assert!(frame.file.is_none());
        assert!(frame.line.is_none());
    }

    #[test]
    fn parse_frame_no_match() {
        assert!(parse_backtrace_frame("not a frame").is_none());
    }

    // -- parse_variable_bindings --

    #[test]
    fn parse_bindings_simple() {
        let bindings = parse_variable_bindings("x = 5, y = 10");
        assert_eq!(bindings, vec![
            ("x".into(), "5".into()),
            ("y".into(), "10".into()),
        ]);
    }

    #[test]
    fn parse_bindings_with_list() {
        let bindings = parse_variable_bindings("x = 5, y = [1, 2, 3]");
        assert_eq!(bindings, vec![
            ("x".into(), "5".into()),
            ("y".into(), "[1, 2, 3]".into()),
        ]);
    }

    #[test]
    fn parse_bindings_nested() {
        let bindings = parse_variable_bindings("a = [[1, 2], [3, 4]], b = f(x, y)");
        assert_eq!(bindings, vec![
            ("a".into(), "[[1, 2], [3, 4]]".into()),
            ("b".into(), "f(x, y)".into()),
        ]);
    }

    #[test]
    fn parse_bindings_empty() {
        assert!(parse_variable_bindings("").is_empty());
        assert!(parse_variable_bindings("  ").is_empty());
    }

    #[test]
    fn parse_bindings_single() {
        let bindings = parse_variable_bindings("x = 42");
        assert_eq!(bindings, vec![("x".into(), "42".into())]);
    }
}
