//! Legacy breakpoint strategy for stock Maxima.
//!
//! Uses function+offset breakpoints (`:break func N`), a temp file for
//! definitions-only loading, and top-level code extraction for the
//! evaluate expression. This is the original behavior before Enhanced
//! Maxima support.

use std::collections::HashMap;
use std::path::Path;

use aximar_core::error::AppError;
use aximar_core::maxima::debugger::{self, CanonicalLocation};
use regex::Regex;
use tracing;

use crate::breakpoints::{self, BreakpointMapping, SourceIndex};
use crate::strategy::{BreakpointStrategy, LoadResult, SetBreakpointResult, StrategyContext};

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
        let has_error = response_lines
            .iter()
            .any(|l| debugger::ERROR_MARKERS.iter().any(|marker| l.contains(marker)));
        if has_error {
            let error_msg = response_lines
                .iter()
                .find(|l| debugger::ERROR_MARKERS.iter().any(|marker| l.contains(marker)))
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
        let mac_file = ctx.source_index.get(source_path);
        let mapping = mac_file.map(|f| breakpoints::map_line_to_breakpoint(f, line as u64));
        tracing::debug!(
            "LegacyStrategy::set_breakpoint: line {} -> {:?}",
            line,
            mapping
        );

        match mapping {
            Some(BreakpointMapping::Mapped {
                function_name,
                offset,
            }) => {
                let cmd = format!(":break {} {}", function_name, offset);
                tracing::debug!("LegacyStrategy::set_breakpoint: {}", cmd);
                let lines = if matches!(ctx.state, crate::types::DebugState::Stopped { .. }) {
                    match ctx.send_debugger_command_raw(&cmd).await {
                        Ok((lines, _)) => lines,
                        Err(_) => {
                            return SetBreakpointResult {
                                verified: false,
                                maxima_id: None,
                                actual_line: None,
                                message: Some(format!(
                                    "Could not set breakpoint in {}",
                                    function_name
                                )),
                            };
                        }
                    }
                } else {
                    match ctx.send_maxima(&format!("{}\n", cmd)).await {
                        Ok((lines, _)) => lines,
                        Err(_) => {
                            return SetBreakpointResult {
                                verified: false,
                                maxima_id: None,
                                actual_line: None,
                                message: Some(format!(
                                    "Could not set breakpoint in {}",
                                    function_name
                                )),
                            };
                        }
                    }
                };

                tracing::debug!(
                    "LegacyStrategy::set_breakpoint: response lines: {:?}",
                    lines
                );
                let bkpt_re = Regex::new(r"Bkpt\s+(\d+)").unwrap();
                for l in &lines {
                    if let Some(caps) = bkpt_re.captures(l) {
                        if let Some(id) =
                            caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok())
                        {
                            tracing::debug!(
                                "LegacyStrategy::set_breakpoint: confirmed, maxima_id={}",
                                id
                            );
                            return SetBreakpointResult {
                                verified: true,
                                maxima_id: Some(id),
                                actual_line: None,
                                message: None,
                            };
                        }
                    }
                }

                tracing::warn!(
                    "LegacyStrategy::set_breakpoint: no Bkpt confirmation in response"
                );
                SetBreakpointResult {
                    verified: false,
                    maxima_id: None,
                    actual_line: None,
                    message: Some(format!(
                        "Could not set breakpoint in {}",
                        function_name
                    )),
                }
            }
            Some(BreakpointMapping::NotInFunction { message }) => SetBreakpointResult {
                verified: false,
                maxima_id: None,
                actual_line: None,
                message: Some(message),
            },
            None => SetBreakpointResult {
                verified: false,
                maxima_id: None,
                actual_line: None,
                message: Some("Could not parse source file".to_string()),
            },
        }
    }

    async fn delete_breakpoint(
        &self,
        ctx: &mut StrategyContext<'_>,
        maxima_id: u32,
    ) {
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
        match evaluate {
            Some(expr) if !expr.trim().is_empty() => Some(expr.to_string()),
            _ => {
                // Extract top-level (non-definition) code from the file.
                let source = std::fs::read_to_string(program_path).ok()?;
                let mac_file = source_index.get(program_path)?;
                let top_level = breakpoints::extract_top_level_code(&source, mac_file);

                let trimmed = top_level.trim();
                if trimmed.is_empty() {
                    return None;
                }

                let converted = maxima_mac_parser::replace_terminators(trimmed);
                let final_code = converted
                    .trim()
                    .trim_end_matches(',')
                    .trim()
                    .to_string();

                if final_code.is_empty() {
                    None
                } else {
                    Some(final_code)
                }
            }
        }
    }

    async fn resolve_frame_paths(
        &self,
        _ctx: &mut StrategyContext<'_>,
        _frame_indices: &[u32],
    ) -> HashMap<u32, CanonicalLocation> {
        // Legacy Maxima doesn't output canonical paths — use heuristic resolution.
        HashMap::new()
    }

    fn supports_deferred_breakpoints(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "Legacy"
    }
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
