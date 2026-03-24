use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use aximar_core::catalog::docs::Docs;
use aximar_core::catalog::search::Catalog;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::labels::{rewrite_labels, LabelContext};
use aximar_core::maxima::output::OutputSink;
use aximar_core::maxima::unicode::{build_texput_init, unicode_to_maxima};
use aximar_core::maxima::process::MaximaProcess;
use aximar_core::maxima::protocol;
use aximar_core::maxima::types::SessionStatus;
use aximar_core::notebooks::{data as notebook_data, io as notebook_io, types as notebook_types};
use aximar_core::session::SessionManager;

use crate::capture::CaptureOutputSink;
use crate::commands::NotebookCommand;
use crate::log::ServerLog;
use crate::notebook::{CellOutput, CellStatus, CellType, Notebook};

#[derive(Clone)]
pub struct AximarMcpServer {
    #[allow(dead_code)]
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
    session: Arc<SessionManager>,
    catalog: Arc<Catalog>,
    docs: Arc<Docs>,
    notebook: Arc<Mutex<Notebook>>,
    output_sink: Arc<CaptureOutputSink>,
    /// Sink used when spawning the Maxima process.
    /// In headless mode this equals output_sink; in connected mode this is a
    /// MultiOutputSink that also feeds the Tauri frontend.
    process_sink: Arc<dyn OutputSink>,
    server_log: Arc<ServerLog>,
    backend: Backend,
    maxima_path: Option<String>,
    eval_timeout: u64,
    /// Optional callback invoked after any notebook mutation (used by connected mode
    /// to push state to the Tauri frontend).
    on_notebook_change: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl AximarMcpServer {
    pub fn new(
        session: Arc<SessionManager>,
        catalog: Arc<Catalog>,
        docs: Arc<Docs>,
        notebook: Arc<Mutex<Notebook>>,
        output_sink: Arc<CaptureOutputSink>,
        server_log: Arc<ServerLog>,
        backend: Backend,
        maxima_path: Option<String>,
        eval_timeout: u64,
    ) -> Self {
        let process_sink: Arc<dyn OutputSink> = output_sink.clone();
        let tool_router = Self::tool_router();
        AximarMcpServer {
            tool_router,
            session,
            catalog,
            docs,
            notebook,
            output_sink,
            process_sink,
            server_log,
            backend,
            maxima_path,
            eval_timeout,
            on_notebook_change: None,
        }
    }

    /// Create a server for connected mode (embedded in Tauri) with a custom
    /// process output sink and a notebook-change callback.
    pub fn new_connected(
        session: Arc<SessionManager>,
        catalog: Arc<Catalog>,
        docs: Arc<Docs>,
        notebook: Arc<Mutex<Notebook>>,
        output_sink: Arc<CaptureOutputSink>,
        process_sink: Arc<dyn OutputSink>,
        server_log: Arc<ServerLog>,
        backend: Backend,
        maxima_path: Option<String>,
        eval_timeout: u64,
        on_notebook_change: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let tool_router = Self::tool_router();
        AximarMcpServer {
            tool_router,
            session,
            catalog,
            docs,
            notebook,
            output_sink,
            process_sink,
            server_log,
            backend,
            maxima_path,
            eval_timeout,
            on_notebook_change: Some(on_notebook_change),
        }
    }

    /// Invoke the notebook-change callback if one is registered (connected mode).
    fn notify_notebook_change(&self) {
        if let Some(ref cb) = self.on_notebook_change {
            cb();
        }
    }

    /// Ensure Maxima session is started; returns Ok(()) if ready.
    async fn ensure_session(&self) -> Result<(), String> {
        let status = self.session.status();
        match status {
            SessionStatus::Ready | SessionStatus::Busy => Ok(()),
            SessionStatus::Stopped | SessionStatus::Error(_) => {
                self.session.begin_start().await;
                let sink: Arc<dyn OutputSink> = self.process_sink.clone();
                match MaximaProcess::spawn(
                    self.backend.clone(),
                    self.maxima_path.clone(),
                    sink,
                )
                .await
                {
                    Ok(process) => {
                        self.session.set_ready(process).await;
                        // Configure texput so Greek letters render correctly
                        // (e.g. theta → \theta instead of Maxima's default \vartheta)
                        let init = build_texput_init();
                        let mut guard = self.session.lock().await;
                        if let Ok(p) = guard.process_mut() {
                            let _ = protocol::evaluate(p, "__init__", &init, &self.catalog, self.eval_timeout).await;
                        }
                        drop(guard);
                        Ok(())
                    }
                    Err(e) => {
                        let msg = format!("{e}");
                        self.session.set_error(msg.clone()).await;
                        Err(msg)
                    }
                }
            }
            SessionStatus::Starting => {
                for _ in 0..50 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    match self.session.status() {
                        SessionStatus::Ready | SessionStatus::Busy => return Ok(()),
                        SessionStatus::Error(e) => return Err(e),
                        SessionStatus::Stopped => return Err("Session stopped".into()),
                        _ => continue,
                    }
                }
                Err("Timeout waiting for session to start".into())
            }
        }
    }
}

// ── Tool parameter types ──────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchFunctionsParams {
    /// Search query (matches function name and description)
    query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetFunctionDocsParams {
    /// Function name (case-insensitive)
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CompleteFunctionParams {
    /// Prefix to complete (e.g. "integ" → "integrate")
    prefix: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CellIdParams {
    /// Cell ID
    cell_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddCellParams {
    /// Cell type: "code" or "markdown" (default: "code")
    cell_type: Option<String>,
    /// Initial cell content
    input: Option<String>,
    /// Insert after this cell ID (appends to end if omitted)
    after_cell_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateCellParams {
    /// Cell ID to update
    cell_id: String,
    /// New cell content
    input: Option<String>,
    /// New cell type: "code" or "markdown"
    cell_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MoveCellParams {
    /// Cell ID to move
    cell_id: String,
    /// Direction: "up" or "down"
    direction: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct EvaluateExpressionParams {
    /// Maxima expression to evaluate
    expression: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct KillVariableParams {
    /// Variable name to kill
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetServerLogParams {
    /// Filter by stream: "stdout", "stderr", or "stdin"
    stream: Option<String>,
    /// Maximum number of entries to return (default: all)
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NotebookPathParams {
    /// File path for the notebook (.ipynb)
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LoadTemplateParams {
    /// Template ID (see list_templates)
    template_id: String,
}

// ── Tool result helpers ───────────────────────────────────────────────

/// Return a successful JSON-serialized result.
/// Using Result<String, String> because rmcp's IntoCallToolResult maps
/// Ok(String) → CallToolResult::success and Err(String) → CallToolResult::error.
fn success_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| format!("Serialization error: {e}"))
}

fn error_result(msg: impl Into<String>) -> Result<String, String> {
    Err(msg.into())
}

// ── Cell serialization for tool responses ─────────────────────────────

#[derive(Serialize)]
struct CellSummary {
    id: String,
    cell_type: CellType,
    input: String,
    status: CellStatus,
    has_output: bool,
    output_preview: Option<String>,
}

// ── Tool implementations ──────────────────────────────────────────────

#[tool_router]
impl AximarMcpServer {
    // ── Documentation tools ───────────────────────────────────────

    #[tool(description = "Search the Maxima function catalog by name or description. Returns matching functions with signatures and brief descriptions.")]
    async fn search_functions(
        &self,
        Parameters(params): Parameters<SearchFunctionsParams>,
    ) -> Result<String, String> {
        let results = self.catalog.search(&params.query);
        if results.is_empty() {
            return success_json(&serde_json::json!({
                "results": [],
                "message": format!("No functions matching '{}'", params.query)
            }));
        }
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.function.name,
                    "signatures": r.function.signatures,
                    "description": r.function.description,
                    "category": r.function.category,
                    "score": r.score,
                })
            })
            .collect();
        success_json(&serde_json::json!({ "results": items }))
    }

    #[tool(description = "Get full documentation for a Maxima function, including usage, examples, and related functions.")]
    async fn get_function_docs(
        &self,
        Parameters(params): Parameters<GetFunctionDocsParams>,
    ) -> Result<String, String> {
        if let Some(doc) = self.docs.get(&params.name) {
            Ok(doc.to_string())
        } else if let Some(func) = self.catalog.get(&params.name) {
            success_json(&serde_json::json!({
                "name": func.name,
                "signatures": func.signatures,
                "description": func.description,
                "category": func.category,
                "note": "Full documentation not available; showing catalog entry."
            }))
        } else {
            let similar = self.catalog.find_similar(&params.name, 3);
            if similar.is_empty() {
                error_result(format!("Function '{}' not found", params.name))
            } else {
                error_result(format!(
                    "Function '{}' not found. Did you mean: {}?",
                    params.name,
                    similar.join(", ")
                ))
            }
        }
    }

    #[tool(description = "Autocomplete a Maxima function name prefix. Returns matching function names with signatures.")]
    async fn complete_function(
        &self,
        Parameters(params): Parameters<CompleteFunctionParams>,
    ) -> Result<String, String> {
        let results = self.catalog.complete(&params.prefix);
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "signature": r.signature,
                    "description": r.description,
                    "insert_text": r.insert_text,
                })
            })
            .collect();
        success_json(&serde_json::json!({ "completions": items }))
    }

    // ── Cell management tools ─────────────────────────────────────

    #[tool(description = "List all cells in the notebook with their IDs, types, status, and content preview.")]
    async fn list_cells(&self) -> Result<String, String> {
        let nb = self.notebook.lock().await;
        let summaries: Vec<CellSummary> = nb
            .cells()
            .iter()
            .map(|c| CellSummary {
                id: c.id.clone(),
                cell_type: c.cell_type,
                input: c.input.clone(),
                status: c.status,
                has_output: c.output.is_some(),
                output_preview: c.output.as_ref().map(|o| {
                    let preview = &o.text_output;
                    if preview.len() > 200 {
                        format!("{}...", &preview[..200])
                    } else {
                        preview.clone()
                    }
                }),
            })
            .collect();
        success_json(&summaries)
    }

    #[tool(description = "Get full details of a specific cell, including its input, output, status, and raw Maxima I/O log.")]
    async fn get_cell(
        &self,
        Parameters(params): Parameters<CellIdParams>,
    ) -> Result<String, String> {
        let nb = self.notebook.lock().await;
        match nb.get_cell(&params.cell_id) {
            Some(cell) => success_json(cell),
            None => error_result(format!("Cell '{}' not found", params.cell_id)),
        }
    }

    #[tool(description = "Add a new cell to the notebook. Returns the new cell's ID.")]
    async fn add_cell(
        &self,
        Parameters(params): Parameters<AddCellParams>,
    ) -> Result<String, String> {
        let cell_type = match params.cell_type.as_deref() {
            Some("markdown") => CellType::Markdown,
            _ => CellType::Code,
        };
        let input = params.input.unwrap_or_default();
        let mut nb = self.notebook.lock().await;
        let effect = nb.apply(NotebookCommand::AddCell {
            cell_type,
            input,
            after_cell_id: params.after_cell_id,
        })?;
        let cell_id = effect.cell_id().unwrap_or("").to_string();
        drop(nb);
        self.notify_notebook_change();
        success_json(&serde_json::json!({ "cell_id": cell_id }))
    }

    #[tool(description = "Update a cell's content or type.")]
    async fn update_cell(
        &self,
        Parameters(params): Parameters<UpdateCellParams>,
    ) -> Result<String, String> {
        let mut nb = self.notebook.lock().await;
        // Apply input update if provided
        if let Some(input) = params.input {
            nb.apply(NotebookCommand::UpdateCellInput {
                cell_id: params.cell_id.clone(),
                input,
            })?;
        }
        // Apply cell type toggle if the requested type differs
        if let Some(ref type_str) = params.cell_type {
            let requested = match type_str.as_str() {
                "markdown" => CellType::Markdown,
                _ => CellType::Code,
            };
            if let Some(cell) = nb.get_cell(&params.cell_id) {
                if cell.cell_type != requested {
                    nb.apply(NotebookCommand::ToggleCellType {
                        cell_id: params.cell_id.clone(),
                    })?;
                }
            } else {
                return error_result(format!("Cell '{}' not found", params.cell_id));
            }
        }
        drop(nb);
        self.notify_notebook_change();
        success_json(&serde_json::json!({ "updated": true }))
    }

    #[tool(description = "Delete a cell from the notebook. Cannot delete the last remaining cell.")]
    async fn delete_cell(
        &self,
        Parameters(params): Parameters<CellIdParams>,
    ) -> Result<String, String> {
        let mut nb = self.notebook.lock().await;
        let effect = nb.apply(NotebookCommand::DeleteCell {
            cell_id: params.cell_id.clone(),
        })?;
        drop(nb);
        self.notify_notebook_change();
        match effect {
            crate::commands::CommandEffect::NoOp { reason } => {
                error_result(reason)
            }
            _ => success_json(&serde_json::json!({ "deleted": true })),
        }
    }

    #[tool(description = "Move a cell up or down in the notebook.")]
    async fn move_cell(
        &self,
        Parameters(params): Parameters<MoveCellParams>,
    ) -> Result<String, String> {
        let mut nb = self.notebook.lock().await;
        let effect = nb.apply(NotebookCommand::MoveCell {
            cell_id: params.cell_id.clone(),
            direction: params.direction.clone(),
        })?;
        drop(nb);
        self.notify_notebook_change();
        match effect {
            crate::commands::CommandEffect::NoOp { reason } => {
                error_result(reason)
            }
            _ => success_json(&serde_json::json!({ "moved": true })),
        }
    }

    // ── Execution tools ───────────────────────────────────────────

    #[tool(description = "Execute a notebook cell. Auto-starts the Maxima session if needed. Returns the evaluation result including text output, LaTeX, plots, and errors.")]
    async fn run_cell(
        &self,
        Parameters(params): Parameters<CellIdParams>,
    ) -> Result<String, String> {
        if let Err(e) = self.ensure_session().await {
            return error_result(format!("Failed to start session: {e}"));
        }

        let (input, cell_type) = {
            let nb = self.notebook.lock().await;
            let cell = match nb.get_cell(&params.cell_id) {
                Some(c) => c,
                None => return error_result(format!("Cell '{}' not found", params.cell_id)),
            };
            if cell.cell_type == CellType::Markdown {
                return success_json(&serde_json::json!({
                    "cell_id": params.cell_id,
                    "message": "Markdown cell — nothing to execute"
                }));
            }
            (cell.input.clone(), cell.cell_type)
        };

        if cell_type == CellType::Code {
            // Set status to Running
            {
                let mut nb = self.notebook.lock().await;
                let _ = nb.apply(NotebookCommand::SetCellStatus {
                    cell_id: params.cell_id.clone(),
                    status: CellStatus::Running,
                });
            }
            self.notify_notebook_change();

            // Clear previous capture
            self.output_sink.take_cell_output();

            // Rewrite labels for display numbering
            let exec_count = {
                let mut nb = self.notebook.lock().await;
                nb.next_execution_count()
            };
            let label_ctx = {
                let nb = self.notebook.lock().await;
                LabelContext {
                    label_map: nb.label_map().clone(),
                    previous_output_label: nb.previous_output_label(&params.cell_id),
                }
            };
            let translated = unicode_to_maxima(&input);
            let rewritten = rewrite_labels(&translated, &label_ctx);

            // Evaluate
            let mut guard = self.session.lock().await;
            let process = match guard.try_begin_eval() {
                Ok(p) => p,
                Err(e) => {
                    let mut nb = self.notebook.lock().await;
                    let _ = nb.apply(NotebookCommand::SetCellStatus {
                        cell_id: params.cell_id.clone(),
                        status: CellStatus::Error,
                    });
                    return error_result(format!("Session not ready: {e}"));
                }
            };

            let result = protocol::evaluate(
                process,
                &params.cell_id,
                &rewritten,
                &self.catalog,
                self.eval_timeout,
            )
            .await;

            guard.end_eval();
            drop(guard);

            // Capture raw output
            let raw_output = self.output_sink.take_cell_output();

            match result {
                Ok(eval_result) => {
                    // Record label mapping
                    if let Some(ref label) = eval_result.output_label {
                        let mut nb = self.notebook.lock().await;
                        nb.record_label(exec_count, label.clone());
                    }

                    let cell_output = CellOutput::from_eval_result(&eval_result, exec_count);
                    let mut nb = self.notebook.lock().await;
                    let _ = nb.apply(NotebookCommand::SetCellOutput {
                        cell_id: params.cell_id.clone(),
                        output: cell_output.clone(),
                        raw_output,
                    });
                    drop(nb);
                    self.notify_notebook_change();
                    success_json(&serde_json::json!({
                        "cell_id": params.cell_id,
                        "execution_count": exec_count,
                        "text_output": cell_output.text_output,
                        "latex": cell_output.latex,
                        "plot_svg": cell_output.plot_svg,
                        "error": cell_output.error,
                        "is_error": cell_output.is_error,
                        "duration_ms": cell_output.duration_ms,
                        "output_label": cell_output.output_label,
                    }))
                }
                Err(e) => {
                    let mut nb = self.notebook.lock().await;
                    let _ = nb.apply(NotebookCommand::SetCellStatus {
                        cell_id: params.cell_id.clone(),
                        status: CellStatus::Error,
                    });
                    // Store raw output even on error
                    if let Some(cell) = nb.get_cell_mut(&params.cell_id) {
                        cell.raw_output = raw_output;
                    }
                    error_result(format!("Evaluation failed: {e}"))
                }
            }
        } else {
            success_json(&serde_json::json!({
                "cell_id": params.cell_id,
                "message": "Non-code cell — nothing to execute"
            }))
        }
    }

    #[tool(description = "Execute all code cells in the notebook in order. Returns results for each cell.")]
    async fn run_all_cells(&self) -> Result<String, String> {
        if let Err(e) = self.ensure_session().await {
            return error_result(format!("Failed to start session: {e}"));
        }

        let cell_ids: Vec<String> = {
            let nb = self.notebook.lock().await;
            nb.cells()
                .iter()
                .filter(|c| c.cell_type == CellType::Code)
                .map(|c| c.id.clone())
                .collect()
        };

        let mut results = Vec::new();
        for cell_id in &cell_ids {
            let result = self
                .run_cell_impl(cell_id)
                .await;
            let is_error = result.is_err();
            results.push(serde_json::json!({
                "cell_id": cell_id,
                "success": !is_error,
            }));
            if is_error {
                break;
            }
        }

        success_json(&serde_json::json!({
            "cells_run": results.len(),
            "results": results,
        }))
    }

    #[tool(description = "Evaluate a Maxima expression without creating a notebook cell. Good for quick calculations. Auto-starts the session if needed.")]
    async fn evaluate_expression(
        &self,
        Parameters(params): Parameters<EvaluateExpressionParams>,
    ) -> Result<String, String> {
        if let Err(e) = self.ensure_session().await {
            return error_result(format!("Failed to start session: {e}"));
        }

        // Clear capture
        self.output_sink.take_cell_output();

        let mut guard = self.session.lock().await;
        let process = match guard.try_begin_eval() {
            Ok(p) => p,
            Err(e) => return error_result(format!("Session not ready: {e}")),
        };

        let translated = unicode_to_maxima(&params.expression);
        let result = protocol::evaluate(
            process,
            "__ephemeral__",
            &translated,
            &self.catalog,
            self.eval_timeout,
        )
        .await;

        guard.end_eval();

        match result {
            Ok(eval_result) => success_json(&serde_json::json!({
                "text_output": eval_result.text_output,
                "latex": eval_result.latex,
                "plot_svg": eval_result.plot_svg,
                "error": eval_result.error,
                "is_error": eval_result.is_error,
                "duration_ms": eval_result.duration_ms,
            })),
            Err(e) => error_result(format!("Evaluation failed: {e}")),
        }
    }

    // ── Session tools ─────────────────────────────────────────────

    #[tool(description = "Get the current Maxima session status: Starting, Ready, Busy, Stopped, or Error.")]
    async fn get_session_status(&self) -> Result<String, String> {
        let status = self.session.status();
        success_json(&serde_json::json!({
            "status": format!("{:?}", status),
        }))
    }

    #[tool(description = "Restart the Maxima session. Kills the current process and starts a new one. All session state (variables, definitions) is lost.")]
    async fn restart_session(&self) -> Result<String, String> {
        // Stop current session
        if let Err(e) = self.session.stop().await {
            tracing::warn!("Error stopping session: {e}");
        }

        // Start fresh
        match self.ensure_session().await {
            Ok(()) => success_json(&serde_json::json!({
                "restarted": true,
                "status": format!("{:?}", self.session.status()),
            })),
            Err(e) => error_result(format!("Failed to restart: {e}")),
        }
    }

    #[tool(description = "List all user-defined variables in the current Maxima session.")]
    async fn list_variables(&self) -> Result<String, String> {
        if let Err(e) = self.ensure_session().await {
            return error_result(format!("Failed to start session: {e}"));
        }

        let mut guard = self.session.lock().await;
        let process = match guard.try_begin_eval() {
            Ok(p) => p,
            Err(e) => return error_result(format!("Session not ready: {e}")),
        };

        let result = protocol::query_variables(process).await;
        guard.end_eval();

        match result {
            Ok(vars) => success_json(&serde_json::json!({ "variables": vars })),
            Err(e) => error_result(format!("Failed to query variables: {e}")),
        }
    }

    #[tool(description = "Remove a variable from the Maxima session (equivalent to `kill(name)` in Maxima).")]
    async fn kill_variable(
        &self,
        Parameters(params): Parameters<KillVariableParams>,
    ) -> Result<String, String> {
        if let Err(e) = self.ensure_session().await {
            return error_result(format!("Failed to start session: {e}"));
        }

        let mut guard = self.session.lock().await;
        let process = match guard.try_begin_eval() {
            Ok(p) => p,
            Err(e) => return error_result(format!("Session not ready: {e}")),
        };

        let result = protocol::kill_variable(process, &params.name).await;
        guard.end_eval();

        match result {
            Ok(()) => success_json(&serde_json::json!({ "killed": params.name })),
            Err(e) => error_result(format!("Failed to kill variable: {e}")),
        }
    }

    // ── Log tools ─────────────────────────────────────────────────

    #[tool(description = "Get the raw Maxima I/O log for a specific cell, showing stdin/stdout/stderr streams.")]
    async fn get_cell_output_log(
        &self,
        Parameters(params): Parameters<CellIdParams>,
    ) -> Result<String, String> {
        let nb = self.notebook.lock().await;
        match nb.get_cell(&params.cell_id) {
            Some(cell) => {
                let lines: Vec<serde_json::Value> = cell
                    .raw_output
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "stream": e.stream,
                            "line": e.line,
                            "timestamp": e.timestamp,
                        })
                    })
                    .collect();
                success_json(&serde_json::json!({ "log": lines }))
            }
            None => error_result(format!("Cell '{}' not found", params.cell_id)),
        }
    }

    #[tool(description = "Get the server-wide Maxima output log. Useful for debugging session issues.")]
    async fn get_server_log(
        &self,
        Parameters(params): Parameters<GetServerLogParams>,
    ) -> Result<String, String> {
        let entries = self
            .server_log
            .get(params.limit, params.stream.as_deref());
        let lines: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "stream": e.stream,
                    "line": e.line,
                    "timestamp": e.timestamp,
                })
            })
            .collect();
        success_json(&serde_json::json!({
            "count": lines.len(),
            "log": lines,
        }))
    }

    // ── Notebook I/O tools ────────────────────────────────────────

    #[tool(description = "Save the current notebook to a file in Jupyter .ipynb format.")]
    async fn save_notebook(
        &self,
        Parameters(params): Parameters<NotebookPathParams>,
    ) -> Result<String, String> {
        let nb = self.notebook.lock().await;
        let notebook = notebook_to_ipynb(&nb);
        drop(nb);

        match notebook_io::write_notebook(&params.path, &notebook) {
            Ok(()) => success_json(&serde_json::json!({
                "saved": true,
                "path": params.path,
            })),
            Err(e) => error_result(format!("Failed to save: {e}")),
        }
    }

    #[tool(description = "Open a Jupyter .ipynb notebook file and load it into the editor, replacing the current notebook state.")]
    async fn open_notebook(
        &self,
        Parameters(params): Parameters<NotebookPathParams>,
    ) -> Result<String, String> {
        match notebook_io::read_notebook(&params.path) {
            Ok(notebook) => {
                let cells = ipynb_to_cell_tuples(&notebook);
                let mut nb = self.notebook.lock().await;
                nb.apply(NotebookCommand::LoadCells { cells })?;
                let cell_count = nb.cells().len();
                drop(nb);
                self.notify_notebook_change();
                success_json(&serde_json::json!({
                    "opened": true,
                    "path": params.path,
                    "cell_count": cell_count,
                }))
            }
            Err(e) => error_result(format!("Failed to open: {e}")),
        }
    }

    #[tool(description = "List available notebook templates with their IDs, titles, and descriptions.")]
    async fn list_templates(&self) -> Result<String, String> {
        let templates = notebook_data::list_templates();
        let items: Vec<serde_json::Value> = templates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "title": t.title,
                    "description": t.description,
                    "cell_count": t.cell_count,
                })
            })
            .collect();
        success_json(&serde_json::json!({ "templates": items }))
    }

    #[tool(description = "Load a template into the notebook, replacing current content. Use list_templates to see available template IDs.")]
    async fn load_template(
        &self,
        Parameters(params): Parameters<LoadTemplateParams>,
    ) -> Result<String, String> {
        match notebook_data::get_template(&params.template_id) {
            Some(notebook) => {
                let cells = ipynb_to_cell_tuples(&notebook);
                let mut nb = self.notebook.lock().await;
                nb.apply(NotebookCommand::LoadCells { cells })?;
                let cell_count = nb.cells().len();
                drop(nb);
                self.notify_notebook_change();
                success_json(&serde_json::json!({
                    "loaded": true,
                    "template_id": params.template_id,
                    "cell_count": cell_count,
                }))
            }
            None => {
                let available = notebook_data::list_templates();
                let ids: Vec<&str> = available.iter().map(|t| t.id.as_str()).collect();
                error_result(format!(
                    "Template '{}' not found. Available: {}",
                    params.template_id,
                    ids.join(", ")
                ))
            }
        }
    }
}

// ── Non-tool helper methods ───────────────────────────────────────────

impl AximarMcpServer {
    /// Internal helper for run_all_cells to call run_cell logic without going through the tool system.
    async fn run_cell_impl(&self, cell_id: &str) -> Result<String, String> {
        self.run_cell(Parameters(CellIdParams {
            cell_id: cell_id.to_string(),
        }))
        .await
    }
}

// ── ServerHandler implementation ──────────────────────────────────────

#[tool_handler]
impl rmcp::handler::server::ServerHandler for AximarMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::default();
        capabilities.tools = Some(Default::default());

        ServerInfo::new(capabilities)
            .with_server_info(
                Implementation::new("aximar-mcp", env!("CARGO_PKG_VERSION"))
                    .with_title("Aximar MCP Server")
                    .with_description("MCP server for the Maxima computer algebra system"),
            )
            .with_instructions(
                "Aximar provides access to the Maxima computer algebra system. \
                 You can search function documentation, create and run notebook cells \
                 with Maxima expressions, inspect variables, and manage the session.\n\n\
                 Prefer Unicode Greek letters (α, β, γ, δ, θ, λ, μ, π, σ, φ, ω, etc.) \
                 everywhere — in Maxima code, markdown cells, comments, and when presenting \
                 results. Write π rather than %pi, ∞ rather than inf, etc. The Aximar \
                 protocol translates Unicode Greek to the corresponding Maxima symbols \
                 automatically, so % forms are not required.\n\n\
                 When building a notebook, run each code cell immediately after creating it \
                 before moving on to the next cell. This ensures earlier definitions are \
                 available to later cells and lets you catch errors early.",
            )
    }
}

// ── Notebook format conversion ────────────────────────────────────────

fn notebook_to_ipynb(nb: &Notebook) -> notebook_types::Notebook {
    let cells: Vec<notebook_types::NotebookCell> = nb
        .cells()
        .iter()
        .map(|cell| {
            let cell_type = match cell.cell_type {
                CellType::Code => notebook_types::CellType::Code,
                CellType::Markdown => notebook_types::CellType::Markdown,
            };
            let execution_count = cell
                .output
                .as_ref()
                .and_then(|o| o.execution_count)
                .map(|c| c as u64);

            let outputs = cell.output.as_ref().map(|o| {
                let mut out = serde_json::json!({
                    "output_type": "execute_result",
                    "text/plain": o.text_output,
                });
                if let Some(ref latex) = o.latex {
                    out["text/latex"] = serde_json::json!(latex);
                }
                vec![out]
            });

            notebook_types::NotebookCell {
                cell_type,
                source: notebook_types::CellSource::String(cell.input.clone()),
                metadata: serde_json::json!({}),
                execution_count,
                outputs,
            }
        })
        .collect();

    notebook_types::Notebook {
        nbformat: 4,
        nbformat_minor: 5,
        metadata: notebook_types::NotebookMetadata {
            kernelspec: notebook_types::KernelSpec {
                name: "maxima".into(),
                display_name: "Maxima".into(),
                language: Some("maxima".into()),
            },
            aximar: Some(notebook_types::AximarMetadata {
                template_id: None,
                title: None,
                description: None,
            }),
        },
        cells,
    }
}

/// Convert an ipynb Notebook into a list of (id, cell_type, input) tuples
/// suitable for the LoadCells command.
fn ipynb_to_cell_tuples(notebook: &notebook_types::Notebook) -> Vec<(String, CellType, String)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LOAD_COUNTER: AtomicU64 = AtomicU64::new(1);

    notebook
        .cells
        .iter()
        .filter_map(|cell| {
            let cell_type = match cell.cell_type {
                notebook_types::CellType::Code => CellType::Code,
                notebook_types::CellType::Markdown => CellType::Markdown,
                notebook_types::CellType::Raw => return None,
            };
            let input = match &cell.source {
                notebook_types::CellSource::String(s) => s.clone(),
                notebook_types::CellSource::Lines(lines) => lines.join(""),
            };
            let id = format!("load-{}", LOAD_COUNTER.fetch_add(1, Ordering::Relaxed));
            Some((id, cell_type, input))
        })
        .collect()
}
