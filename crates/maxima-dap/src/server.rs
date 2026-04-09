//! DAP server implementation for Maxima.
//!
//! Handles the DAP request/response lifecycle, managing a Maxima process
//! with debug mode enabled. Translates between DAP concepts (file:line
//! breakpoints, stack frames) and Maxima's text-based debugger.

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

/// Matches Maxima input/output labels like `(%i1)`, `(%o42)`, `(%t3)`.
static MAXIMA_LABEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(%[iot]\d+\)\s*$").unwrap());

/// Matches debugger source location lines: `/path/to/file.mac:12::`
static DEBUGGER_SOURCE_LOC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^.+:\d+::$").unwrap());

/// Matches backtrace frames: `#0: func(args) (file.mac line 7)`
static BACKTRACE_FRAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#\d+:\s+\w+\(").unwrap());

/// Matches breakpoint/step location: `(file.mac line 14, in function $ADD)`
static BREAKPOINT_LOC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(.*\bline\s+\d+.*in function\b").unwrap());

use aximar_core::error::AppError;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::debugger::{self, PromptKind};
use aximar_core::maxima::output::{OutputEvent, OutputSink};
use aximar_core::maxima::process::MaximaProcess;

use maxima_mac_parser::MacItem;

use crate::breakpoints::{self, BreakpointMapping, SourceIndex};
use crate::frames;
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

    // -----------------------------------------------------------------------
    // Request handlers
    // -----------------------------------------------------------------------

    async fn handle_initialize(
        &mut self,
        request: &Request,
        args: &InitializeArguments,
    ) -> Result<(), TransportError> {
        self.lines_start_at_1 = args.lines_start_at1.unwrap_or(true);

        let capabilities = Capabilities {
            supports_configuration_done_request: Some(true),
            supports_evaluate_for_hovers: Some(true),
            supports_function_breakpoints: Some(false),
            supports_conditional_breakpoints: Some(false),
            supports_step_back: Some(false),
            ..Default::default()
        };

        self.state = DebugState::Initialized;
        self.send_response(request, Some(ResponseBody::Initialize(capabilities)))
            .await?;

        // Send the initialized event
        self.send_event(Event::Initialized).await?;

        Ok(())
    }

    async fn handle_launch(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::LaunchRequestArguments,
    ) -> Result<(), TransportError> {
        // Parse our custom launch arguments from additional_data
        let launch_args: MaximaLaunchArguments = match &args.additional_data {
            Some(data) => match serde_json::from_value(data.clone()) {
                Ok(a) => a,
                Err(e) => {
                    return self
                        .send_error_response(
                            request.seq,
                            &format!("invalid launch arguments: {}", e),
                        )
                        .await;
                }
            },
            None => {
                return self
                    .send_error_response(request.seq, "missing launch arguments")
                    .await;
            }
        };

        let program_path = PathBuf::from(&launch_args.program);
        if !program_path.exists() {
            return self
                .send_error_response(
                    request.seq,
                    &format!("program not found: {}", launch_args.program),
                )
                .await;
        }

        // Parse the backend
        let backend = match launch_args.backend.as_str() {
            "local" => Backend::Local,
            other => {
                return self
                    .send_error_response(
                        request.seq,
                        &format!(
                            "unsupported backend: {} (only 'local' supported for debugging)",
                            other
                        ),
                    )
                    .await;
            }
        };

        // Spawn Maxima process
        let custom_path = launch_args.maxima_path.clone();
        let process =
            match MaximaProcess::spawn(backend, custom_path, self.output_sink.clone()).await {
                Ok(p) => p,
                Err(e) => {
                    return self
                        .send_error_response(
                            request.seq,
                            &format!("failed to start Maxima: {}", e),
                        )
                        .await;
                }
            };

        self.process = Some(process);
        self.program_path = Some(program_path.clone());
        self.launch_args = Some(launch_args.clone());

        // Enable debug mode
        if let Err(e) = self.send_maxima("debugmode(true)$\n").await {
            return self
                .send_error_response(
                    request.seq,
                    &format!("failed to enable debug mode: {}", e),
                )
                .await;
        }

        // Detect backend (SBCL required for full debugging)
        if let Err(e) = self.check_lisp_backend().await {
            self.send_output_event(&format!("Warning: {}\n", e), OutputEventCategory::Console)
                .await?;
        }

        // Index the source file for breakpoint mapping (but don't load
        // definitions yet — we defer to configurationDone so that
        // breakpoints can be set before top-level code runs).
        if let Err(e) = self.source_index.index_file(&program_path) {
            self.send_output_event(
                &format!(
                    "Warning: could not parse {} for breakpoint mapping: {}\n",
                    program_path.display(),
                    e
                ),
                OutputEventCategory::Console,
            )
            .await?;
        }

        self.state = DebugState::Running;

        // Send the launch response
        self.send_response(request, Some(ResponseBody::Launch))
            .await?;

        // Flush any Maxima output as DAP output events
        self.flush_output().await?;

        Ok(())
    }

    async fn handle_set_breakpoints(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::SetBreakpointsArguments,
    ) -> Result<(), TransportError> {
        let source_path = args
            .source
            .path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_default();

        // Clear any existing breakpoints for this file in Maxima
        if let Some(existing) = self.breakpoints.get(&source_path) {
            let ids_to_delete: Vec<u32> = existing
                .iter()
                .filter_map(|bp| bp.maxima_id)
                .collect();
            let is_stopped = matches!(self.state, DebugState::Stopped { .. });
            for maxima_id in ids_to_delete {
                let cmd = format!(":delbreak {}", maxima_id);
                if is_stopped {
                    let _ = self.send_debugger_command_raw(&cmd).await;
                } else {
                    let _ = self.send_maxima(&format!("{}\n", cmd)).await;
                }
            }
        }

        let mut dap_breakpoints = Vec::new();
        let mut mapped_breakpoints = Vec::new();

        let source_breakpoints = args.breakpoints.as_deref().unwrap_or(&[]);

        // Index the file if not already done
        if self.source_index.get(&source_path).is_none() {
            let _ = self.source_index.index_file(&source_path);
        }

        for src_bp in source_breakpoints {
            let line = src_bp.line;
            let bp_id = self.next_breakpoint_id;
            self.next_breakpoint_id += 1;

            let mac_file = self.source_index.get(&source_path);
            let mapping = mac_file.map(|f| breakpoints::map_line_to_breakpoint(f, line as u64));
            tracing::debug!(
                "setBreakpoints: line {} -> {:?}",
                line,
                mapping
            );

            match mapping {
                Some(BreakpointMapping::Mapped {
                    function_name,
                    offset,
                }) => {
                    // If the file is loaded, set the breakpoint in Maxima now.
                    // Otherwise store it as pending — it will be set after
                    // definitions are loaded in configurationDone.
                    let (verified, maxima_id) = if self.file_loaded {
                        self.set_maxima_breakpoint(&function_name, offset).await
                    } else {
                        (false, None)
                    };

                    let pending = !self.file_loaded;
                    let message = if pending {
                        Some("Pending — will be set when file is loaded".to_string())
                    } else if !verified {
                        Some(format!("Could not set breakpoint in {}", function_name))
                    } else {
                        None
                    };

                    dap_breakpoints.push(Breakpoint {
                        id: Some(bp_id),
                        verified,
                        source: Some(Source {
                            path: Some(source_path.to_string_lossy().to_string()),
                            ..Default::default()
                        }),
                        line: Some(line),
                        message: message.clone(),
                        ..Default::default()
                    });

                    mapped_breakpoints.push(MappedBreakpoint {
                        dap_id: bp_id,
                        source_path: source_path.clone(),
                        line,
                        function: Some(function_name),
                        offset: Some(offset),
                        verified,
                        maxima_id,
                        message,
                    });
                }
                _ => {
                    let message = match &mapping {
                        Some(BreakpointMapping::NotInFunction { message }) => message.clone(),
                        _ => "Could not parse source file".to_string(),
                    };

                    dap_breakpoints.push(Breakpoint {
                        id: Some(bp_id),
                        verified: false,
                        source: Some(Source {
                            path: Some(source_path.to_string_lossy().to_string()),
                            ..Default::default()
                        }),
                        line: Some(line),
                        message: Some(message.clone()),
                        ..Default::default()
                    });

                    mapped_breakpoints.push(MappedBreakpoint {
                        dap_id: bp_id,
                        source_path: source_path.clone(),
                        line,
                        function: None,
                        offset: None,
                        verified: false,
                        maxima_id: None,
                        message: Some(message),
                    });
                }
            }
        }

        self.breakpoints.insert(source_path, mapped_breakpoints);

        self.send_response(
            request,
            Some(ResponseBody::SetBreakpoints(SetBreakpointsResponse {
                breakpoints: dap_breakpoints,
            })),
        )
        .await
    }

    async fn handle_configuration_done(
        &mut self,
        request: &Request,
    ) -> Result<(), TransportError> {
        self.send_response(request, Some(ResponseBody::ConfigurationDone))
            .await?;

        // Load only function/macro definitions from the file (deferred
        // from launch so breakpoints can be set before top-level code runs).
        if let Err(e) = self.load_program_file().await {
            self.send_output_event(
                &format!("Error loading program: {}\n", e),
                OutputEventCategory::Stderr,
            )
            .await?;
            self.state = DebugState::Terminated;
            self.send_event(Event::Terminated(None)).await?;
            return Ok(());
        }

        // Set any pending breakpoints now that functions are defined.
        self.set_pending_breakpoints().await?;

        // Either evaluate the user's expression or run the file's
        // top-level code.
        let evaluate_expr = self
            .launch_args
            .as_ref()
            .and_then(|a| a.evaluate.clone())
            .filter(|e| !e.trim().is_empty());

        let expr = match evaluate_expr {
            Some(expr) => expr,
            None => {
                // No explicit evaluate expression — extract and execute
                // the top-level (non-definition) code from the file.
                // Definitions were already loaded by load_program_file().
                let program_path = self.program_path.clone().unwrap_or_default();
                match self.extract_file_top_level(&program_path) {
                    Some(code) => code,
                    None => {
                        // No top-level code to execute — file only has
                        // definitions. Terminate cleanly.
                        self.send_output_event(
                            "File contains only function definitions. \
                             Add an \"evaluate\" expression to your launch config \
                             to call a function and trigger breakpoints.\n",
                            OutputEventCategory::Console,
                        )
                        .await?;
                        self.state = DebugState::Terminated;
                        self.send_event(Event::Terminated(None)).await?;
                        return Ok(());
                    }
                }
            }
        };

        tracing::debug!("configurationDone: evaluating {:?}", expr);
        match self.send_maxima_and_wait(&expr).await {
            Ok(PromptKind::Debugger { level }) => {
                tracing::debug!("configurationDone: hit breakpoint at level {}", level);
                self.state = DebugState::Stopped { level };
                self.send_stopped_event(StoppedEventReason::Breakpoint)
                    .await?;
            }
            Ok(PromptKind::Normal) => {
                tracing::debug!("configurationDone: expression completed without breakpoint");
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await?;
            }
            Err(e) => {
                self.send_output_event(
                    &format!("Error evaluating expression: {}\n", e),
                    OutputEventCategory::Stderr,
                )
                .await?;
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await?;
            }
        }
        self.flush_output().await?;

        Ok(())
    }

    /// Load only function/macro definitions into Maxima.
    ///
    /// Extracts definitions from the source, writes them to a temp file,
    /// and `batchload`s that. We must use `batchload` (not raw stdin)
    /// because Maxima needs the file association for `:break` line info.
    ///
    /// The temp file path is stored so that `frames.rs` can remap
    /// backtrace source references back to the original program file.
    async fn load_program_file(&mut self) -> Result<(), AppError> {
        let program_path = self
            .program_path
            .clone()
            .ok_or(AppError::ProcessNotRunning)?;

        let source = std::fs::read_to_string(&program_path)?;
        let mac_file = self
            .source_index
            .get(&program_path)
            .ok_or(AppError::CommunicationError("file not indexed".to_string()))?;

        let definitions = breakpoints::extract_definitions(&source, mac_file);
        if definitions.trim().is_empty() {
            tracing::debug!("load_program_file: no definitions to load");
            self.file_loaded = true;
            return Ok(());
        }

        // Write definitions to a named temp file and batchload it.
        // The temp file must have a .mac extension for Maxima to accept it
        // and must persist until the debug session ends (NamedTempFile is
        // deleted on drop, so we store it in the struct).
        let temp_file = tempfile::Builder::new()
            .prefix(".maxima-dap-")
            .suffix(".mac")
            .tempfile()?;
        let temp_path = temp_file.path().to_path_buf();
        std::fs::write(&temp_path, &definitions)?;
        self.defs_temp_path = Some(temp_path.clone());
        // Keep the handle alive so the file isn't deleted.
        self.defs_temp_file = Some(temp_file);

        let load_cmd = format!(
            "batchload(\"{}\")$\n",
            temp_path.to_string_lossy().replace('\\', "/")
        );
        tracing::debug!(
            "load_program_file: batchloading definitions from {}",
            temp_path.display()
        );
        let response_lines = self.send_maxima(&load_cmd).await?;

        // Check if batchload triggered an error that dropped us into
        // the Maxima debugger (e.g. redefining a built-in operator).
        let entered_debugger = response_lines
            .iter()
            .any(|l| debugger::detect_debugger_prompt(l).is_some());
        if entered_debugger {
            // Escape the error debugger so subsequent commands work.
            let _ = self.send_maxima(":top\n").await;
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

        self.file_loaded = true;
        Ok(())
    }

    /// Set all pending (unverified) breakpoints in Maxima and notify the
    /// client of their updated status via breakpoint-changed events.
    async fn set_pending_breakpoints(&mut self) -> Result<(), TransportError> {
        // Suppress output while setting breakpoints — Maxima may print
        // "No line info" errors for functions from other files.
        self.suppress_output = true;

        // Collect all pending breakpoints across all files.
        let all_paths: Vec<PathBuf> = self.breakpoints.keys().cloned().collect();
        for path in all_paths {
            let bps = match self.breakpoints.get(&path) {
                Some(bps) => bps.clone(),
                None => continue,
            };

            let mut updated = Vec::new();
            for mut bp in bps {
                if bp.verified || bp.function.is_none() {
                    // Already set or unmappable — keep as-is.
                    updated.push(bp);
                    continue;
                }

                let function = bp.function.as_ref().unwrap().clone();
                let offset = bp.offset.unwrap_or(0);
                let (verified, maxima_id) =
                    self.set_maxima_breakpoint(&function, offset).await;

                bp.verified = verified;
                bp.maxima_id = maxima_id;
                bp.message = if verified {
                    None
                } else {
                    Some(format!("Could not set breakpoint in {}", function))
                };

                // Notify client of the breakpoint status.
                self.send_event(Event::Breakpoint(BreakpointEventBody {
                    reason: BreakpointEventReason::Changed,
                    breakpoint: Breakpoint {
                        id: Some(bp.dap_id),
                        verified: bp.verified,
                        line: Some(bp.line),
                        message: bp.message.clone(),
                        source: Some(Source {
                            path: Some(path.to_string_lossy().to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                }))
                .await?;

                updated.push(bp);
            }

            self.breakpoints.insert(path, updated);
        }

        let _ = self.flush_output().await;
        self.suppress_output = false;
        Ok(())
    }

    async fn handle_threads(&mut self, request: &Request) -> Result<(), TransportError> {
        // Maxima is single-threaded
        self.send_response(
            request,
            Some(ResponseBody::Threads(ThreadsResponse {
                threads: vec![Thread {
                    id: 1,
                    name: "Maxima".to_string(),
                }],
            })),
        )
        .await
    }

    async fn handle_continue(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::ContinueArguments,
    ) -> Result<(), TransportError> {
        self.send_response(
            request,
            Some(ResponseBody::Continue(ContinueResponse {
                all_threads_continued: Some(true),
            })),
        )
        .await?;

        match self.send_debugger_command(":resume").await {
            Ok(PromptKind::Debugger { level }) => {
                self.state = DebugState::Stopped { level };
                self.flush_output().await?;
                self.send_stopped_event(StoppedEventReason::Breakpoint)
                    .await?;
            }
            Ok(PromptKind::Normal) => {
                self.state = DebugState::Terminated;
                self.flush_output().await?;
                self.send_event(Event::Terminated(None)).await?;
            }
            Err(e) => {
                self.send_output_event(
                    &format!("Error: {}\n", e),
                    OutputEventCategory::Stderr,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_next(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::NextArguments,
    ) -> Result<(), TransportError> {
        self.send_response(request, Some(ResponseBody::Next))
            .await?;

        tracing::debug!("handle_next: sending :next (state={:?})", self.state);
        match self.send_debugger_command(":next").await {
            Ok(PromptKind::Debugger { level }) => {
                tracing::debug!("handle_next: stopped at debugger level {}", level);
                self.state = DebugState::Stopped { level };
                self.flush_output().await?;
                self.send_stopped_event(StoppedEventReason::Step).await?;
            }
            Ok(PromptKind::Normal) => {
                tracing::debug!("handle_next: expression completed (sentinel reached)");
                self.state = DebugState::Terminated;
                self.flush_output().await?;
                self.send_event(Event::Terminated(None)).await?;
            }
            Err(e) => {
                tracing::error!("handle_next: error: {}", e);
                self.send_output_event(
                    &format!("Error: {}\n", e),
                    OutputEventCategory::Stderr,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_step_in(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::StepInArguments,
    ) -> Result<(), TransportError> {
        self.send_response(request, Some(ResponseBody::StepIn))
            .await?;

        match self.send_debugger_command(":step").await {
            Ok(PromptKind::Debugger { level }) => {
                self.state = DebugState::Stopped { level };
                self.flush_output().await?;
                self.send_stopped_event(StoppedEventReason::Step).await?;
            }
            Ok(PromptKind::Normal) => {
                self.state = DebugState::Terminated;
                self.flush_output().await?;
                self.send_event(Event::Terminated(None)).await?;
            }
            Err(e) => {
                self.send_output_event(
                    &format!("Error: {}\n", e),
                    OutputEventCategory::Stderr,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_stack_trace(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::StackTraceArguments,
    ) -> Result<(), TransportError> {
        let (bt_lines, bt_frame_args) = match self.get_backtrace().await {
            Ok(result) => result,
            Err(e) => {
                return self
                    .send_error_response(
                        request.seq,
                        &format!("failed to get backtrace: {}", e),
                    )
                    .await;
            }
        };

        let program_path = self.program_path.clone().unwrap_or_default();
        let path_remaps = self.build_path_remaps();
        let stack_frames =
            frames::parse_backtrace(&bt_lines, &self.source_index, &program_path, &path_remaps);

        // Cache for scopes/variables requests
        self.cached_frame_args = bt_frame_args;
        self.cached_frames = stack_frames.clone();

        // Reset variable references
        self.var_refs.clear();
        self.next_var_ref = 1;

        let total_frames = stack_frames.len() as i64;

        self.send_response(
            request,
            Some(ResponseBody::StackTrace(StackTraceResponse {
                stack_frames,
                total_frames: Some(total_frames),
            })),
        )
        .await
    }

    async fn handle_scopes(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::ScopesArguments,
    ) -> Result<(), TransportError> {
        let frame_id = args.frame_id as u32;

        // Create a variable reference for this frame's locals
        let var_ref = self.next_var_ref;
        self.next_var_ref += 1;
        self.var_refs
            .insert(var_ref, VariableRef::FrameLocals { frame_index: frame_id });

        let scopes = vec![Scope {
            name: "Locals".to_string(),
            presentation_hint: Some(ScopePresentationhint::Locals),
            variables_reference: var_ref,
            named_variables: None,
            indexed_variables: None,
            expensive: false,
            source: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        }];

        self.send_response(
            request,
            Some(ResponseBody::Scopes(ScopesResponse { scopes })),
        )
        .await
    }

    async fn handle_variables(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::VariablesArguments,
    ) -> Result<(), TransportError> {
        let var_ref = args.variables_reference;

        let variables = match self.var_refs.get(&var_ref).cloned() {
            Some(VariableRef::FrameLocals { frame_index }) => {
                let args_text = self
                    .cached_frame_args
                    .get(frame_index as usize)
                    .cloned()
                    .unwrap_or_default();
                let mut vars = frames::frame_args_to_variables(&args_text);

                // Also include block-local variables from the function definition
                let locals = self.get_block_locals_for_frame(frame_index).await;
                vars.extend(locals);

                vars
            }
            Some(VariableRef::Expandable { ref expression }) => {
                let expr = expression.clone();
                self.expand_variable(&expr).await
            }
            None => Vec::new(),
        };

        self.send_response(
            request,
            Some(ResponseBody::Variables(VariablesResponse { variables })),
        )
        .await
    }

    /// Look up block-local variables for a stack frame and evaluate their
    /// current values at the debugger prompt.
    async fn get_block_locals_for_frame(&mut self, frame_index: u32) -> Vec<Variable> {
        // Get the function name from the cached stack frame
        let function_name = match self.cached_frames.get(frame_index as usize) {
            Some(frame) => frame.name.clone(),
            None => return Vec::new(),
        };

        // Find block_locals from the parsed source
        let block_locals = self.find_block_locals(&function_name);
        if block_locals.is_empty() {
            return Vec::new();
        }

        // Suppress output while evaluating locals — these are internal
        // queries and should not appear in the Debug Console.
        self.suppress_output = true;

        let mut variables = Vec::new();
        for local_name in &block_locals {
            let value = match self.evaluate_at_debugger(local_name).await {
                Ok(v) => v.trim().to_string(),
                Err(_) => "?".to_string(),
            };
            variables.push(Variable {
                name: local_name.clone(),
                value,
                type_field: None,
                presentation_hint: None,
                evaluate_name: Some(local_name.clone()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                memory_reference: None,
            });
        }

        // Drain any remaining output from the evaluations while
        // still suppressed, so it goes to the protocol channel.
        let _ = self.flush_output().await;
        self.suppress_output = false;

        variables
    }

    /// Find block_locals for a function name from the source index.
    fn find_block_locals(&self, function_name: &str) -> Vec<String> {
        let program_path = match &self.program_path {
            Some(p) => p,
            None => return Vec::new(),
        };
        let mac_file = match self.source_index.get(program_path) {
            Some(f) => f,
            None => return Vec::new(),
        };
        for item in &mac_file.items {
            match item {
                MacItem::FunctionDef(f) | MacItem::MacroDef(f) if f.name == function_name => {
                    return f.block_locals.clone();
                }
                _ => continue,
            }
        }
        Vec::new()
    }

    async fn handle_evaluate(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::EvaluateArguments,
    ) -> Result<(), TransportError> {
        let expression = args.expression.clone();

        // Only allow evaluation when stopped at a debugger prompt
        if !matches!(self.state, DebugState::Stopped { .. }) {
            return self
                .send_error_response(
                    request.seq,
                    "can only evaluate expressions when stopped at a breakpoint",
                )
                .await;
        }

        let result = match self.evaluate_at_debugger(&expression).await {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        };

        self.send_response(
            request,
            Some(ResponseBody::Evaluate(EvaluateResponse {
                result,
                type_field: None,
                presentation_hint: None,
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                memory_reference: None,
            })),
        )
        .await
    }

    async fn handle_disconnect(
        &mut self,
        request: &Request,
    ) -> Result<(), TransportError> {
        // Clean up the Maxima process
        if let Some(ref mut process) = self.process {
            let _ = process.write_stdin(":top\n").await;
            let _ = process.kill().await;
        }
        self.process = None;
        self.state = DebugState::Terminated;

        self.send_response(request, Some(ResponseBody::Disconnect))
            .await?;

        self.send_event(Event::Terminated(None)).await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Maxima communication helpers
    // -----------------------------------------------------------------------

    /// Send a command to Maxima's stdin and read until sentinel.
    async fn send_maxima(&mut self, cmd: &str) -> Result<Vec<String>, AppError> {
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;
        let sentinel = "__MAXIMA_DAP_DONE__";
        process.write_stdin(cmd).await?;
        process
            .write_stdin(&format!("print(\"{}\")$\n", sentinel))
            .await?;
        process.read_until_sentinel(sentinel).await
    }

    /// Evaluate an expression and wait for either completion or a debugger
    /// breakpoint.
    ///
    /// The sentinel is embedded *inside* a wrapping `block()` so it only
    /// fires when the expression completes — it is never left as a separate
    /// line in stdin that could be consumed at a `dbm:>` prompt.
    async fn send_maxima_and_wait(&mut self, expr: &str) -> Result<PromptKind, AppError> {
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;
        let sentinel = "__MAXIMA_DAP_DONE__";
        let trimmed = expr
            .trim()
            .trim_end_matches(|c: char| c == ';' || c == '$');
        let wrapped = format!(
            "block([__dap_r__], __dap_r__: ({}), print(\"{}\"), __dap_r__)$\n",
            trimmed, sentinel
        );
        tracing::debug!("send_maxima_and_wait: sending {:?}", wrapped.trim());
        process.write_stdin(&wrapped).await?;
        let (lines, prompt_kind) = process.read_dap_response(Some(sentinel)).await?;
        tracing::debug!("send_maxima_and_wait: got {:?}, lines: {:?}", prompt_kind, lines);
        Ok(prompt_kind)
    }

    /// Send a debugger command (like `:step`, `:resume`) and wait for the
    /// next debugger prompt or normal prompt (sentinel from outer eval).
    ///
    /// Uses chunk-based reading to detect debugger prompts that lack a
    /// trailing newline.  The sentinel is NOT sent here — it was already
    /// queued by `send_maxima_and_wait` as part of the original expression
    /// evaluation.  When the expression eventually completes (after one or
    /// more `:resume` / `:step` commands), the sentinel fires and this
    /// returns `PromptKind::Normal`.
    async fn send_debugger_command(&mut self, cmd: &str) -> Result<PromptKind, AppError> {
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;
        let sentinel = "__MAXIMA_DAP_DONE__";
        tracing::debug!("send_debugger_command: sending {:?}", cmd);
        process.write_stdin(&format!("{}\n", cmd)).await?;
        let (lines, prompt_kind) = process.read_dap_response(Some(sentinel)).await?;
        tracing::debug!(
            "send_debugger_command: got {:?}, lines: {:?}",
            prompt_kind,
            lines
        );
        Ok(prompt_kind)
    }

    /// Send a raw debugger command and read until the debugger prompt.
    /// Used for commands where we know we'll stay in the debugger (like `:bt`).
    ///
    /// Uses chunk-based reading to detect the debugger prompt without a
    /// trailing newline.
    async fn send_debugger_command_raw(
        &mut self,
        cmd: &str,
    ) -> Result<(Vec<String>, u32), AppError> {
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;
        process.write_stdin(&format!("{}\n", cmd)).await?;
        let (lines, prompt_kind) = process.read_dap_response(None).await?;
        match prompt_kind {
            PromptKind::Debugger { level } => Ok((lines, level)),
            PromptKind::Normal => Err(AppError::CommunicationError(
                "unexpected normal prompt in debugger context".into(),
            )),
        }
    }

    /// Get the backtrace from Maxima.
    /// Returns (raw lines, args text per frame).
    async fn get_backtrace(&mut self) -> Result<(Vec<String>, Vec<String>), AppError> {
        let (lines, _level) = self.send_debugger_command_raw(":bt").await?;

        let mut args_per_frame = Vec::new();
        for line in &lines {
            if let Some(frame) = debugger::parse_backtrace_frame(line) {
                args_per_frame.push(frame.args);
            }
        }

        Ok((lines, args_per_frame))
    }

    /// Set a breakpoint in Maxima using `:break func N`.
    /// Returns (verified, maxima_id).
    ///
    /// State-aware: uses sentinel-based reading at a normal prompt and
    /// debugger-prompt reading when already stopped in the debugger.
    async fn set_maxima_breakpoint(
        &mut self,
        function_name: &str,
        offset: u32,
    ) -> (bool, Option<u32>) {
        let cmd = format!(":break {} {}", function_name, offset);
        tracing::debug!("set_maxima_breakpoint: {}", cmd);
        let lines = if matches!(self.state, DebugState::Stopped { .. }) {
            // At debugger prompt — response ends with another debugger prompt
            match self.send_debugger_command_raw(&cmd).await {
                Ok((lines, _)) => lines,
                Err(_) => return (false, None),
            }
        } else {
            // At normal prompt — use sentinel-based reading
            match self.send_maxima(&format!("{}\n", cmd)).await {
                Ok(lines) => lines,
                Err(_) => return (false, None),
            }
        };

        tracing::debug!("set_maxima_breakpoint: response lines: {:?}", lines);
        let bkpt_re = Regex::new(r"Bkpt\s+(\d+)").unwrap();
        for line in &lines {
            if let Some(caps) = bkpt_re.captures(line) {
                if let Some(id) = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()) {
                    tracing::debug!("set_maxima_breakpoint: confirmed, maxima_id={}", id);
                    return (true, Some(id));
                }
            }
        }
        // No confirmation — Maxima rejected the breakpoint (e.g. "No line info",
        // function not defined in the current session).
        tracing::warn!("set_maxima_breakpoint: no Bkpt confirmation in response");
        (false, None)
    }

    /// Evaluate an expression at the debugger prompt.
    async fn evaluate_at_debugger(&mut self, expression: &str) -> Result<String, AppError> {
        let cmd = format!("{};\n", expression.trim_end_matches(|c| c == ';' || c == '$'));
        let (lines, _level) = self.send_debugger_command_raw(&cmd).await?;

        let result: Vec<&str> = lines
            .iter()
            .map(|l| l.as_str())
            .filter(|l| debugger::detect_debugger_prompt(l).is_none())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(result.join("\n"))
    }

    /// Expand a compound variable by evaluating sub-parts.
    async fn expand_variable(&mut self, expression: &str) -> Vec<Variable> {
        let length_expr = format!("length({})", expression);
        let length = match self.evaluate_at_debugger(&length_expr).await {
            Ok(result) => result.trim().parse::<usize>().unwrap_or(0),
            Err(_) => return Vec::new(),
        };

        let mut variables = Vec::new();
        for i in 1..=length.min(100) {
            let part_expr = format!("part({}, {})", expression, i);
            let value = match self.evaluate_at_debugger(&part_expr).await {
                Ok(v) => v.trim().to_string(),
                Err(_) => "?".to_string(),
            };
            variables.push(Variable {
                name: format!("[{}]", i),
                value,
                type_field: None,
                presentation_hint: None,
                evaluate_name: Some(part_expr),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                memory_reference: None,
            });
        }
        variables
    }

    /// Extract top-level (non-definition) code from a source file.
    ///
    /// Returns `None` if there is no meaningful top-level code to execute
    /// (e.g. the file contains only function definitions and comments).
    fn extract_file_top_level(&self, program_path: &Path) -> Option<String> {
        let source = std::fs::read_to_string(program_path).ok()?;
        let mac_file = self.source_index.get(program_path)?;
        let top_level = breakpoints::extract_top_level_code(&source, mac_file);

        // Check if there's anything meaningful (not just whitespace/comments)
        let trimmed = top_level.trim();
        if trimmed.is_empty() {
            return None;
        }

        // The top-level code contains multiple statements separated by
        // $ or ; terminators. Since send_maxima_and_wait wraps the
        // expression inside block([__dap_r__], __dap_r__: (EXPR), ...),
        // we must convert statement terminators ($ and ;) into commas
        // so they become block-level statement separators rather than
        // breaking out of the wrapping block.
        //
        // Uses the mac parser's lexer for correct handling of comments,
        // strings, and nested expressions.
        let converted = maxima_mac_parser::replace_terminators(trimmed);
        let final_code = converted
            .trim()
            .trim_end_matches(',')
            .trim()
            .to_string();

        if final_code.is_empty() {
            return None;
        }

        Some(final_code)
    }

    /// Build a path remapping table from temp file → original source.
    ///
    /// Used by `frames::parse_backtrace` to translate Maxima's temp file
    /// references back to the user's original `.mac` file.
    fn build_path_remaps(&self) -> HashMap<PathBuf, PathBuf> {
        let mut remaps = HashMap::new();
        if let (Some(temp), Some(original)) = (&self.defs_temp_path, &self.program_path) {
            remaps.insert(temp.clone(), original.clone());
            // Also map the canonical form in case Maxima resolves symlinks.
            if let Ok(canonical) = temp.canonicalize() {
                remaps.insert(canonical, original.clone());
            }
        }
        remaps
    }

    /// Check the Lisp backend and warn if it's not SBCL.
    async fn check_lisp_backend(&mut self) -> Result<(), String> {
        match self.send_maxima(":lisp (lisp-implementation-type)\n").await {
            Ok(lines) => {
                let output = lines.join(" ");
                if !output.contains("SBCL") {
                    return Err(format!(
                        "Maxima is running on a non-SBCL Lisp backend. \
                         Debugging features like :bt and :frame may not work correctly. \
                         Backend detected: {}",
                        output.trim()
                    ));
                }
                Ok(())
            }
            Err(e) => Err(format!("Could not detect Lisp backend: {}", e)),
        }
    }

    // -----------------------------------------------------------------------
    // Message sending helpers
    // -----------------------------------------------------------------------

    fn next_seq(&mut self) -> i64 {
        self.seq_counter += 1;
        self.seq_counter
    }

    async fn send_response(
        &mut self,
        request: &Request,
        body: Option<ResponseBody>,
    ) -> Result<(), TransportError> {
        let seq = self.next_seq();
        let msg = BaseMessage {
            seq,
            message: Sendable::Response(Response {
                request_seq: request.seq,
                success: true,
                message: None,
                body,
                error: None,
            }),
        };
        self.transport.write_message(&msg).await
    }

    async fn send_error_response(
        &mut self,
        request_seq: i64,
        message: &str,
    ) -> Result<(), TransportError> {
        let seq = self.next_seq();
        let msg = BaseMessage {
            seq,
            message: Sendable::Response(Response {
                request_seq,
                success: false,
                message: Some(emmy_dap_types::responses::ResponseMessage::Error(
                    message.to_string(),
                )),
                body: None,
                error: None,
            }),
        };
        self.transport.write_message(&msg).await
    }

    async fn send_event(&mut self, event: Event) -> Result<(), TransportError> {
        let seq = self.next_seq();
        let msg = BaseMessage {
            seq,
            message: Sendable::Event(event),
        };
        self.transport.write_message(&msg).await
    }

    /// Send a custom DAP event not defined in the standard spec.
    ///
    /// VS Code silently ignores unknown event types, but companion
    /// extensions can intercept them via `onDidReceiveDebugSessionCustomEvent`.
    async fn send_custom_event<T: serde::Serialize>(
        &mut self,
        event_name: &'static str,
        body: T,
    ) -> Result<(), TransportError> {
        let msg = types::CustomEvent {
            seq: self.next_seq(),
            message_type: "event",
            event: event_name,
            body,
        };
        self.transport.write_message(&msg).await
    }

    async fn send_stopped_event(
        &mut self,
        reason: StoppedEventReason,
    ) -> Result<(), TransportError> {
        self.send_event(Event::Stopped(StoppedEventBody {
            reason,
            description: None,
            thread_id: Some(1),
            preserve_focus_hint: None,
            text: None,
            all_threads_stopped: Some(true),
            hit_breakpoint_ids: None,
        }))
        .await
    }

    async fn send_output_event(
        &mut self,
        text: &str,
        category: OutputEventCategory,
    ) -> Result<(), TransportError> {
        self.send_event(Event::Output(OutputEventBody {
            category: Some(category),
            output: text.to_string(),
            group: None,
            variables_reference: None,
            source: None,
            line: None,
            column: None,
            data: None,
        }))
        .await
    }

    /// Flush buffered Maxima output as DAP output events, filtering out
    /// internal protocol noise (sentinels, prompts, labels, debugger
    /// commands) so the Debug Console shows only user-visible output.
    ///
    /// Filtered lines are still sent as custom `maxima-output` events
    /// so the companion VS Code extension can display them in a
    /// dedicated "Maxima Protocol" output channel.
    async fn flush_output(&mut self) -> Result<(), TransportError> {
        let events = self.output_sink.drain();
        for event in events {
            if self.suppress_output || Self::is_noise(&event) {
                // Send as custom event for the extension's protocol output channel.
                self.send_custom_event(
                    "maxima-output",
                    types::MaximaOutputEventBody {
                        category: event.stream.clone(),
                        output: event.line.clone(),
                    },
                )
                .await?;
                continue;
            }
            let category = match event.stream.as_str() {
                "stderr" => OutputEventCategory::Stderr,
                _ => OutputEventCategory::Stdout,
            };
            self.send_output_event(&format!("{}\n", event.line), category)
                .await?;
        }
        Ok(())
    }

    /// Returns true if the output line is internal protocol noise that
    /// should not be shown to the user in the Debug Console.
    fn is_noise(event: &OutputEvent) -> bool {
        let line = event.line.trim();

        // Never show stdin echoes (commands sent to Maxima).
        if event.stream == "stdin" {
            return true;
        }

        // Empty lines.
        if line.is_empty() {
            return true;
        }

        // Any sentinel strings (DAP and aximar-core).
        if line.contains("__MAXIMA_DAP_DONE__")
            || line.contains("__dap_r__")
            || line.contains("__AXIMAR_")
        {
            return true;
        }

        // Maxima startup messages: "Loading /path/to/maxima-init.mac"
        if line.starts_with("Loading ") {
            return true;
        }

        // Maxima input/output labels: (%i1), (%o1), (%i42), etc.
        if MAXIMA_LABEL_RE.is_match(line) {
            return true;
        }

        // Debugger prompts: (dbm:1)
        if debugger::detect_debugger_prompt(line).is_some() {
            return true;
        }

        // Debugger source location lines: /path/to/file.mac:12::
        if DEBUGGER_SOURCE_LOC_RE.is_match(line) {
            return true;
        }

        // Backtrace frames: #0: add(a=3,b=4) (file.mac line 12)
        // These come from automatic :bt calls; VS Code shows the
        // stack trace in its own panel.
        if BACKTRACE_FRAME_RE.is_match(line) {
            return true;
        }

        // Breakpoint/step location: (file.mac line 14, in function $ADD)
        if BREAKPOINT_LOC_RE.is_match(line) {
            return true;
        }

        // Breakpoint messages: set confirmations ("Bkpt 1 for add at line 0")
        // and hit notifications ("Bkpt 0: (file.mac line 3, in function $add)").
        // VS Code shows breakpoint status via its own UI.
        if line.starts_with("Bkpt ") {
            return true;
        }

        // Bare words from batchload/debugmode/backend detection.
        if matches!(line, "done" | "true" | "false" | "SBCL" | "GCL" | "CLISP" | "ECL") {
            return true;
        }

        false
    }
}

