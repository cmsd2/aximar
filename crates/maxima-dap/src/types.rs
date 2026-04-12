//! Supplementary types for the DAP server.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Launch arguments sent by the client in the `launch` request.
///
/// These are custom fields embedded in the DAP `launch` request's
/// `additional_data` (since DAP launch args are debugger-specific).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaximaLaunchArguments {
    /// Path to the `.mac` file to debug.
    pub program: String,
    /// Custom path to the `maxima` binary (optional).
    #[serde(default)]
    pub maxima_path: Option<String>,
    /// Backend type: "local", "docker", or "wsl" (default: "local").
    #[serde(default = "default_backend")]
    pub backend: String,
    /// Whether to stop on entry (at the first statement).
    #[serde(default)]
    pub stop_on_entry: bool,
    /// An expression to evaluate after loading the program.
    /// Execution starts when this expression is evaluated.
    #[serde(default)]
    pub evaluate: Option<String>,
    /// Working directory for the debug session.
    #[serde(default)]
    pub cwd: Option<String>,
    /// Evaluation timeout in seconds.  If an expression takes longer
    /// than this, Maxima is interrupted and the session terminates.
    /// Set to 0 to disable.  Default: 60.
    #[serde(default = "default_eval_timeout")]
    pub eval_timeout: u64,
}

fn default_eval_timeout() -> u64 {
    60
}

fn default_backend() -> String {
    "local".to_string()
}

/// Debug session state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugState {
    /// Before `initialize` request.
    Uninitialized,
    /// After `initialize`, before `launch`.
    Initialized,
    /// Program is running (not at a breakpoint).
    Running,
    /// Stopped at a debugger prompt with the given nesting level.
    Stopped {
        level: u32,
        /// Canonical absolute file path from Enhanced Maxima's `file:line::` output.
        canonical_file: Option<String>,
        /// Line number from the canonical location.
        canonical_line: Option<u32>,
    },
    /// Session has ended.
    Terminated,
}

/// A breakpoint mapped between DAP (file:line) and Maxima (function+offset).
#[derive(Debug, Clone)]
pub struct MappedBreakpoint {
    /// DAP-assigned breakpoint ID.
    pub dap_id: i64,
    /// Source file path.
    pub source_path: PathBuf,
    /// 1-based line number in the source file.
    pub line: i64,
    /// Maxima function name (if the line is inside a function).
    pub function: Option<String>,
    /// Offset within the function body (Legacy mode only).
    pub offset: Option<u32>,
    /// Whether the breakpoint was successfully set in Maxima.
    pub verified: bool,
    /// Actual line after line-snapping (Enhanced mode only).
    /// When set, differs from `line` — the breakpoint was moved to a nearby executable line.
    pub actual_line: Option<i64>,
    /// Maxima's internal breakpoint ID (from `:break` response).
    pub maxima_id: Option<u32>,
    /// Message for unverified breakpoints (e.g. "line is not inside a function").
    pub message: Option<String>,
}

/// Body of the custom `maxima-output` DAP event.
///
/// This event carries Maxima I/O lines that are filtered out of the
/// Debug Console (sentinels, prompts, labels, etc.) so the companion
/// VS Code extension can display them in a dedicated output channel.
#[derive(Debug, Clone, Serialize)]
pub struct MaximaOutputEventBody {
    /// The stream this line came from: `"stdout"`, `"stderr"`, or `"stdin"`.
    pub category: String,
    /// The output line content.
    pub output: String,
}

/// A custom DAP event envelope for events not defined in the standard spec.
///
/// VS Code silently ignores unknown event types, but companion extensions
/// can intercept them via `onDidReceiveDebugSessionCustomEvent`.
#[derive(Debug, Clone, Serialize)]
pub struct CustomEvent<T: Serialize> {
    pub seq: i64,
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub event: &'static str,
    pub body: T,
}

/// Reference types for the `variables` request.
///
/// The DAP protocol uses integer `variablesReference` values. We track
/// what each reference points to so we can resolve `variables` requests.
#[derive(Debug, Clone)]
pub enum VariableRef {
    /// Local variables for a specific stack frame.
    FrameLocals { frame_index: u32 },
    /// An expandable compound value (list, matrix, etc.).
    Expandable {
        /// The Maxima expression to expand.
        expression: String,
    },
}
