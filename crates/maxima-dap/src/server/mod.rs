//! DAP server implementation for Maxima.
//!
//! Handles the DAP request/response lifecycle, managing a Maxima process
//! with debug mode enabled. Translates between DAP concepts (file:line
//! breakpoints, stack frames) and Maxima's text-based debugger.

mod breakpoints;
mod communication;
mod execution;
mod inspection;
mod lifecycle;
mod protocol;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::types;

use emmy_dap_types::base_message::{BaseMessage, Sendable};
use emmy_dap_types::events::{
    BreakpointEventBody, Event, OutputEventBody, StoppedEventBody,
};
use emmy_dap_types::requests::{Command, InitializeArguments, Request};
use emmy_dap_types::responses::{
    ContinueResponse, EvaluateResponse, Response, ResponseBody, ScopesResponse,
    SetBreakpointsResponse, StackTraceResponse, ThreadsResponse, VariablesResponse,
};
use emmy_dap_types::types::{
    Breakpoint, BreakpointEventReason, Capabilities, OutputEventCategory, Scope,
    ScopePresentationhint, Source, StackFrame, StoppedEventReason, Thread, Variable,
};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

use aximar_core::error::AppError;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::debugger::{self, CanonicalLocation, PromptKind};
use aximar_core::maxima::output::{OutputEvent, OutputSink};
use aximar_core::maxima::process::MaximaProcess;

use maxima_mac_parser::MacItem;

use crate::breakpoints::SourceIndex;
use crate::frames;
use crate::strategy::{BreakpointStrategy, StrategyContext};
use crate::strategy_enhanced::EnhancedStrategy;
use crate::strategy_legacy::LegacyStrategy;
use crate::transport::{DapTransport, TransportError};
use crate::types::{DebugState, MappedBreakpoint, MaximaLaunchArguments, VariableRef};

/// DAP output sink that collects output events for the server to emit.
struct DapOutputSink {
    events: std::sync::Mutex<Vec<OutputEvent>>,
}

impl DapOutputSink {
    fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn drain(&self) -> Vec<OutputEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }
}

impl OutputSink for DapOutputSink {
    fn emit(&self, event: OutputEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// The DAP server.
pub struct DapServer {
    transport: DapTransport,
    seq_counter: i64,
    state: DebugState,
    process: Option<MaximaProcess>,
    program_path: Option<PathBuf>,
    launch_args: Option<MaximaLaunchArguments>,
    source_index: SourceIndex,
    breakpoints: HashMap<PathBuf, Vec<MappedBreakpoint>>,
    next_breakpoint_id: i64,
    /// Cached backtrace frames from the last `stackTrace` request.
    cached_frames: Vec<StackFrame>,
    /// Cached raw backtrace frame data (args text) parallel to `cached_frames`.
    cached_frame_args: Vec<String>,
    /// Variable reference map for `variables` requests.
    var_refs: HashMap<i64, VariableRef>,
    next_var_ref: i64,
    output_sink: Arc<DapOutputSink>,
    /// Whether the client uses 1-based lines (DAP default).
    lines_start_at_1: bool,
    /// Whether the program file has been loaded into Maxima.
    /// Breakpoints can only be set in Maxima after the file is loaded
    /// (functions must exist for `:break func N` to work).
    file_loaded: bool,
    /// Temp file handle for the definitions file. Kept alive so the OS
    /// doesn't delete it while Maxima still references the functions.
    defs_temp_file: Option<tempfile::NamedTempFile>,
    /// Path to the temp file used for batchloading definitions.
    /// Maxima associates functions with this path; we remap it back to
    /// the original `program_path` in stack traces and output.
    defs_temp_path: Option<PathBuf>,
    /// When true, `flush_output` routes all output to the protocol
    /// channel instead of the Debug Console. Used during internal
    /// evaluations (e.g. querying block-local variable values).
    suppress_output: bool,
    /// Breakpoint strategy: Legacy (function+offset) or Enhanced (file:line).
    /// Set during `handle_launch` based on runtime detection.
    strategy: Option<Box<dyn BreakpointStrategy>>,
}

impl DapServer {
    pub fn new(transport: DapTransport) -> Self {
        let output_sink = Arc::new(DapOutputSink::new());
        Self {
            transport,
            seq_counter: 0,
            state: DebugState::Uninitialized,
            process: None,
            program_path: None,
            launch_args: None,
            source_index: SourceIndex::new(),
            breakpoints: HashMap::new(),
            next_breakpoint_id: 1,
            cached_frames: Vec::new(),
            cached_frame_args: Vec::new(),
            var_refs: HashMap::new(),
            next_var_ref: 1,
            output_sink,
            lines_start_at_1: true,
            file_loaded: false,
            defs_temp_file: None,
            defs_temp_path: None,
            suppress_output: false,
            strategy: None,
        }
    }

    /// Run the DAP server message loop.
    pub async fn run(&mut self) -> Result<(), TransportError> {
        loop {
            let msg = match self.transport.read_message().await? {
                Some(msg) => msg,
                None => {
                    tracing::info!("client disconnected");
                    break;
                }
            };

            let msg_type = msg
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            match msg_type.as_str() {
                "request" => {
                    if let Err(e) = self.handle_request(msg).await {
                        tracing::error!("error handling request: {}", e);
                    }
                }
                other => {
                    tracing::warn!("ignoring unknown message type: {}", other);
                }
            }
        }
        Ok(())
    }

    async fn handle_request(&mut self, raw: Value) -> Result<(), TransportError> {
        let seq = raw.get("seq").and_then(|v| v.as_i64()).unwrap_or(0);
        let command_str = raw
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let request: Request = match serde_json::from_value(raw) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("failed to parse request '{}': {}", command_str, e);
                self.send_error_response(seq, &format!("parse error: {}", e))
                    .await?;
                return Ok(());
            }
        };

        let result = match &request.command {
            Command::Initialize(args) => self.handle_initialize(&request, args).await,
            Command::Launch(args) => self.handle_launch(&request, args).await,
            Command::SetBreakpoints(args) => self.handle_set_breakpoints(&request, args).await,
            Command::ConfigurationDone => self.handle_configuration_done(&request).await,
            Command::Threads => self.handle_threads(&request).await,
            Command::Continue(args) => self.handle_continue(&request, args).await,
            Command::Next(args) => self.handle_next(&request, args).await,
            Command::StepIn(args) => self.handle_step_in(&request, args).await,
            Command::StackTrace(args) => self.handle_stack_trace(&request, args).await,
            Command::Scopes(args) => self.handle_scopes(&request, args).await,
            Command::Variables(args) => self.handle_variables(&request, args).await,
            Command::Evaluate(args) => self.handle_evaluate(&request, args).await,
            Command::Disconnect(_) => self.handle_disconnect(&request).await,
            _ => {
                // Unsupported command — send ack
                self.send_response(&request, None).await
            }
        };

        if let Err(e) = result {
            tracing::error!("handler error for {}: {}", command_str, e);
        }
        Ok(())
    }
}
