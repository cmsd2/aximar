//! Legacy breakpoint strategy for stock Maxima.
//!
//! Uses function+offset breakpoints (`:break func N`), a temp file for
//! definitions-only loading, and top-level code extraction for the
//! evaluate expression. This is the original behavior before Enhanced
//! Maxima support.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use aximar_core::error::AppError;
use aximar_core::maxima::debugger::{self, CanonicalLocation};
use regex::Regex;
use tracing;

use crate::breakpoints::{self, BreakpointMapping, SourceIndex};
use crate::strategy::{
    frame_paths_from_frame_command, BreakpointStrategy, LoadResult, SetBreakpointResult,
    StrategyContext,
};

/// Legacy strategy using function+offset breakpoints.
pub struct LegacyStrategy;

#[async_trait::async_trait]
impl BreakpointStrategy for LegacyStrategy {
    async fn load_program(
        &self,
        ctx: &mut StrategyContext<'_>,
        program_path: &Path,
    ) -> Result<LoadResult, AppError> {
        let source = std::fs::read_to_string(program_path)?;
        let mac_file = ctx
            .source_index
            .get(program_path)
            .ok_or(AppError::CommunicationError("file not indexed".to_string()))?;

        let definitions = breakpoints::extract_definitions(&source, mac_file);
        if definitions.trim().is_empty() {
            tracing::debug!("LegacyStrategy::load_program: no definitions to load");
            return Ok(LoadResult {
                loaded_path: program_path.to_path_buf(),
                temp_file: None,
            });
        }

        // Write definitions to a named temp file and batchload it.
        let temp_file = tempfile::Builder::new()
            .prefix(".maxima-dap-")
            .suffix(".mac")
            .tempfile()?;
        let temp_path = temp_file.path().to_path_buf();
        std::fs::write(&temp_path, &definitions)?;

        let load_cmd = format!(
            "batchload(\"{}\")$\n",
            temp_path.to_string_lossy().replace('\\', "/")
        );
        tracing::debug!(
            "LegacyStrategy::load_program: batchloading definitions from {}",
            temp_path.display()
        );
        let (response_lines, prompt_kind) = ctx.send_maxima(&load_cmd).await?;

        // Check if batchload triggered an error that dropped us into
        // the Maxima debugger.
        let entered_debugger = matches!(
            prompt_kind,
            aximar_core::maxima::debugger::PromptKind::Debugger { .. }
        ) || response_lines
            .iter()
            .any(|l| debugger::detect_debugger_prompt(l).is_some());
        if entered_debugger {
            let _ = ctx.send_maxima(":top\n").await;
            let error_msg = response_lines
                .iter()
                .find(|l| l.contains("error") || l.contains("define:"))
                .cloned()
                .unwrap_or_else(|| "batchload failed".to_string());
            return Err(AppError::CommunicationError(format!(
                "Error loading definitions: {}",
                error_msg
            )));
        }

        // Also check for error markers (e.g. syntax errors) that don't
        // trigger the debugger — Maxima just prints the error and returns
        // to its normal prompt, so the sentinel still arrives and
        // send_maxima succeeds.
        let has_error = response_lines.iter().any(|l| {
            debugger::ERROR_MARKERS
                .iter()
                .any(|marker| l.contains(marker))
        });
        if has_error {
            let error_msg = response_lines
                .iter()
                .find(|l| {
                    debugger::ERROR_MARKERS
                        .iter()
                        .any(|marker| l.contains(marker))
                })
                .cloned()
                .unwrap_or_else(|| "batchload failed".to_string());
            return Err(AppError::CommunicationError(format!(
                "Error loading definitions: {}",
                error_msg
            )));
        }

        Ok(LoadResult {
            loaded_path: temp_path,
            temp_file: Some(temp_file),
        })
    }

    async fn set_breakpoint(
        &self,
        ctx: &mut StrategyContext<'_>,
        source_path: &Path,
        line: i64,
    ) -> SetBreakpointResult {
        let unverified = |message: String| SetBreakpointResult {
            verified: false,
            maxima_id: None,
            actual_line: None,
            message: Some(message),
        };
        let Some(mac_file) = ctx.source_index.get(source_path) else {
            return unverified("Could not parse source file".to_string());
        };
        let function_name = match breakpoints::map_line_to_breakpoint(mac_file, line as u64) {
            BreakpointMapping::Mapped { function_name, .. } => function_name,
            BreakpointMapping::NotInFunction { message } => return unverified(message),
        };
        // This line, then the function's later lines: Maxima's debugger
        // can't stop on some lines (see `evaluator_skips`), so the
        // breakpoint moves to the first one it can stop on, as Enhanced
        // mode's line-snapping does
        let candidates: Vec<(i64, u32)> = (line..)
            .map_while(
                |l| match breakpoints::map_line_to_breakpoint(mac_file, l as u64) {
                    BreakpointMapping::Mapped {
                        function_name: f,
                        offset,
                    } if f == function_name => Some((l, offset)),
                    _ => None,
                },
            )
            .collect();
        tracing::debug!(
            "LegacyStrategy::set_breakpoint: line {} in {}, candidates {:?}",
            line,
            function_name,
            candidates
        );

        for (candidate, offset) in candidates {
            let cmd = format!(":break {} {}", function_name, offset);
            let Ok(lines) = send_command(ctx, &cmd).await else {
                return unverified(format!("Could not set breakpoint in {}", function_name));
            };
            tracing::debug!("LegacyStrategy::set_breakpoint: {} -> {:?}", cmd, lines);
            // No Bkpt: nothing on this line to stop on, such as a bare value
            let Some(id) = bkpt_id(&lines) else {
                continue;
            };
            let query = format!(
                ":lisp (princ (caar (car (aref maxima::*break-points* {}))))",
                id
            );
            let skipped = match send_command(ctx, &query).await {
                Ok(lines) => evaluator_skips(&lines),
                Err(_) => false,
            };
            if skipped {
                self.delete_breakpoint(ctx, id).await;
                continue;
            }
            return SetBreakpointResult {
                verified: true,
                maxima_id: Some(id),
                actual_line: (candidate != line).then_some(candidate),
                message: None,
            };
        }
        unverified(format!(
            "Maxima's debugger can't stop on this line or any later line of {}: \
             it skips lines that start with arithmetic (+, -, *, /, ^) or a bare value. \
             Put the breakpoint on an assignment or a function call.",
            function_name
        ))
    }

    async fn delete_breakpoint(&self, ctx: &mut StrategyContext<'_>, maxima_id: u32) {
        let cmd = format!(":delete {}", maxima_id);
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
        source_index: &SourceIndex,
    ) -> Option<String> {
        // The definitions are already loaded; run the file's other top-level
        // statements (loads, settings, globals) before the evaluate
        // expression, as batchloading the file would. Enhanced mode runs
        // `batchload(file), evaluate`, which does the same.
        let top_level = top_level_code(program_path, source_index);
        let evaluate = evaluate.map(str::trim).filter(|e| !e.is_empty());
        match (top_level, evaluate) {
            (Some(code), Some(expr)) => Some(format!("{code}, {expr}")),
            (Some(code), None) => Some(code),
            (None, Some(expr)) => Some(expr.to_string()),
            (None, None) => None,
        }
    }

    async fn resolve_frame_paths(
        &self,
        ctx: &mut StrategyContext<'_>,
        frame_indices: &[u32],
    ) -> HashMap<u32, CanonicalLocation> {
        frame_paths_from_frame_command(ctx, frame_indices).await
    }

    fn supports_deferred_breakpoints(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "Legacy"
    }
}

/// The file's top-level statements other than definitions, joined with
/// commas so they can run inside one expression. `None` if there are none.
/// Comments go first: one left at the end would leave a dangling comma.
fn top_level_code(program_path: &Path, source_index: &SourceIndex) -> Option<String> {
    let source = std::fs::read_to_string(program_path).ok()?;
    let mac_file = source_index.get(program_path)?;
    let source = maxima_mac_parser::blank_comments(&source);
    let top_level = breakpoints::extract_top_level_code(&source, mac_file);
    let converted = maxima_mac_parser::replace_terminators(top_level.trim());
    let code = converted.trim().trim_end_matches(',').trim();
    (!code.is_empty()).then(|| code.to_string())
}

/// Mapping metadata from a Legacy breakpoint for storage in `MappedBreakpoint`.
///
/// The Legacy strategy maps file:line to function+offset, and this info
/// is needed for pending breakpoints (set before file is loaded).
pub struct LegacyMapping {
    pub function_name: Option<String>,
    pub offset: Option<u32>,
    pub message: Option<String>,
}

impl LegacyStrategy {
    /// Map a source line to function+offset without setting the breakpoint.
    /// Used when file is not yet loaded (pending breakpoints).
    pub fn map_line(source_index: &SourceIndex, source_path: &Path, line: i64) -> LegacyMapping {
        let mac_file = source_index.get(source_path);
        let mapping = mac_file.map(|f| breakpoints::map_line_to_breakpoint(f, line as u64));

        match mapping {
            Some(BreakpointMapping::Mapped {
                function_name,
                offset,
            }) => LegacyMapping {
                function_name: Some(function_name),
                offset: Some(offset),
                message: None,
            },
            Some(BreakpointMapping::NotInFunction { message }) => LegacyMapping {
                function_name: None,
                offset: None,
                message: Some(message),
            },
            None => LegacyMapping {
                function_name: None,
                offset: None,
                message: Some("Could not parse source file".to_string()),
            },
        }
    }
}

/// Send a command at whichever prompt Maxima is at.
async fn send_command(ctx: &mut StrategyContext<'_>, cmd: &str) -> Result<Vec<String>, AppError> {
    if matches!(ctx.state, crate::types::DebugState::Stopped { .. }) {
        ctx.send_debugger_command_raw(cmd)
            .await
            .map(|(lines, _)| lines)
    } else {
        ctx.send_maxima(&format!("{}\n", cmd))
            .await
            .map(|(lines, _)| lines)
    }
}

static BKPT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"Bkpt\s+(\d+)").unwrap());

/// The ID in Maxima's reply to `:break`, as in `Bkpt 0 for f (in file line 3)`.
fn bkpt_id(lines: &[String]) -> Option<u32> {
    lines
        .iter()
        .find_map(|l| BKPT_RE.captures(l))
        .and_then(|caps| caps[1].parse().ok())
}

/// Whether the form a breakpoint is on is one Maxima's evaluator never
/// checks for breakpoints, from the operator `:lisp` printed. `meval1`
/// evaluates `+`, `*`, `^` and `.` (MPLUS, MTIMES, MEXPT, MNCTIMES)
/// before its debugger hook, so a breakpoint on such a form never fires
/// and `:step` never stops there.
fn evaluator_skips(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|l| matches!(l.trim(), "MPLUS" | "MTIMES" | "MEXPT" | "MNCTIMES"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expression(source: &str, evaluate: Option<&str>) -> Option<String> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.mac");
        std::fs::write(&path, source).unwrap();
        let mut index = SourceIndex::new();
        index.index_file(&path).unwrap();
        LegacyStrategy.build_evaluate_expression(&path, evaluate, &index)
    }

    const PROGRAM: &str =
        "load(mypkg)$\nscale : 10$\n\nf(x) := block(\n  [y],\n  y : g(x) * scale,\n  y)$\n";

    #[test]
    fn top_level_code_runs_before_the_evaluate_expression() {
        // load(mypkg) defines g; without it f(2) can't run (#43)
        let expr = expression(PROGRAM, Some("f(2)")).unwrap();
        assert_eq!(expr, "load(mypkg),\nscale : 10, f(2)");
    }

    #[test]
    fn without_evaluate_only_the_top_level_code_runs() {
        let expr = expression(PROGRAM, None).unwrap();
        assert_eq!(expr, "load(mypkg),\nscale : 10");
        assert_eq!(expression(PROGRAM, Some("  ")), Some(expr));
    }

    #[test]
    fn comments_are_not_top_level_code() {
        // A leading comment alone used to become "/* ... */, f(2)"
        let source = "/* About\n   this file */\nf(x) := x + 1$ /* trailing\n note */\n";
        assert_eq!(expression(source, Some("f(2)")).as_deref(), Some("f(2)"));
        assert_eq!(expression(source, None), None);

        let source = "/* setup */ load(mypkg)$ /* done */\nf(x) := g(x)$\n";
        assert_eq!(
            expression(source, Some("f(2)")).as_deref(),
            Some("load(mypkg), f(2)")
        );
    }

    #[test]
    fn definitions_only_files_run_just_the_evaluate_expression() {
        let source = "f(x) := x + 1$\n";
        assert_eq!(expression(source, Some("f(2)")).as_deref(), Some("f(2)"));
        assert_eq!(expression(source, None), None);
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn reads_the_breakpoint_id_from_maxima_s_reply() {
        let reply = lines("Bkpt 12 for helper (in /tmp/main.mac line 3) ");
        assert_eq!(bkpt_id(&reply), Some(12));
        let none = lines(" No instructions recorded for this line 3 of mid");
        assert_eq!(bkpt_id(&none), None);
    }

    #[test]
    fn arithmetic_forms_are_ones_the_evaluator_skips() {
        // :lisp (princ ...) prints the operator, then its value
        assert!(evaluator_skips(&lines("MTIMES\nMTIMES")));
        assert!(evaluator_skips(&lines("MEXPT")));
        assert!(!evaluator_skips(&lines("MSETQ\nMSETQ")));
        assert!(!evaluator_skips(&lines("$PRINT")));
        assert!(!evaluator_skips(&[]));
    }
}
