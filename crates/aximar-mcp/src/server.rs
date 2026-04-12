use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router};
use tokio::sync::Mutex;

use aximar_core::commands::{CommandEffect, NotebookCommand};
use aximar_core::safety;
use aximar_core::catalog::docs::Docs;
use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::maxima::unicode::unicode_to_maxima;
use aximar_core::maxima::protocol;
use aximar_core::notebooks::{data as notebook_data, io as notebook_io};
use aximar_core::registry::{NotebookContextRef, NotebookRegistry};
use aximar_core::session_ops::{self, SessionStatusCallback};

use crate::capture::CaptureOutputSink;
use crate::convert::{ipynb_to_cell_tuples, notebook_to_ipynb};
use crate::notebook::CellType;
use crate::params::*;

/// Factory function that builds a process output sink for a given notebook.
/// Args: (notebook_id, capture_sink) → process_sink.
///
/// In standalone mode, the factory returns the capture sink directly.
/// In connected mode, the factory wraps it in a MultiOutputSink that also
/// feeds the Tauri frontend.
pub type ProcessSinkFactory =
    Arc<dyn Fn(&str, &Arc<CaptureOutputSink>) -> Arc<dyn OutputSink> + Send + Sync>;

// ── Shared server core ────────────────────────────────────────────────

/// Shared state and logic used by both `AximarMcpServer` (notebook mode)
/// and `SimpleMcpServer` (simple mode).
#[derive(Clone)]
pub struct ServerCore {
    pub(crate) registry: Arc<Mutex<NotebookRegistry>>,
    pub(crate) catalog: Arc<Catalog>,
    pub(crate) docs: Arc<Docs>,
    pub(crate) packages: Arc<PackageCatalog>,
    pub(crate) backend: Backend,
    pub(crate) maxima_path: Option<String>,
    pub(crate) eval_timeout: u64,
    pub(crate) process_sink_factory: ProcessSinkFactory,
    pub(crate) on_notebook_change: Option<Arc<dyn Fn(&str, CommandEffect) + Send + Sync>>,
    pub(crate) on_notebook_lifecycle: Option<Arc<dyn Fn(&str, &str) + Send + Sync>>,
    pub(crate) on_session_status: Option<SessionStatusCallback>,
    pub(crate) allow_dangerous: bool,
}

impl ServerCore {
    /// Create a core for standalone mode (headless).
    pub fn new(
        registry: Arc<Mutex<NotebookRegistry>>,
        catalog: Arc<Catalog>,
        docs: Arc<Docs>,
        packages: Arc<PackageCatalog>,
        backend: Backend,
        maxima_path: Option<String>,
        eval_timeout: u64,
        allow_dangerous: bool,
    ) -> Self {
        let process_sink_factory: ProcessSinkFactory =
            Arc::new(|_id, capture| capture.clone() as Arc<dyn OutputSink>);
        Self {
            registry,
            catalog,
            docs,
            packages,
            backend,
            maxima_path,
            eval_timeout,
            process_sink_factory,
            on_notebook_change: None,
            on_notebook_lifecycle: None,
            on_session_status: None,
            allow_dangerous,
        }
    }

    /// Create a core for connected mode (GUI) with custom callbacks.
    pub fn new_connected(
        registry: Arc<Mutex<NotebookRegistry>>,
        catalog: Arc<Catalog>,
        docs: Arc<Docs>,
        packages: Arc<PackageCatalog>,
        backend: Backend,
        maxima_path: Option<String>,
        eval_timeout: u64,
        process_sink_factory: ProcessSinkFactory,
        on_notebook_change: Arc<dyn Fn(&str, CommandEffect) + Send + Sync>,
        on_notebook_lifecycle: Arc<dyn Fn(&str, &str) + Send + Sync>,
        on_session_status: SessionStatusCallback,
    ) -> Self {
        Self {
            registry,
            catalog,
            docs,
            packages,
            backend,
            maxima_path,
            eval_timeout,
            process_sink_factory,
            on_notebook_change: Some(on_notebook_change),
            on_notebook_lifecycle: Some(on_notebook_lifecycle),
            on_session_status: Some(on_session_status),
            allow_dangerous: false, // connected mode: GUI handles approval
        }
    }

    // ── Helpers ───────────────────────────────────────────────────

    /// Resolve a notebook context from an optional ID (defaults to active).
    pub(crate) async fn resolve_context(
        &self,
        notebook_id: Option<&str>,
    ) -> Result<NotebookContextRef, String> {
        let reg = self.registry.lock().await;
        reg.resolve(notebook_id)
    }

    /// Invoke the notebook-change callback if one is registered (connected mode).
    pub(crate) fn notify_notebook_change(&self, notebook_id: &str, effect: CommandEffect) {
        if let Some(ref cb) = self.on_notebook_change {
            cb(notebook_id, effect);
        }
    }

    /// Invoke the lifecycle callback if one is registered (connected mode).
    pub(crate) fn notify_lifecycle(&self, notebook_id: &str, event_type: &str) {
        if let Some(ref cb) = self.on_notebook_lifecycle {
            cb(notebook_id, event_type);
        }
    }

    /// Ensure Maxima session is started for the given notebook context.
    pub(crate) async fn ensure_session(&self, ctx: &NotebookContextRef) -> Result<(), String> {
        let factory = self.process_sink_factory.clone();
        session_ops::ensure_session(
            ctx,
            self.backend.clone(),
            self.maxima_path.clone(),
            move |ctx| (factory)(&ctx.id, &ctx.capture_sink),
            &self.catalog,
            self.eval_timeout,
            self.on_session_status.as_ref(),
        )
        .await
        .map_err(|e| e.to_string())
    }

    // ── Shared tool implementations ──────────────────────────────

    pub(crate) async fn do_search_functions(&self, query: &str) -> Result<String, String> {
        let results = self.catalog.search(query);
        if results.is_empty() {
            return success_json(&serde_json::json!({
                "results": [],
                "message": format!("No functions matching '{query}'")
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

    pub(crate) async fn do_get_function_docs(&self, name: &str) -> Result<String, String> {
        if let Some(doc) = self.docs.get(name) {
            Ok(doc.to_string())
        } else if let Some(func) = self.catalog.get(name) {
            success_json(&serde_json::json!({
                "name": func.name,
                "signatures": func.signatures,
                "description": func.description,
                "category": func.category,
                "note": "Full documentation not available; showing catalog entry."
            }))
        } else {
            let similar = self.catalog.find_similar(name, 3);
            if similar.is_empty() {
                error_result(format!("Function '{name}' not found"))
            } else {
                error_result(format!(
                    "Function '{name}' not found. Did you mean: {}?",
                    similar.join(", ")
                ))
            }
        }
    }

    pub(crate) async fn do_list_deprecated(&self) -> Result<String, String> {
        let results = self.catalog.find_deprecated();
        success_json(&serde_json::json!({
            "count": results.len(),
            "deprecated": results,
        }))
    }

    pub(crate) async fn do_complete_function(&self, prefix: &str) -> Result<String, String> {
        let mut results = self.catalog.complete(prefix);

        // Also include package functions (deduped)
        let pkg_results = self.packages.complete_functions(prefix);
        let existing: std::collections::HashSet<String> =
            results.iter().map(|r| r.name.to_lowercase()).collect();
        for r in pkg_results {
            if !existing.contains(&r.name.to_lowercase()) {
                results.push(r);
            }
        }

        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let mut obj = serde_json::json!({
                    "name": r.name,
                    "signature": r.signature,
                    "description": r.description,
                    "insert_text": r.insert_text,
                });
                if let Some(pkg) = &r.package {
                    obj["package"] = serde_json::json!(pkg);
                }
                obj
            })
            .collect();
        success_json(&serde_json::json!({ "completions": items }))
    }

    pub(crate) async fn do_search_packages(&self, query: &str) -> Result<String, String> {
        let results = self.packages.search(query);
        if results.is_empty() {
            return success_json(&serde_json::json!({
                "results": [],
                "message": format!("No packages matching '{query}'")
            }));
        }
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.package.name,
                    "description": r.package.description,
                    "functions": r.package.functions,
                    "score": r.score,
                })
            })
            .collect();
        success_json(&serde_json::json!({ "results": items }))
    }

    pub(crate) async fn do_list_packages(&self) -> Result<String, String> {
        let all = self.packages.all();
        let items: Vec<serde_json::Value> = all
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "description": p.description,
                    "function_count": p.functions.len(),
                })
            })
            .collect();
        success_json(&serde_json::json!({
            "count": items.len(),
            "packages": items,
        }))
    }

    pub(crate) async fn do_get_package(&self, name: &str) -> Result<String, String> {
        match self.packages.get(name) {
            Some(pkg) => success_json(&serde_json::json!({
                "name": pkg.name,
                "description": pkg.description,
                "functions": pkg.functions,
                "load_command": format!("load(\"{}\")$", pkg.name),
            })),
            None => error_result(format!("Package '{name}' not found")),
        }
    }

    pub(crate) async fn do_evaluate_expression(
        &self,
        expression: &str,
        notebook_id: Option<&str>,
    ) -> Result<String, String> {
        // Safety: evaluate_expression has no cell → no approval path → always block dangerous calls
        if !self.allow_dangerous {
            let dangerous = safety::detect_dangerous_calls(expression, Some(&self.packages));
            if !dangerous.is_empty() {
                let names: Vec<&str> = dangerous.iter().map(|d| d.function_name.as_str()).collect();
                return error_result(format!(
                    "Dangerous function(s) blocked: {}. Use a notebook cell for approval, or --allow-dangerous in headless mode.",
                    names.join(", ")
                ));
            }
        }

        let ctx = self.resolve_context(notebook_id).await?;
        if let Err(e) = self.ensure_session(&ctx).await {
            return error_result(format!("Failed to start session: {e}"));
        }

        // Clear capture
        ctx.capture_sink.take_cell_output();

        let mut guard = ctx.session.lock().await;
        let process = match guard.try_begin_eval() {
            Ok(p) => p,
            Err(e) => return error_result(format!("Session not ready: {e}")),
        };

        let translated = unicode_to_maxima(expression);
        let result = protocol::evaluate_with_packages(
            process,
            "__ephemeral__",
            &translated,
            &self.catalog,
            &self.packages,
            self.eval_timeout,
        )
        .await;

        guard.end_eval();

        match result {
            Ok(eval_result) => success_json(&serde_json::json!({
                "text_output": eval_result.text_output,
                "latex": eval_result.latex,
                "plot_svg": eval_result.plot_svg,
                "plot_data": eval_result.plot_data,
                "error": eval_result.error,
                "is_error": eval_result.is_error,
                "duration_ms": eval_result.duration_ms,
                "output_label": eval_result.output_label,
            })),
            Err(e) => error_result(format!("Evaluation failed: {e}")),
        }
    }

    pub(crate) async fn do_get_session_status(
        &self,
        notebook_id: Option<&str>,
    ) -> Result<String, String> {
        let ctx = self.resolve_context(notebook_id).await?;
        let status = ctx.session.status();
        success_json(&serde_json::json!({
            "status": format!("{:?}", status),
        }))
    }

    pub(crate) async fn do_restart_session(
        &self,
        notebook_id: Option<&str>,
    ) -> Result<String, String> {
        let ctx = self.resolve_context(notebook_id).await?;
        // Stop current session
        if let Err(e) = ctx.session.stop().await {
            tracing::warn!("Error stopping session: {e}");
        }

        // Start fresh
        match self.ensure_session(&ctx).await {
            Ok(()) => success_json(&serde_json::json!({
                "restarted": true,
                "status": format!("{:?}", ctx.session.status()),
            })),
            Err(e) => error_result(format!("Failed to restart: {e}")),
        }
    }

    pub(crate) async fn do_list_variables(
        &self,
        notebook_id: Option<&str>,
    ) -> Result<String, String> {
        let ctx = self.resolve_context(notebook_id).await?;
        if let Err(e) = self.ensure_session(&ctx).await {
            return error_result(format!("Failed to start session: {e}"));
        }

        let mut guard = ctx.session.lock().await;
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

    pub(crate) async fn do_kill_variable(
        &self,
        name: &str,
        notebook_id: Option<&str>,
    ) -> Result<String, String> {
        let ctx = self.resolve_context(notebook_id).await?;
        if let Err(e) = self.ensure_session(&ctx).await {
            return error_result(format!("Failed to start session: {e}"));
        }

        let mut guard = ctx.session.lock().await;
        let process = match guard.try_begin_eval() {
            Ok(p) => p,
            Err(e) => return error_result(format!("Session not ready: {e}")),
        };

        let result = protocol::kill_variable(process, name).await;
        guard.end_eval();

        match result {
            Ok(()) => success_json(&serde_json::json!({ "killed": name })),
            Err(e) => error_result(format!("Failed to kill variable: {e}")),
        }
    }

    pub(crate) async fn do_get_server_log(
        &self,
        notebook_id: Option<&str>,
        stream: Option<&str>,
        limit: Option<usize>,
    ) -> Result<String, String> {
        let ctx = self.resolve_context(notebook_id).await?;
        let entries = ctx.server_log.get(limit, stream);
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

    pub(crate) async fn do_create_session(&self, path: Option<&str>) -> Result<String, String> {
        let mut reg = self.registry.lock().await;
        let id = reg.create();
        if let Some(p) = path {
            let _ = reg.set_path(&id, Some(std::path::PathBuf::from(p)));
        }
        drop(reg);
        self.notify_lifecycle(&id, "created");
        success_json(&serde_json::json!({ "session_id": id }))
    }

    pub(crate) async fn do_close_session(&self, session_id: &str) -> Result<String, String> {
        // Validate the session exists
        {
            let reg = self.registry.lock().await;
            if let Err(e) = reg.get(session_id) {
                return error_result(e);
            }
        }

        // In connected mode, emit close_requested and let the frontend mediate
        if self.on_notebook_lifecycle.is_some() {
            self.notify_lifecycle(session_id, "close_requested");
            return success_json(&serde_json::json!({
                "status": "pending_confirmation",
                "session_id": session_id,
            }));
        }

        // Standalone mode: close directly
        let session = {
            let reg = self.registry.lock().await;
            match reg.get(session_id) {
                Ok(ctx) => ctx.session.clone(),
                Err(e) => return error_result(e),
            }
        };
        let _ = session.stop().await;
        let mut reg = self.registry.lock().await;
        match reg.close(session_id) {
            Ok(_) => {
                drop(reg);
                self.notify_lifecycle(session_id, "closed");
                success_json(&serde_json::json!({
                    "closed": true,
                    "session_id": session_id,
                }))
            }
            Err(e) => error_result(e),
        }
    }

    pub(crate) async fn do_list_sessions(&self) -> Result<String, String> {
        let reg = self.registry.lock().await;
        let notebooks = reg.list();
        success_json(&serde_json::json!({
            "active_session_id": reg.active_id(),
            "sessions": notebooks,
        }))
    }
}

// ── AximarMcpServer (notebook mode) ───────────────────────────────────

#[derive(Clone)]
pub struct AximarMcpServer {
    #[allow(dead_code)]
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
    pub(crate) core: ServerCore,
}

impl AximarMcpServer {
    /// Create a server for standalone mode (headless, stdio transport).
    pub fn new(
        registry: Arc<Mutex<NotebookRegistry>>,
        catalog: Arc<Catalog>,
        docs: Arc<Docs>,
        packages: Arc<PackageCatalog>,
        backend: Backend,
        maxima_path: Option<String>,
        eval_timeout: u64,
        allow_dangerous: bool,
    ) -> Self {
        let core = ServerCore::new(
            registry,
            catalog,
            docs,
            packages,
            backend,
            maxima_path,
            eval_timeout,
            allow_dangerous,
        );
        Self::from_core(core)
    }

    /// Create a server for connected mode (embedded in Tauri) with a custom
    /// process sink factory and callbacks for GUI synchronization.
    pub fn new_connected(
        registry: Arc<Mutex<NotebookRegistry>>,
        catalog: Arc<Catalog>,
        docs: Arc<Docs>,
        packages: Arc<PackageCatalog>,
        backend: Backend,
        maxima_path: Option<String>,
        eval_timeout: u64,
        process_sink_factory: ProcessSinkFactory,
        on_notebook_change: Arc<dyn Fn(&str, CommandEffect) + Send + Sync>,
        on_notebook_lifecycle: Arc<dyn Fn(&str, &str) + Send + Sync>,
        on_session_status: SessionStatusCallback,
    ) -> Self {
        let core = ServerCore::new_connected(
            registry,
            catalog,
            docs,
            packages,
            backend,
            maxima_path,
            eval_timeout,
            process_sink_factory,
            on_notebook_change,
            on_notebook_lifecycle,
            on_session_status,
        );
        Self::from_core(core)
    }

    /// Build from an existing `ServerCore`.
    pub fn from_core(core: ServerCore) -> Self {
        AximarMcpServer {
            tool_router: Self::tool_router(),
            core,
        }
    }
}

// ── Tool implementations (notebook mode — all tools) ──────────────────

#[tool_router]
impl AximarMcpServer {
    // ── Documentation tools ───────────────────────────────────────

    #[tool(description = "Search the Maxima function catalog by name or description. Returns matching functions with signatures and brief descriptions. Searches across 2500+ built-in and package functions. Supports partial name matching (e.g. \"integ\" finds integrate) and description keywords (e.g. \"matrix inverse\").")]
    async fn search_functions(
        &self,
        Parameters(params): Parameters<SearchFunctionsParams>,
    ) -> Result<String, String> {
        self.core.do_search_functions(&params.query).await
    }

    #[tool(description = "Get full documentation for a Maxima function, including usage, examples, and related functions. Falls back to a catalog summary if full docs are unavailable. Suggests similar function names if the exact name is not found.")]
    async fn get_function_docs(
        &self,
        Parameters(params): Parameters<GetFunctionDocsParams>,
    ) -> Result<String, String> {
        self.core.do_get_function_docs(&params.name).await
    }

    #[tool(description = "List Maxima functions that are deprecated, obsolete, or superseded. Returns names, descriptions, and suggested replacements where available. Consider calling this at the start of a session to avoid using obsolete functions in your notebook.")]
    async fn list_deprecated(&self) -> Result<String, String> {
        self.core.do_list_deprecated().await
    }

    #[tool(description = "Autocomplete a Maxima function name prefix. Returns matching function names with signatures. Includes both built-in and package functions alongside each other.")]
    async fn complete_function(
        &self,
        Parameters(params): Parameters<CompleteFunctionParams>,
    ) -> Result<String, String> {
        self.core.do_complete_function(&params.prefix).await
    }

    // ── Package tools ─────────────────────────────────────────────

    #[tool(description = "Search available Maxima packages by name or description. Returns packages with their load paths and function lists. Load a package in a code cell with load(\"name\")$ before using its functions.")]
    async fn search_packages(
        &self,
        Parameters(params): Parameters<SearchPackagesParams>,
    ) -> Result<String, String> {
        self.core.do_search_packages(&params.query).await
    }

    #[tool(description = "List all available Maxima packages that can be loaded with load(\"name\")$. Use get_package to see what functions a specific package provides.")]
    async fn list_packages(&self) -> Result<String, String> {
        self.core.do_list_packages().await
    }

    #[tool(description = "Get details of a specific Maxima package, including description and list of functions it provides. Load a package in a code cell with load(\"name\")$ before using its functions.")]
    async fn get_package(
        &self,
        Parameters(params): Parameters<GetPackageParams>,
    ) -> Result<String, String> {
        self.core.do_get_package(&params.name).await
    }

    // ── Notebook lifecycle tools ──────────────────────────────────

    #[tool(description = "List all open notebooks with their IDs, titles, and active status.")]
    async fn list_notebooks(&self) -> Result<String, String> {
        let reg = self.core.registry.lock().await;
        let notebooks = reg.list();
        success_json(&serde_json::json!({
            "active_notebook_id": reg.active_id(),
            "notebooks": notebooks,
        }))
    }

    #[tool(description = "Create a new notebook with its own independent Maxima session. Returns the new notebook's ID. Variables and definitions in one notebook are isolated from other notebooks.")]
    async fn create_notebook(&self) -> Result<String, String> {
        let mut reg = self.core.registry.lock().await;
        let id = reg.create();
        drop(reg);
        self.core.notify_lifecycle(&id, "created");
        success_json(&serde_json::json!({ "notebook_id": id }))
    }

    #[tool(description = "Close a notebook and stop its Maxima session. Cannot close the last notebook. In the GUI, the user will be prompted to confirm if there are unsaved changes.")]
    async fn close_notebook(
        &self,
        Parameters(params): Parameters<CloseNotebookParams>,
    ) -> Result<String, String> {
        // Validate the notebook exists before doing anything
        {
            let reg = self.core.registry.lock().await;
            if let Err(e) = reg.get(&params.notebook_id) {
                return error_result(e);
            }
        }

        // In connected mode, emit close_requested and let the frontend mediate
        if self.core.on_notebook_lifecycle.is_some() {
            self.core.notify_lifecycle(&params.notebook_id, "close_requested");
            return success_json(&serde_json::json!({
                "status": "pending_confirmation",
                "notebook_id": params.notebook_id,
            }));
        }

        // Standalone mode: close directly
        let session = {
            let reg = self.core.registry.lock().await;
            match reg.get(&params.notebook_id) {
                Ok(ctx) => ctx.session.clone(),
                Err(e) => return error_result(e),
            }
        };
        let _ = session.stop().await;
        let mut reg = self.core.registry.lock().await;
        match reg.close(&params.notebook_id) {
            Ok(_) => {
                drop(reg);
                self.core.notify_lifecycle(&params.notebook_id, "closed");
                success_json(&serde_json::json!({
                    "closed": true,
                    "notebook_id": params.notebook_id,
                }))
            }
            Err(e) => error_result(e),
        }
    }

    #[tool(description = "Switch the active notebook. Most tools default to the active notebook when notebook_id is omitted.")]
    async fn switch_notebook(
        &self,
        Parameters(params): Parameters<SwitchNotebookParams>,
    ) -> Result<String, String> {
        let mut reg = self.core.registry.lock().await;
        match reg.set_active(&params.notebook_id) {
            Ok(()) => success_json(&serde_json::json!({
                "active_notebook_id": params.notebook_id,
            })),
            Err(e) => error_result(e),
        }
    }

    // ── Cell management tools ─────────────────────────────────────

    #[tool(description = "List all cells in the notebook with their IDs, types, status, and content preview. Shows a truncated preview of each cell's input and output. Use get_cell for full content.")]
    async fn list_cells(
        &self,
        Parameters(params): Parameters<NotebookIdParam>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        let nb = ctx.notebook.lock().await;
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

    #[tool(description = "Get full details of a specific cell, including its input, output, status, and raw Maxima I/O log. Output includes text_output (print statements, warnings, provisos), LaTeX-rendered result, plot SVGs, and any errors.")]
    async fn get_cell(
        &self,
        Parameters(params): Parameters<CellIdParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        let nb = ctx.notebook.lock().await;
        match nb.get_cell(&params.cell_id) {
            Some(cell) => success_json(cell),
            None => error_result(format!("Cell '{}' not found", params.cell_id)),
        }
    }

    #[tool(description = "Add a new cell to the notebook. Returns the new cell's ID.

Unicode Greek letters (α, β, γ, θ, π, etc.) are supported in code cells and translated to Maxima symbols automatically. For markdown cells: use real newlines (not literal \\n) for line breaks, and single backslashes for LaTeX commands (e.g. \\sin, \\varphi).

Output works like normal Maxima: statements ending with `;` display their result, statements ending with `$` are silent. The last statement's result is rendered as LaTeX when it ends with `;`. End the last statement with `$` to suppress all output.

Best practice: run each cell immediately after creating it (using run_cell) to verify the output before proceeding.")]
    async fn add_cell(
        &self,
        Parameters(params): Parameters<AddCellParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        let cell_type = match params.cell_type.as_deref() {
            Some("markdown") => CellType::Markdown,
            _ => CellType::Code,
        };
        let input = params.input.unwrap_or_default();
        let mut nb = ctx.notebook.lock().await;
        let effect = nb.apply(NotebookCommand::AddCell {
            cell_type,
            input,
            after_cell_id: params.after_cell_id,
            before_cell_id: None,
        })?;
        let cell_id = effect.cell_id().unwrap_or("").to_string();
        drop(nb);
        self.core.notify_notebook_change(&ctx.id, effect);
        success_json(&serde_json::json!({ "cell_id": cell_id }))
    }

    #[tool(description = "Update a cell's content, type, or both in a single call. Provide only the fields you want to change.

Unicode Greek letters (α, β, γ, θ, π, etc.) are supported in code cells and translated to Maxima symbols automatically. For markdown cells: use real newlines (not literal \\n) for line breaks, and single backslashes for LaTeX commands (e.g. \\sin, \\varphi).

Output works like normal Maxima: statements ending with `;` display their result, statements ending with `$` are silent. The last statement's result is rendered as LaTeX when it ends with `;`. End the last statement with `$` to suppress all output.

Best practice: run each cell immediately after updating it (using run_cell) to verify the output before moving on.")]
    async fn update_cell(
        &self,
        Parameters(params): Parameters<UpdateCellParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        let mut nb = ctx.notebook.lock().await;
        let mut last_effect = None;
        // Apply input update if provided
        if let Some(input) = params.input {
            last_effect = Some(nb.apply(NotebookCommand::UpdateCellInput {
                cell_id: params.cell_id.clone(),
                input,
                trusted: false,
            })?);
        }
        // Apply cell type toggle if the requested type differs
        if let Some(ref type_str) = params.cell_type {
            let requested = match type_str.as_str() {
                "markdown" => CellType::Markdown,
                _ => CellType::Code,
            };
            if let Some(cell) = nb.get_cell(&params.cell_id) {
                if cell.cell_type != requested {
                    last_effect = Some(nb.apply(NotebookCommand::ToggleCellType {
                        cell_id: params.cell_id.clone(),
                    })?);
                }
            } else {
                return error_result(format!("Cell '{}' not found", params.cell_id));
            }
        }
        drop(nb);
        if let Some(effect) = last_effect {
            self.core.notify_notebook_change(&ctx.id, effect);
        }
        success_json(&serde_json::json!({ "updated": true }))
    }

    #[tool(description = "Delete a cell from the notebook. Cannot delete the last remaining cell.")]
    async fn delete_cell(
        &self,
        Parameters(params): Parameters<CellIdParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        let mut nb = ctx.notebook.lock().await;
        let effect = nb.apply(NotebookCommand::DeleteCell {
            cell_id: params.cell_id.clone(),
        })?;
        drop(nb);
        self.core.notify_notebook_change(&ctx.id, effect.clone());
        match effect {
            CommandEffect::NoOp { reason } => {
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
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        let mut nb = ctx.notebook.lock().await;
        let effect = nb.apply(NotebookCommand::MoveCell {
            cell_id: params.cell_id.clone(),
            direction: params.direction.clone(),
        })?;
        drop(nb);
        self.core.notify_notebook_change(&ctx.id, effect.clone());
        match effect {
            CommandEffect::NoOp { reason } => {
                error_result(reason)
            }
            _ => success_json(&serde_json::json!({ "moved": true })),
        }
    }

    // ── Execution tools ───────────────────────────────────────────

    #[tool(description = "Execute a notebook cell. Auto-starts the Maxima session if needed. Returns the evaluation result including text output, LaTeX, plots, and errors.

Output works like normal Maxima: statements ending with `;` display their result, statements ending with `$` are silent. The last statement's result is rendered as LaTeX when it ends with `;`. End the last statement with `$` to suppress all output.

Plots: plot2d and plot3d produce inline SVG returned in the result. Prefer these over the draw package, which outputs to gnuplot directly and won't be captured.

Best practice: run each cell immediately after creating it before moving on to the next. This ensures earlier definitions are available to later cells and lets you catch errors early.")]
    async fn run_cell(
        &self,
        Parameters(params): Parameters<CellIdParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;

        // Safety check for dangerous functions
        {
            let nb = ctx.notebook.lock().await;
            let cell = nb.get_cell(&params.cell_id)
                .ok_or_else(|| format!("Cell '{}' not found", params.cell_id))?;
            let trusted = cell.trusted;
            let input = cell.input.clone();
            drop(nb);

            if !trusted {
                let dangerous = safety::detect_dangerous_calls(&input, Some(&self.core.packages));
                if !dangerous.is_empty() {
                    let func_names: Vec<String> = dangerous.iter().map(|d| d.function_name.clone()).collect();

                    if self.core.allow_dangerous {
                        // Headless + --allow-dangerous: proceed
                    } else if self.core.on_notebook_change.is_some() {
                        // Connected mode: set pending approval and notify GUI
                        let mut nb = ctx.notebook.lock().await;
                        let effect = nb.apply(NotebookCommand::SetCellPendingApproval {
                            cell_id: params.cell_id.clone(),
                            dangerous_functions: func_names.clone(),
                        })?;
                        drop(nb);
                        self.core.notify_notebook_change(&ctx.id, effect);
                        return success_json(&serde_json::json!({
                            "cell_id": params.cell_id,
                            "pending_approval": true,
                            "dangerous_functions": func_names,
                            "message": "Cell contains dangerous functions. Approve in the GUI to execute.",
                        }));
                    } else {
                        // Headless without --allow-dangerous: block
                        return error_result(format!(
                            "Dangerous function(s) blocked: {}. Use --allow-dangerous to allow.",
                            func_names.join(", ")
                        ));
                    }
                }
            }
        }

        if let Err(e) = self.core.ensure_session(&ctx).await {
            return error_result(format!("Failed to start session: {e}"));
        }

        use aximar_core::error::AppError;
        use aximar_core::evaluation::evaluate_cell;

        match evaluate_cell(&ctx, &params.cell_id, &self.core.catalog, &self.core.packages, self.core.eval_timeout).await {
            Ok(result) => {
                for effect in &result.effects {
                    self.core.notify_notebook_change(&ctx.id, effect.clone());
                }
                success_json(&serde_json::json!({
                    "cell_id": params.cell_id,
                    "execution_count": result.execution_count,
                    "text_output": result.cell_output.text_output,
                    "latex": result.cell_output.latex,
                    "plot_svg": result.cell_output.plot_svg,
                    "error": result.cell_output.error,
                    "is_error": result.cell_output.is_error,
                    "duration_ms": result.cell_output.duration_ms,
                    "output_label": result.cell_output.output_label,
                }))
            }
            Err(AppError::CellIsMarkdown) => {
                success_json(&serde_json::json!({
                    "cell_id": params.cell_id,
                    "message": "Markdown cell — nothing to execute"
                }))
            }
            Err(AppError::EmptyInput) => {
                success_json(&serde_json::json!({
                    "cell_id": params.cell_id,
                    "message": "Empty cell — nothing to execute"
                }))
            }
            Err(e) => error_result(format!("Evaluation failed: {e}")),
        }
    }

    #[tool(description = "Execute all code cells in the notebook in order. Returns results for each cell. Stops on the first cell that produces an error. When building a notebook, prefer running cells individually with run_cell to verify each result before proceeding.")]
    async fn run_all_cells(
        &self,
        Parameters(params): Parameters<NotebookIdParam>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        if let Err(e) = self.core.ensure_session(&ctx).await {
            return error_result(format!("Failed to start session: {e}"));
        }

        let cell_ids: Vec<String> = {
            let nb = ctx.notebook.lock().await;
            nb.cells()
                .iter()
                .filter(|c| c.cell_type == CellType::Code)
                .map(|c| c.id.clone())
                .collect()
        };

        let mut results = Vec::new();
        for cell_id in &cell_ids {
            let result = self
                .run_cell(Parameters(CellIdParams {
                    cell_id: cell_id.to_string(),
                    notebook_id: Some(ctx.id.clone()),
                }))
                .await;
            match &result {
                Err(_) => {
                    results.push(serde_json::json!({
                        "cell_id": cell_id,
                        "success": false,
                    }));
                    break;
                }
                Ok(json_str) => {
                    // Check if the result indicates pending approval
                    let is_pending = serde_json::from_str::<serde_json::Value>(json_str)
                        .ok()
                        .and_then(|v| v.get("pending_approval")?.as_bool())
                        .unwrap_or(false);
                    results.push(serde_json::json!({
                        "cell_id": cell_id,
                        "success": !is_pending,
                        "pending_approval": is_pending,
                    }));
                    if is_pending {
                        break;
                    }
                }
            }
        }

        success_json(&serde_json::json!({
            "cells_run": results.len(),
            "results": results,
        }))
    }

    #[tool(description = "Evaluate a Maxima expression without creating a notebook cell. Good for quick calculations and checks. Auto-starts the session if needed. The expression is ephemeral (no cell is created), but session state persists — variables and definitions set here are available to subsequent cells and expressions.

Unicode Greek letters (α, β, γ, θ, π, etc.) are translated to Maxima symbols automatically. Statements ending with `;` display their result, statements ending with `$` are silent. The last statement's result is rendered as LaTeX when it ends with `;`. End the last statement with `$` to suppress all output.")]
    async fn evaluate_expression(
        &self,
        Parameters(params): Parameters<EvaluateExpressionParams>,
    ) -> Result<String, String> {
        self.core
            .do_evaluate_expression(&params.expression, params.notebook_id.as_deref())
            .await
    }

    // ── Session tools ─────────────────────────────────────────────

    #[tool(description = "Get the current Maxima session status: Starting, Ready, Busy, Stopped, or Error.")]
    async fn get_session_status(
        &self,
        Parameters(params): Parameters<NotebookIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_get_session_status(params.notebook_id.as_deref())
            .await
    }

    #[tool(description = "Restart the Maxima session. Kills the current process and starts a new one. All session state is lost, including variables, function definitions, and loaded packages. You will need to re-run cells or re-load packages after restarting.")]
    async fn restart_session(
        &self,
        Parameters(params): Parameters<NotebookIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_restart_session(params.notebook_id.as_deref())
            .await
    }

    #[tool(description = "List all user-defined variables in the current Maxima session. Internal Maxima and package variables are filtered out — only variables you have explicitly assigned are shown.")]
    async fn list_variables(
        &self,
        Parameters(params): Parameters<NotebookIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_list_variables(params.notebook_id.as_deref())
            .await
    }

    #[tool(description = "Remove a variable from the Maxima session (equivalent to `kill(name)` in Maxima). For reproducible notebooks, prefer adding a code cell with kill(name)$ instead, so the operation is visible and re-runnable.")]
    async fn kill_variable(
        &self,
        Parameters(params): Parameters<KillVariableParams>,
    ) -> Result<String, String> {
        self.core
            .do_kill_variable(&params.name, params.notebook_id.as_deref())
            .await
    }

    // ── Log tools ─────────────────────────────────────────────────

    #[tool(description = "Get the raw Maxima I/O log for a specific cell, showing stdin/stdout/stderr streams. Useful for debugging unexpected output or protocol-level issues when a cell's parsed result doesn't match expectations.")]
    async fn get_cell_output_log(
        &self,
        Parameters(params): Parameters<CellIdParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        let nb = ctx.notebook.lock().await;
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

    #[tool(description = "Get the server-wide Maxima output log. Useful for debugging session startup or crash issues. Supports optional stream filter (\"stdout\", \"stderr\", or \"stdin\") and a limit on the number of entries returned.")]
    async fn get_server_log(
        &self,
        Parameters(params): Parameters<GetServerLogParams>,
    ) -> Result<String, String> {
        self.core
            .do_get_server_log(
                params.notebook_id.as_deref(),
                params.stream.as_deref(),
                params.limit,
            )
            .await
    }

    // ── Notebook I/O tools ────────────────────────────────────────

    #[tool(description = "Save the current notebook to a file in Jupyter .ipynb format. The saved file is compatible with Jupyter and other .ipynb tools.")]
    async fn save_notebook(
        &self,
        Parameters(params): Parameters<NotebookPathParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        let nb = ctx.notebook.lock().await;
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

    #[tool(description = "Open a Jupyter .ipynb notebook file and load it into the editor, replacing the current notebook state. Supports standard .ipynb format from Jupyter and other compatible tools.")]
    async fn open_notebook(
        &self,
        Parameters(params): Parameters<NotebookPathParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        match notebook_io::read_notebook(&params.path) {
            Ok(notebook) => {
                let cells = ipynb_to_cell_tuples(&notebook);
                let mut nb = ctx.notebook.lock().await;
                let effect = nb.apply(NotebookCommand::LoadCells { cells })?;
                let cell_count = nb.cells().len();
                drop(nb);
                self.core.notify_notebook_change(&ctx.id, effect);
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

    #[tool(description = "Load a template into the notebook, replacing current content. Use list_templates first to see available template IDs and descriptions. Warning: this replaces all existing cells in the notebook.")]
    async fn load_template(
        &self,
        Parameters(params): Parameters<LoadTemplateParams>,
    ) -> Result<String, String> {
        let ctx = self.core.resolve_context(params.notebook_id.as_deref()).await?;
        match notebook_data::get_template(&params.template_id) {
            Some(notebook) => {
                let cells = ipynb_to_cell_tuples(&notebook);
                let mut nb = ctx.notebook.lock().await;
                let effect = nb.apply(NotebookCommand::LoadCells { cells })?;
                let cell_count = nb.cells().len();
                drop(nb);
                self.core.notify_notebook_change(&ctx.id, effect);
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

// ── ServerHandler implementation (notebook mode) ──────────────────────

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
                 available to later cells and lets you catch errors early.\n\n\
                 To clear a variable binding, add a code cell with `kill(varname)$` and run it. \
                 This keeps the operation visible in the notebook so it works when re-run from \
                 scratch. The kill_variable tool is fine for ad-hoc cleanup, but when building a \
                 notebook that should work start to finish, prefer a code cell.\n\n\
                 Markdown cell escaping: cell input strings are stored verbatim, not \
                 interpreted as JSON escape sequences. Use real newlines (not \\n literals) \
                 for line breaks. For LaTeX in markdown, use single backslashes \
                 (e.g. `\\sin`, `\\circ`, `\\varphi`) — do not double-escape them.\n\n\
                 For plotting, prefer `plot2d` and `plot3d` over the `draw` package \
                 (`draw2d`, `draw3d`). The plot functions produce inline SVG that Aximar \
                 can capture and display in the notebook, while draw outputs to gnuplot \
                 directly and the resulting plot will not be visible. `plot2d` supports \
                 implicit equations natively (e.g. `plot2d(x^2+y^2=1, [x,-2,2], [y,-2,2])`).\n\n\
                 At the start of a session, consider calling `list_deprecated` to check \
                 whether any functions you plan to use are deprecated or obsolete. The tool \
                 returns suggested replacements where available.\n\n\
                 Many Maxima functions live in loadable packages (e.g. `distrib`, \
                 `linearalgebra`, `draw`). Use `search_packages` or `list_packages` to \
                 discover available packages and `get_package` to see what functions a \
                 package provides. Load a package with a code cell containing \
                 `load(\"name\")$`.\n\n\
                 Cell output works like normal Maxima: statements ending with `;` \
                 display their result, statements ending with `$` are silent. The \
                 last statement's result is rendered as LaTeX when it ends with `;`. \
                 End the last statement with `$` to suppress all output. Use \
                 `print(expr)` for explicit plain text or `tex(expr)` for rendered \
                 LaTeX at any point. User `tex()` calls appear as rendered LaTeX \
                 blocks in the text output — useful in loops, e.g. \
                 `for n:1 thru 3 do (print(\"n =\", n, \":\"), tex(f(n)))` produces \
                 labelled rendered math for each iteration.\n\n\
                 When working with multiple notebooks, use `list_notebooks` to see all open \
                 notebooks, `create_notebook` to open a new one, `close_notebook` to remove \
                 one, and `switch_notebook` to change the active default. Most tools accept \
                 an optional `notebook_id` parameter — when omitted, they target the active \
                 notebook.",
            )
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
