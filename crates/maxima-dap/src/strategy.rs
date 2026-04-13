//! Strategy pattern for breakpoint protocol differences between
//! Legacy Maxima and Enhanced Maxima debuggers.
//!
//! The `BreakpointStrategy` trait encapsulates how breakpoints are set,
//! how the program file is loaded, and how the evaluate expression is
//! built. The `DapServer` holds a `Box<dyn BreakpointStrategy>` chosen
//! at launch based on runtime detection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aximar_core::error::AppError;
use aximar_core::maxima::debugger::{CanonicalLocation, PromptKind};
use aximar_core::maxima::process::MaximaProcess;

use crate::breakpoints::SourceIndex;
use crate::types::DebugState;

/// Result of setting a single breakpoint in Maxima.
#[derive(Debug, Clone)]
pub struct SetBreakpointResult {
    /// Whether the breakpoint was successfully set (or deferred for Enhanced).
    pub verified: bool,
    /// Maxima's internal breakpoint ID (from `:break` response).
    pub maxima_id: Option<u32>,
    /// Actual line after line-snapping (may differ from requested line).
    /// Enhanced mode only — `None` for Legacy.
    pub actual_line: Option<i64>,
    /// Message for unverified/deferred breakpoints.
    pub message: Option<String>,
}

/// Result of loading the program file.
pub struct LoadResult {
    /// Path to the file Maxima associates with function definitions.
    /// For Legacy this is the temp file; for Enhanced it's the original.
    pub loaded_path: PathBuf,
    /// Temp file handle to keep alive (Legacy only).
    pub temp_file: Option<tempfile::NamedTempFile>,
}

/// Mutable context passed to strategy methods, providing access to the
/// Maxima process and debug session state without requiring `&mut DapServer`.
pub struct StrategyContext<'a> {
    pub process: &'a mut MaximaProcess,
    pub state: &'a DebugState,
    pub source_index: &'a SourceIndex,
}

impl<'a> StrategyContext<'a> {
    /// Send a command to Maxima's stdin and read until sentinel or
    /// debugger prompt (in debug mode).
    pub async fn send_maxima(
        &mut self,
        cmd: &str,
    ) -> Result<(Vec<String>, PromptKind), AppError> {
        let sentinel = "__MAXIMA_DAP_DONE__";
        self.process.write_stdin(cmd).await?;
        self.process
            .write_stdin(&format!("print(\"{}\")$\n", sentinel))
            .await?;
        self.process.read_until_sentinel(sentinel).await
    }

    /// Send a raw debugger command and read until debugger prompt.
    pub async fn send_debugger_command_raw(
        &mut self,
        cmd: &str,
    ) -> Result<(Vec<String>, u32), AppError> {
        self.process.write_stdin(&format!("{}\n", cmd)).await?;
        let (lines, prompt_kind) = self.process.read_dap_response(None).await?;
        match prompt_kind {
            PromptKind::Debugger { level, .. } => Ok((lines, level)),
            PromptKind::Normal => Err(AppError::CommunicationError(
                "unexpected normal prompt in debugger context".into(),
            )),
        }
    }
}

/// Abstracts the breakpoint protocol differences between Legacy and
/// Enhanced Maxima debuggers.
#[async_trait::async_trait]
pub trait BreakpointStrategy: Send {
    /// Load the program file into Maxima.
    async fn load_program(
        &self,
        ctx: &mut StrategyContext<'_>,
        program_path: &Path,
    ) -> Result<LoadResult, AppError>;

    /// Set a single breakpoint in Maxima.
    ///
    /// For Legacy: maps file:line to function+offset and sends `:break func offset`.
    /// For Enhanced: sends `:break "file" LINE` directly.
    async fn set_breakpoint(
        &self,
        ctx: &mut StrategyContext<'_>,
        source_path: &Path,
        line: i64,
    ) -> SetBreakpointResult;

    /// Delete a breakpoint by its Maxima-assigned ID.
    async fn delete_breakpoint(
        &self,
        ctx: &mut StrategyContext<'_>,
        maxima_id: u32,
    );

    /// Get the expression to evaluate after loading.
    ///
    /// Returns `None` if there is nothing to evaluate (definitions-only file).
    ///
    /// For Legacy: extracts top-level code from the file.
    /// For Enhanced: wraps the batchload + optional evaluate in a single expression.
    fn build_evaluate_expression(
        &self,
        program_path: &Path,
        evaluate: Option<&str>,
        source_index: &SourceIndex,
    ) -> Option<String>;

    /// Resolve canonical absolute paths for backtrace frames.
    ///
    /// Given the frame indices from a `:bt` output, returns a map of
    /// frame index → canonical location. Enhanced mode issues `:frame N`
    /// for each frame; Legacy returns an empty map (heuristic resolution).
    async fn resolve_frame_paths(
        &self,
        ctx: &mut StrategyContext<'_>,
        frame_indices: &[u32],
    ) -> HashMap<u32, CanonicalLocation>;

    /// Whether this strategy supports deferred breakpoints.
    ///
    /// Enhanced mode sets breakpoints before the file is loaded (they
    /// resolve during batchload). Legacy mode requires functions to exist
    /// before `:break` works, so pending breakpoints must wait.
    fn supports_deferred_breakpoints(&self) -> bool;

    /// Returns a human-readable name for this strategy (for logging).
    fn name(&self) -> &'static str;
}
