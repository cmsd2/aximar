use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router};

use crate::params::{
    CompleteFunctionParams, GetFunctionDocsParams, GetPackageParams, SearchFunctionsParams,
    SearchPackagesParams,
};
use crate::simple_params::*;
use crate::server::ServerCore;

// ── SimpleMcpServer (simple/session mode) ─────────────────────────────

/// MCP server with a reduced, session-oriented tool set.
///
/// Exposes evaluation, session management, and catalog/docs tools without
/// the full notebook cell model. This is the default mode when running
/// `aximar-mcp` without the `--notebook` flag.
#[derive(Clone)]
pub struct SimpleMcpServer {
    #[allow(dead_code)]
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
    pub(crate) core: ServerCore,
}

impl SimpleMcpServer {
    /// Build from an existing `ServerCore`.
    pub fn new(core: ServerCore) -> Self {
        SimpleMcpServer {
            tool_router: Self::tool_router(),
            core,
        }
    }
}

// ── Tool implementations (simple mode — session-oriented) ─────────────

#[tool_router]
impl SimpleMcpServer {
    // ── Session lifecycle tools ───────────────────────────────────

    #[tool(description = "Create a new isolated Maxima session. Returns the new session's ID. Variables and definitions in one session are isolated from other sessions. Optionally pass a filesystem path so that load() and batchload() resolve relative paths from that directory.",
           annotations(read_only_hint = false, destructive_hint = false))]
    async fn create_session(
        &self,
        Parameters(params): Parameters<CreateSessionParams>,
    ) -> Result<String, String> {
        self.core.do_create_session(params.path.as_deref()).await
    }

    #[tool(description = "Close a session and stop its Maxima process. Cannot close the last remaining session.",
           annotations(read_only_hint = false, destructive_hint = true))]
    async fn close_session(
        &self,
        Parameters(params): Parameters<CloseSessionParams>,
    ) -> Result<String, String> {
        self.core.do_close_session(&params.session_id).await
    }

    #[tool(description = "List all open sessions with their IDs and active status.",
           annotations(read_only_hint = true))]
    async fn list_sessions(&self) -> Result<String, String> {
        self.core.do_list_sessions().await
    }

    // ── Evaluation ────────────────────────────────────────────────

    #[tool(description = "Evaluate a Maxima expression. Auto-starts the session if needed. Session state persists — variables and definitions set here are available to subsequent expressions.

Unicode Greek letters (α, β, γ, θ, π, etc.) are translated to Maxima symbols automatically. Statements ending with `;` display their result; statements ending with `$` are silent. The last statement's result is rendered as LaTeX when it ends with `;`. End the last statement with `$` to suppress all output, just like in normal Maxima.",
           annotations(read_only_hint = false, destructive_hint = false))]
    async fn evaluate_expression(
        &self,
        Parameters(params): Parameters<SimpleEvaluateExpressionParams>,
    ) -> Result<String, String> {
        self.core
            .do_evaluate_expression(&params.expression, params.session_id.as_deref())
            .await
    }

    // ── Session management ────────────────────────────────────────

    #[tool(description = "Get the current Maxima session status: Starting, Ready, Busy, Stopped, or Error.",
           annotations(read_only_hint = true))]
    async fn get_session_status(
        &self,
        Parameters(params): Parameters<SessionIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_get_session_status(params.session_id.as_deref())
            .await
    }

    #[tool(description = "Restart the Maxima session. Kills the current process and starts a new one. All session state is lost, including variables, function definitions, and loaded packages.",
           annotations(read_only_hint = false, destructive_hint = true))]
    async fn restart_session(
        &self,
        Parameters(params): Parameters<SessionIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_restart_session(params.session_id.as_deref())
            .await
    }

    #[tool(description = "List all user-defined variables in the current Maxima session. Internal Maxima and package variables are filtered out — only variables you have explicitly assigned are shown.",
           annotations(read_only_hint = true))]
    async fn list_variables(
        &self,
        Parameters(params): Parameters<SessionIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_list_variables(params.session_id.as_deref())
            .await
    }

    #[tool(description = "Remove a variable from the Maxima session (equivalent to `kill(name)` in Maxima).",
           annotations(read_only_hint = false, destructive_hint = true))]
    async fn kill_variable(
        &self,
        Parameters(params): Parameters<SimpleKillVariableParams>,
    ) -> Result<String, String> {
        self.core
            .do_kill_variable(&params.name, params.session_id.as_deref())
            .await
    }

    // ── Documentation tools ───────────────────────────────────────

    #[tool(description = "Search the Maxima function catalog by name or description. Returns matching functions with signatures and brief descriptions. Searches across 2500+ built-in and package functions. Supports partial name matching (e.g. \"integ\" finds integrate) and description keywords (e.g. \"matrix inverse\").",
           annotations(read_only_hint = true))]
    async fn search_functions(
        &self,
        Parameters(params): Parameters<SearchFunctionsParams>,
    ) -> Result<String, String> {
        self.core.do_search_functions(&params.query).await
    }

    #[tool(description = "Get full documentation for a Maxima function, including usage, examples, and related functions. Falls back to a catalog summary if full docs are unavailable. Suggests similar function names if the exact name is not found.",
           annotations(read_only_hint = true))]
    async fn get_function_docs(
        &self,
        Parameters(params): Parameters<GetFunctionDocsParams>,
    ) -> Result<String, String> {
        self.core.do_get_function_docs(&params.name).await
    }

    #[tool(description = "List Maxima functions that are deprecated, obsolete, or superseded. Returns names, descriptions, and suggested replacements where available.",
           annotations(read_only_hint = true))]
    async fn list_deprecated(&self) -> Result<String, String> {
        self.core.do_list_deprecated().await
    }

    #[tool(description = "Autocomplete a Maxima function name prefix. Returns matching function names with signatures. Includes both built-in and package functions.",
           annotations(read_only_hint = true))]
    async fn complete_function(
        &self,
        Parameters(params): Parameters<CompleteFunctionParams>,
    ) -> Result<String, String> {
        self.core.do_complete_function(&params.prefix).await
    }

    // ── Package tools ─────────────────────────────────────────────

    #[tool(description = "Search available Maxima packages by name or description. Returns packages with their load paths and function lists. Load a package with load(\"name\")$ before using its functions.",
           annotations(read_only_hint = true))]
    async fn search_packages(
        &self,
        Parameters(params): Parameters<SearchPackagesParams>,
    ) -> Result<String, String> {
        self.core.do_search_packages(&params.query).await
    }

    #[tool(description = "List all available Maxima packages that can be loaded with load(\"name\")$. Use get_package to see what functions a specific package provides.",
           annotations(read_only_hint = true))]
    async fn list_packages(&self) -> Result<String, String> {
        self.core.do_list_packages().await
    }

    #[tool(description = "Get details of a specific Maxima package, including description and list of functions it provides.",
           annotations(read_only_hint = true))]
    async fn get_package(
        &self,
        Parameters(params): Parameters<GetPackageParams>,
    ) -> Result<String, String> {
        self.core.do_get_package(&params.name).await
    }

    // ── Log tools ─────────────────────────────────────────────────

    #[tool(description = "Get the server-wide Maxima output log. Useful for debugging session startup or crash issues. Supports optional stream filter (\"stdout\", \"stderr\", or \"stdin\") and a limit on the number of entries returned.",
           annotations(read_only_hint = true))]
    async fn get_server_log(
        &self,
        Parameters(params): Parameters<SimpleGetServerLogParams>,
    ) -> Result<String, String> {
        self.core
            .do_get_server_log(
                params.session_id.as_deref(),
                params.stream.as_deref(),
                params.limit,
            )
            .await
    }
}

// ── ServerHandler implementation (simple mode) ────────────────────────

#[tool_handler]
impl rmcp::handler::server::ServerHandler for SimpleMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::default();
        capabilities.tools = Some(Default::default());

        ServerInfo::new(capabilities)
            .with_server_info(
                Implementation::new("aximar-mcp", env!("CARGO_PKG_VERSION"))
                    .with_title("Aximar MCP Server (Simple)")
                    .with_description(
                        "MCP server for the Maxima computer algebra system — session mode",
                    ),
            )
            .with_instructions(
                "Aximar provides access to the Maxima computer algebra system. \
                 You can evaluate expressions, search function documentation, \
                 inspect variables, and manage sessions.\n\n\
                 Prefer Unicode Greek letters (α, β, γ, δ, θ, λ, μ, π, σ, φ, ω, etc.) \
                 in Maxima code. Write π rather than %pi, ∞ rather than inf, etc. The \
                 protocol translates Unicode Greek to the corresponding Maxima symbols \
                 automatically.\n\n\
                 Use `evaluate_expression` to run Maxima code. Session state persists \
                 between calls — variables, function definitions, and loaded packages \
                 carry over. Use `restart_session` to start fresh.\n\n\
                 Many Maxima functions live in loadable packages (e.g. `distrib`, \
                 `linearalgebra`). Use `search_packages` or `list_packages` to discover \
                 available packages and `get_package` to see what functions a package \
                 provides. Load a package with `load(\"name\")$`.\n\n\
                 Output works like normal Maxima: statements ending with `;` display \
                 their result, statements ending with `$` are silent. The last \
                 statement's result is rendered as LaTeX when it ends with `;`. End \
                 the last statement with `$` to suppress all output.\n\n\
                 Multiple sessions are supported. Use `create_session` to create an \
                 isolated session, `list_sessions` to see all open sessions, and pass \
                 `session_id` to target a specific session. When omitted, tools target \
                 the default session.",
            )
    }
}
