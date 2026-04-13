//! Enhanced breakpoint strategy for patched Maxima with deferred
//! file:line breakpoints.
//!
//! Uses `:break "file.mac" LINE` for file:line breakpoints, deferred
//! breakpoints (set before loading), and direct batchload of the
//! original file (no temp file needed).

use std::collections::HashMap;
use std::path::Path;

use aximar_core::error::AppError;
use aximar_core::maxima::debugger::{self, CanonicalLocation};
use regex::Regex;
use tracing;

use crate::breakpoints::SourceIndex;
use crate::strategy::{BreakpointStrategy, LoadResult, SetBreakpointResult, StrategyContext};

/// Enhanced strategy using file:line breakpoints and deferred resolution.
pub struct EnhancedStrategy;

#[async_trait::async_trait]
impl BreakpointStrategy for EnhancedStrategy {
    async fn load_program(
        &self,
        _ctx: &mut StrategyContext<'_>,
        program_path: &Path,
    ) -> Result<LoadResult, AppError> {
        // Enhanced mode doesn't need to load anything during this phase.
        // The batchload happens in build_evaluate_expression, which allows
        // deferred breakpoints to be set first and then resolved during
        // the batchload.
        tracing::debug!(
            "EnhancedStrategy::load_program: skipping separate load (batchload in evaluate phase)"
        );
        Ok(LoadResult {
            loaded_path: program_path.to_path_buf(),
            temp_file: None,
        })
    }

    async fn set_breakpoint(
        &self,
        ctx: &mut StrategyContext<'_>,
        source_path: &Path,
        line: i64,
    ) -> SetBreakpointResult {
        let path_str = source_path.to_string_lossy().replace('\\', "/");
        let cmd = format!(":break \"{}\" {}", path_str, line);
        tracing::debug!("EnhancedStrategy::set_breakpoint: {}", cmd);

        let lines = if matches!(ctx.state, crate::types::DebugState::Stopped { .. }) {
            match ctx.send_debugger_command_raw(&cmd).await {
                Ok((lines, _)) => lines,
                Err(e) => {
                    tracing::warn!(
                        "EnhancedStrategy::set_breakpoint: error sending command: {}",
                        e
                    );
                    return SetBreakpointResult {
                        verified: false,
                        maxima_id: None,
                        actual_line: None,
                        message: Some(format!("Could not set breakpoint: {}", e)),
                    };
                }
            }
        } else {
            match ctx.send_maxima(&format!("{}\n", cmd)).await {
                Ok((lines, _)) => lines,
                Err(e) => {
                    tracing::warn!(
                        "EnhancedStrategy::set_breakpoint: error sending command: {}",
                        e
                    );
                    return SetBreakpointResult {
                        verified: false,
                        maxima_id: None,
                        actual_line: None,
                        message: Some(format!("Could not set breakpoint: {}", e)),
                    };
                }
            }
        };

        tracing::debug!(
            "EnhancedStrategy::set_breakpoint: response lines: {:?}",
            lines
        );
        parse_enhanced_breakpoint_response(&lines, line)
    }

    async fn delete_breakpoint(
        &self,
        ctx: &mut StrategyContext<'_>,
        maxima_id: u32,
    ) {
        let cmd = format!(":delbreak {}", maxima_id);
        if matches!(ctx.state, crate::types::DebugState::Stopped { .. }) {
            let _ = ctx.send_debugger_command_raw(&cmd).await;
        } else {
            let _ = ctx.send_maxima(&format!("{}\n", cmd)).await;
        }
    }

    fn build_evaluate_expression(
        &self,
        program_path: &Path,
        evaluate: Option<&str>,
        _source_index: &SourceIndex,
    ) -> Option<String> {
        let path_str = program_path.to_string_lossy().replace('\\', "/");
        match evaluate {
            Some(expr) if !expr.trim().is_empty() => {
                Some(format!("batchload(\"{}\"), {}", path_str, expr))
            }
            _ => Some(format!("batchload(\"{}\")", path_str)),
        }
    }

    async fn resolve_frame_paths(
        &self,
        ctx: &mut StrategyContext<'_>,
        frame_indices: &[u32],
    ) -> HashMap<u32, CanonicalLocation> {
        let mut canonical_paths = HashMap::new();
        for &idx in frame_indices {
            let cmd = format!(":frame {}", idx);
            match ctx.send_debugger_command_raw(&cmd).await {
                Ok((frame_lines, _)) => {
                    if let Some(loc) = debugger::find_canonical_location(&frame_lines) {
                        canonical_paths.insert(idx, loc);
                    }
                }
                Err(e) => {
                    tracing::warn!("resolve_frame_paths: :frame {} failed: {}", idx, e);
                }
            }
        }
        canonical_paths
    }

    fn supports_deferred_breakpoints(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "Enhanced"
    }
}

/// Parse the response from an Enhanced `:break "file" LINE` command.
///
/// Possible responses:
/// - `Bkpt N for $func (in file line M)` — immediate verification, line may differ (snapped)
/// - `Breakpoint at FILE line N deferred (file not yet loaded)` — deferred, no ID assigned yet
/// - `Line N has no executable code; adjusted to line M` — line-snapping info (precedes Bkpt)
/// - `No function in FILE contains line N` — error
/// - `No executable code found near line N of FILE` — error
fn parse_enhanced_breakpoint_response(lines: &[String], requested_line: i64) -> SetBreakpointResult {
    let bkpt_re = Regex::new(r"Bkpt\s+(\d+)").unwrap();
    let adjusted_re = Regex::new(r"adjusted to line\s+(\d+)").unwrap();
    // Actual Enhanced Maxima format: "Breakpoint at FILE line N deferred (file not yet loaded)"
    // Note: no breakpoint ID is assigned for deferred breakpoints.
    let deferred_re = Regex::new(r"deferred \(file not yet loaded\)").unwrap();

    let mut maxima_id: Option<u32> = None;
    let mut actual_line: Option<i64> = None;
    let mut is_deferred = false;

    for line in lines {
        // Check for line-snapping adjustment
        if let Some(caps) = adjusted_re.captures(line) {
            if let Some(snapped) = caps.get(1).and_then(|m| m.as_str().parse::<i64>().ok()) {
                actual_line = Some(snapped);
            }
        }

        // Check for immediate breakpoint confirmation: "Bkpt N for $func (in file line M)"
        if let Some(caps) = bkpt_re.captures(line) {
            if let Some(id) = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()) {
                maxima_id = Some(id);
                // Extract line number from "line M" or "at line M"
                let in_line_re = Regex::new(r"(?:at |in .+ )line\s+(\d+)").unwrap();
                if let Some(line_caps) = in_line_re.captures(line) {
                    if let Some(l) = line_caps.get(1).and_then(|m| m.as_str().parse::<i64>().ok())
                    {
                        actual_line = Some(l);
                    }
                }
            }
        }

        // Check for deferred breakpoint (no ID assigned)
        if deferred_re.is_match(line) {
            is_deferred = true;
        }
    }

    if is_deferred {
        // Deferred breakpoints have no Maxima ID until resolution.
        // They'll be matched by line number in refresh_breakpoint_status.
        SetBreakpointResult {
            verified: false,
            maxima_id: None,
            actual_line: None,
            message: Some("Deferred — will resolve when file is loaded".to_string()),
        }
    } else if let Some(id) = maxima_id {
        SetBreakpointResult {
            verified: true,
            maxima_id: Some(id),
            actual_line: if actual_line != Some(requested_line) {
                actual_line
            } else {
                None
            },
            message: None,
        }
    } else {
        // No Bkpt or Deferred found — breakpoint was rejected
        let error_msg = lines
            .iter()
            .find(|l| !l.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| "Could not set breakpoint".to_string());
        SetBreakpointResult {
            verified: false,
            maxima_id: None,
            actual_line: None,
            message: Some(error_msg),
        }
    }
}

/// Parsed breakpoint from execution output.
pub struct ResolvedBreakpoint {
    pub maxima_id: u32,
    pub line: i64,
    pub file: Option<String>,
}

/// Parse breakpoint resolution messages from Maxima output.
///
/// Extracts resolved breakpoint info from lines like:
///   `Bkpt N for $func (in /path/file.mac line M)` — resolution during batchload
///   `Bkpt N: (file.mac line M) (line K of $FUNC)` — `:info :bkpt` format
///   `Bkpt N for func at line M` — legacy format
pub fn parse_breakpoint_resolutions(lines: &[String]) -> Vec<ResolvedBreakpoint> {
    let bkpt_id_re = Regex::new(r"Bkpt\s+(\d+)").unwrap();
    // Breakpoint-hit format: "(in /full/path/file.mac line M)"
    let in_file_re = Regex::new(r"\(in\s+(.+?)\s+line\s+(\d+)\)").unwrap();
    // :info :bkpt format: "(file.mac line M)"  (short filename, no "in" prefix)
    let info_file_re = Regex::new(r"\(([^)]+?)\s+line\s+(\d+)\)").unwrap();
    let at_line_re = Regex::new(r"\bline\s+(\d+)").unwrap();

    let mut results = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if let Some(id_caps) = bkpt_id_re.captures(trimmed) {
            if let Some(id) = id_caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()) {
                // Try breakpoint-hit format first: "(in /path/file.mac line M)"
                if let Some(file_caps) = in_file_re.captures(trimmed) {
                    let file = file_caps.get(1).map(|m| m.as_str().to_string());
                    if let Some(l) =
                        file_caps.get(2).and_then(|m| m.as_str().parse::<i64>().ok())
                    {
                        results.push(ResolvedBreakpoint { maxima_id: id, line: l, file });
                        continue;
                    }
                }
                // Try :info :bkpt format: "(file.mac line M)"
                if let Some(file_caps) = info_file_re.captures(trimmed) {
                    let file = file_caps.get(1).map(|m| m.as_str().to_string());
                    if let Some(l) =
                        file_caps.get(2).and_then(|m| m.as_str().parse::<i64>().ok())
                    {
                        results.push(ResolvedBreakpoint { maxima_id: id, line: l, file });
                        continue;
                    }
                }
                // Fallback: "at line M" (no file info)
                if let Some(line_caps) = at_line_re.captures(trimmed) {
                    if let Some(l) =
                        line_caps.get(1).and_then(|m| m.as_str().parse::<i64>().ok())
                    {
                        results.push(ResolvedBreakpoint { maxima_id: id, line: l, file: None });
                    }
                }
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_immediate_breakpoint() {
        // Actual Enhanced Maxima format: "Bkpt N for $func (in file line M)"
        let lines = vec!["Bkpt 0 for $add (in /tmp/test.mac line 14)".to_string()];
        let result = parse_enhanced_breakpoint_response(&lines, 14);
        assert!(result.verified);
        assert_eq!(result.maxima_id, Some(0));
        assert_eq!(result.actual_line, None); // Same as requested
        assert!(result.message.is_none());
    }

    #[test]
    fn parse_snapped_breakpoint() {
        let lines = vec![
            "Line 13 has no executable code; adjusted to line 14".to_string(),
            "Bkpt 1 for $add (in /tmp/test.mac line 14)".to_string(),
        ];
        let result = parse_enhanced_breakpoint_response(&lines, 13);
        assert!(result.verified);
        assert_eq!(result.maxima_id, Some(1));
        assert_eq!(result.actual_line, Some(14));
        assert!(result.message.is_none());
    }

    #[test]
    fn parse_deferred_breakpoint() {
        // Actual Enhanced Maxima format — no breakpoint ID assigned
        let lines =
            vec!["Breakpoint at /tmp/test.mac line 10 deferred (file not yet loaded)".to_string()];
        let result = parse_enhanced_breakpoint_response(&lines, 10);
        assert!(!result.verified);
        assert_eq!(result.maxima_id, None); // No ID for deferred
        assert!(result.message.unwrap().contains("Deferred"));
    }

    #[test]
    fn parse_rejected_no_function() {
        let lines = vec!["No function in /tmp/test.mac contains line 10".to_string()];
        let result = parse_enhanced_breakpoint_response(&lines, 10);
        assert!(!result.verified);
        assert_eq!(result.maxima_id, None);
        assert!(result.message.is_some());
    }

    #[test]
    fn parse_rejected_no_executable_code() {
        let lines =
            vec!["No executable code found near line 10 of /tmp/test.mac".to_string()];
        let result = parse_enhanced_breakpoint_response(&lines, 10);
        assert!(!result.verified);
        assert_eq!(result.maxima_id, None);
        assert!(result.message.is_some());
    }

    #[test]
    fn parse_legacy_format_still_works() {
        // In case Enhanced Maxima also uses "at line" format in some contexts
        let lines = vec!["Bkpt 2 for $add at line 14".to_string()];
        let result = parse_enhanced_breakpoint_response(&lines, 14);
        assert!(result.verified);
        assert_eq!(result.maxima_id, Some(2));
        assert_eq!(result.actual_line, None);
    }

    #[test]
    fn info_bkpt_enhanced_format() {
        // Breakpoint-hit format with full path
        let lines = vec![
            "Bkpt 0 for $add (in /tmp/test.mac line 14)".to_string(),
            "Bkpt 1 for $mul (in /tmp/test.mac line 20)".to_string(),
        ];
        let result = parse_breakpoint_resolutions(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].maxima_id, 0);
        assert_eq!(result[0].line, 14);
        assert_eq!(result[0].file.as_deref(), Some("/tmp/test.mac"));
        assert_eq!(result[1].maxima_id, 1);
        assert_eq!(result[1].line, 20);
        assert_eq!(result[1].file.as_deref(), Some("/tmp/test.mac"));
    }

    #[test]
    fn info_bkpt_short_filename_format() {
        // Actual :info :bkpt output from Enhanced Maxima
        let lines = vec![
            "Bkpt 0: (debug.mac line 2) (line 1 of $G)".to_string(),
            "Bkpt 1: (temp.mac line 9) (line 1 of $F)".to_string(),
        ];
        let result = parse_breakpoint_resolutions(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].maxima_id, 0);
        assert_eq!(result[0].line, 2);
        assert_eq!(result[0].file.as_deref(), Some("debug.mac"));
        assert_eq!(result[1].maxima_id, 1);
        assert_eq!(result[1].line, 9);
        assert_eq!(result[1].file.as_deref(), Some("temp.mac"));
    }

    #[test]
    fn info_bkpt_legacy_format() {
        let lines = vec![
            "Bkpt 0:(test.mac 3)".to_string(),
        ];
        // Legacy format has no "line" keyword — should not match
        let result = parse_breakpoint_resolutions(&lines);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn info_bkpt_at_line_format() {
        let lines = vec![
            "Bkpt 2 for $add at line 14".to_string(),
        ];
        let result = parse_breakpoint_resolutions(&lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].maxima_id, 2);
        assert_eq!(result[0].line, 14);
        assert_eq!(result[0].file, None);
    }

    #[test]
    fn info_bkpt_skips_noise() {
        let lines = vec![
            "".to_string(),
            "Bkpt 0 for $add (in /tmp/test.mac line 14)".to_string(),
            "some noise".to_string(),
            "Bkpt 1 for $mul (in /tmp/test.mac line 20)".to_string(),
        ];
        let result = parse_breakpoint_resolutions(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].maxima_id, 0);
        assert_eq!(result[0].line, 14);
        assert_eq!(result[1].maxima_id, 1);
        assert_eq!(result[1].line, 20);
    }
}
