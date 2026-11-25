//! Step Out. Maxima's debugger has no step-out command, so the adapter
//! steps with `:step` until the run is back in a caller of the function it
//! was in, reaches a breakpoint, fails or ends.
//!
//! It has to be `:step`: `:next` on a function's last statement doesn't
//! stop in the caller, it runs on to the end of the program. `:step` also
//! enters calls, so a stop in another function may be deeper or a caller;
//! the stack's depth (from `:bt`) tells which. A stop in the starting
//! function itself needs no backtrace, unless the function is on the stack
//! more than once (recursion), when only the depth tells the calls apart.
//!
//! `:step` doesn't say "Bkpt" when it lands on a breakpoint's line, so a
//! stop is also checked against the breakpoints the adapter set.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use aximar_core::maxima::debugger::{self, CanonicalLocation};
use regex::Regex;

use crate::types::MappedBreakpoint;

/// The location line of a debugger stop: `(file.mac line 2, in function $f)`.
static STOP_LOCATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(\S+\s+line\s+\d+,\s+in\s+function\s+\$?([^\s)]+)\)").unwrap());

/// Where a Step Out starts: the function on top of the stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepOutStart {
    /// Lowercase, without Maxima's `$`.
    pub function: String,
    pub depth: usize,
    /// The function is on the stack more than once.
    pub recursive: bool,
}

/// What a `:step` during a Step Out came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterStep {
    /// Still in the starting function: step again.
    Inside,
    /// Stopped at a breakpoint, which ends a Step Out as it ends a Continue.
    AtBreakpoint,
    /// In another function, or a call of the same one: compare the
    /// stack's depth to tell a caller from a callee.
    CheckDepth,
}

fn normalize(function: &str) -> String {
    function.trim_start_matches('$').to_ascii_lowercase()
}

/// The function a debugger stop names, if any.
pub fn stopped_in(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| STOP_LOCATION_RE.captures(line))
        .map(|caps| normalize(&caps[1]))
}

/// The number of frames in `:bt` output.
pub fn depth(backtrace: &[String]) -> usize {
    backtrace
        .iter()
        .filter(|line| debugger::parse_backtrace_frame(line).is_some())
        .count()
}

fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

/// Whether a stop is on the line of a verified breakpoint. `remaps` maps
/// the files Maxima runs (Legacy mode's temp file) to their sources.
pub fn at_breakpoint(
    stop: &CanonicalLocation,
    remaps: &HashMap<PathBuf, PathBuf>,
    breakpoints: &HashMap<PathBuf, Vec<MappedBreakpoint>>,
) -> bool {
    let file = PathBuf::from(&stop.file);
    let file = remaps.get(&file).cloned().unwrap_or(file);
    breakpoints.iter().any(|(source, bps)| {
        same_file(source, &file)
            && bps
                .iter()
                .any(|bp| bp.verified && bp.actual_line.unwrap_or(bp.line) == stop.line as i64)
    })
}

impl StepOutStart {
    /// From `:bt` output. `None` when the stack is empty: the run is in
    /// top-level code, with no function to step out of.
    pub fn from_backtrace(backtrace: &[String]) -> Option<Self> {
        let functions: Vec<String> = backtrace
            .iter()
            .filter_map(|line| debugger::parse_backtrace_frame(line))
            .map(|frame| normalize(&frame.function))
            .collect();
        let function = functions.first()?.clone();
        Some(Self {
            recursive: functions.iter().filter(|f| **f == function).count() > 1,
            depth: functions.len(),
            function,
        })
    }

    /// Decide from the output of one `:step`.
    pub fn after_step(&self, lines: &[String]) -> AfterStep {
        if lines
            .iter()
            .any(|line| debugger::parse_breakpoint_hit(line).is_some())
        {
            return AfterStep::AtBreakpoint;
        }
        match stopped_in(lines) {
            Some(function) if function == self.function && !self.recursive => AfterStep::Inside,
            _ => AfterStep::CheckDepth,
        }
    }

    /// Whether a stack of this depth is in a caller of the starting frame.
    pub fn is_out(&self, depth: usize) -> bool {
        depth < self.depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    const BT: &str =
        "#0: myFunction(x=(b+a)^2) (MyPackage.mac line 1)\n#1: runTask() (main.mac line 8)";

    #[test]
    fn starts_from_the_top_frame() {
        let start = StepOutStart::from_backtrace(&lines(BT)).unwrap();
        assert_eq!(
            start,
            StepOutStart {
                function: "myfunction".into(),
                depth: 2,
                recursive: false
            }
        );
        assert_eq!(StepOutStart::from_backtrace(&[]), None);
    }

    #[test]
    fn steps_on_inside_the_function_and_checks_the_depth_elsewhere() {
        let start = StepOutStart::from_backtrace(&lines(BT)).unwrap();
        let inside = lines("(MyPackage.mac line 2, in function $myFunction)\n(dbm:1)");
        assert_eq!(start.after_step(&inside), AfterStep::Inside);
        // A caller and a callee look alike until the depth is known
        let elsewhere = lines("(main.mac line 9, in function $runTask)\n(dbm:1)");
        assert_eq!(start.after_step(&elsewhere), AfterStep::CheckDepth);
        assert!(start.is_out(1));
        assert!(!start.is_out(3));
        assert_eq!(start.after_step(&lines("(dbm:1)")), AfterStep::CheckDepth);
    }

    #[test]
    fn a_breakpoint_ends_the_step() {
        let start = StepOutStart::from_backtrace(&lines(BT)).unwrap();
        let hit = lines("Bkpt 1: (other.mac line 3, in function $g)\n(dbm:1)");
        assert_eq!(start.after_step(&hit), AfterStep::AtBreakpoint);
    }

    #[test]
    fn a_stop_on_a_breakpoint_s_line_is_a_breakpoint() {
        let bp = |line, verified| MappedBreakpoint {
            dap_id: 1,
            source_path: PathBuf::from("/src/main.mac"),
            line,
            function: None,
            offset: None,
            verified,
            actual_line: None,
            maxima_id: None,
            message: None,
        };
        let breakpoints = HashMap::from([(
            PathBuf::from("/src/main.mac"),
            vec![bp(3, true), bp(7, false)],
        )]);
        // Legacy mode runs a temp copy of the program
        let remaps = HashMap::from([(
            PathBuf::from("/tmp/.maxima-dap-x.mac"),
            PathBuf::from("/src/main.mac"),
        )]);
        let stop = |file: &str, line| CanonicalLocation {
            file: file.into(),
            line,
        };
        assert!(at_breakpoint(
            &stop("/tmp/.maxima-dap-x.mac", 3),
            &remaps,
            &breakpoints
        ));
        assert!(at_breakpoint(
            &stop("/src/main.mac", 3),
            &remaps,
            &breakpoints
        ));
        assert!(!at_breakpoint(
            &stop("/src/main.mac", 4),
            &remaps,
            &breakpoints
        ));
        // An unverified breakpoint never fires
        assert!(!at_breakpoint(
            &stop("/src/main.mac", 7),
            &remaps,
            &breakpoints
        ));
        assert!(!at_breakpoint(
            &stop("/src/other.mac", 3),
            &remaps,
            &breakpoints
        ));
    }

    #[test]
    fn in_recursion_the_same_function_needs_the_depth() {
        let bt =
            lines("#0: f(n=1) (r.mac line 2)\n#1: f(n=2) (r.mac line 2)\n#2: g() (r.mac line 5)");
        let start = StepOutStart::from_backtrace(&bt).unwrap();
        assert!(start.recursive);
        let same_name = lines("(r.mac line 2, in function $f)");
        assert_eq!(start.after_step(&same_name), AfterStep::CheckDepth);
        assert!(start.is_out(2) && !start.is_out(3));
        assert_eq!(depth(&bt), 3);
    }
}
