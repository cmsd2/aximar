use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router};

use crate::params::{
    CompleteFunctionParams, GetFunctionDocsParams, GetPackageParams, SearchFunctionsParams,
    SearchPackagesParams,
};
use crate::server::{ClientCell, ExpressionLabels, ServerCore};
use crate::simple_params::*;

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

    #[tool(
        description = "Create a new isolated Maxima session. Returns the new session's ID. Variables and definitions in one session are isolated from other sessions. Optionally pass a filesystem path so that load() and batchload() resolve relative paths from that directory.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn create_session(
        &self,
        Parameters(params): Parameters<CreateSessionParams>,
    ) -> Result<String, String> {
        self.core.do_create_session(params.path.as_deref()).await
    }

    #[tool(
        description = "Close a session and stop its Maxima process. Cannot close the last remaining session.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn close_session(
        &self,
        Parameters(params): Parameters<CloseSessionParams>,
    ) -> Result<String, String> {
        self.core.do_close_session(&params.session_id).await
    }

    #[tool(
        description = "List all open sessions with their IDs and active status.",
        annotations(read_only_hint = true)
    )]
    async fn list_sessions(&self) -> Result<String, String> {
        self.core.do_list_sessions().await
    }

    // ── Evaluation ────────────────────────────────────────────────

    #[tool(
        description = "Evaluate a Maxima expression. Auto-starts the session if needed. Session state persists — variables and definitions set here are available to subsequent expressions.

Unicode Greek letters (α, β, γ, θ, π, etc.) are translated to Maxima symbols automatically. Statements ending with `;` display their result; statements ending with `$` are silent. The last statement's result is rendered as LaTeX when it ends with `;`. End the last statement with `$` to suppress all output, just like in normal Maxima.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn evaluate_expression(
        &self,
        Parameters(params): Parameters<SimpleEvaluateExpressionParams>,
    ) -> Result<String, String> {
        self.core
            .do_evaluate_expression(
                &params.expression,
                params.session_id.as_deref(),
                match params.cell_id {
                    Some(id) => ExpressionLabels::ClientCell(ClientCell {
                        id,
                        cells_above: params.cells_above,
                    }),
                    // A plain Maxima session: %oN means Maxima's own label
                    None => ExpressionLabels::Maxima,
                },
            )
            .await
    }

    // ── Session management ────────────────────────────────────────

    #[tool(
        description = "Get the current Maxima session status: Starting, Ready, Busy, Stopped, or Error.",
        annotations(read_only_hint = true)
    )]
    async fn get_session_status(
        &self,
        Parameters(params): Parameters<SessionIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_get_session_status(params.session_id.as_deref())
            .await
    }

    #[tool(
        description = "Restart the Maxima session. Kills the current process and starts a new one. All session state is lost, including variables, function definitions, and loaded packages.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn restart_session(
        &self,
        Parameters(params): Parameters<SessionIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_restart_session(params.session_id.as_deref())
            .await
    }

    #[tool(
        description = "List all user-defined variables in the current Maxima session. Internal Maxima and package variables are filtered out — only variables you have explicitly assigned are shown.",
        annotations(read_only_hint = true)
    )]
    async fn list_variables(
        &self,
        Parameters(params): Parameters<SessionIdParam>,
    ) -> Result<String, String> {
        self.core
            .do_list_variables(params.session_id.as_deref())
            .await
    }

    #[tool(
        description = "Remove a variable from the Maxima session (equivalent to `kill(name)` in Maxima).",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn kill_variable(
        &self,
        Parameters(params): Parameters<SimpleKillVariableParams>,
    ) -> Result<String, String> {
        self.core
            .do_kill_variable(&params.name, params.session_id.as_deref())
            .await
    }

    // ── Documentation tools ───────────────────────────────────────

    #[tool(
        description = "Search the Maxima function catalog by name or description. Returns matching functions with signatures and brief descriptions. Searches across 2500+ built-in and package functions. Supports partial name matching (e.g. \"integ\" finds integrate) and description keywords (e.g. \"matrix inverse\").",
        annotations(read_only_hint = true)
    )]
    async fn search_functions(
        &self,
        Parameters(params): Parameters<SearchFunctionsParams>,
    ) -> Result<String, String> {
        self.core.do_search_functions(&params.query).await
    }

    #[tool(
        description = "Get full documentation for a Maxima function, including usage, examples, and related functions. Falls back to a catalog summary if full docs are unavailable. Suggests similar function names if the exact name is not found.",
        annotations(read_only_hint = true)
    )]
    async fn get_function_docs(
        &self,
        Parameters(params): Parameters<GetFunctionDocsParams>,
    ) -> Result<String, String> {
        self.core.do_get_function_docs(&params.name).await
    }

    #[tool(
        description = "List Maxima functions that are deprecated, obsolete, or superseded. Returns names, descriptions, and suggested replacements where available.",
        annotations(read_only_hint = true)
    )]
    async fn list_deprecated(&self) -> Result<String, String> {
        self.core.do_list_deprecated().await
    }

    #[tool(
        description = "Autocomplete a Maxima function name prefix. Returns matching function names with signatures. Includes both built-in and package functions.",
        annotations(read_only_hint = true)
    )]
    async fn complete_function(
        &self,
        Parameters(params): Parameters<CompleteFunctionParams>,
    ) -> Result<String, String> {
        self.core.do_complete_function(&params.prefix).await
    }

    // ── Package tools ─────────────────────────────────────────────

    #[tool(
        description = "Search available Maxima packages by name or description. Returns packages with their load paths and function lists. Load a package with load(\"name\")$ before using its functions.",
        annotations(read_only_hint = true)
    )]
    async fn search_packages(
        &self,
        Parameters(params): Parameters<SearchPackagesParams>,
    ) -> Result<String, String> {
        self.core.do_search_packages(&params.query).await
    }

    #[tool(
        description = "List all available Maxima packages that can be loaded with load(\"name\")$. Use get_package to see what functions a specific package provides.",
        annotations(read_only_hint = true)
    )]
    async fn list_packages(&self) -> Result<String, String> {
        self.core.do_list_packages().await
    }

    #[tool(
        description = "Get details of a specific Maxima package, including description and list of functions it provides.",
        annotations(read_only_hint = true)
    )]
    async fn get_package(
        &self,
        Parameters(params): Parameters<GetPackageParams>,
    ) -> Result<String, String> {
        self.core.do_get_package(&params.name).await
    }

    // ── Log tools ─────────────────────────────────────────────────

    #[tool(
        description = "Get the server-wide Maxima output log. Useful for debugging session startup or crash issues. Supports optional stream filter (\"stdout\", \"stderr\", or \"stdin\") and a limit on the number of entries returned.",
        annotations(read_only_hint = true)
    )]
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
                 automatically. Notation that isn't Maxima syntax, such as ∑, ∫, ∂, ≈, ° or 2π, is \
                 rejected with a hint: write sum(...), integrate(...), diff(...), 2*π.\n\n\
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
                 Symbolic vs numeric gotcha: Maxima keeps expressions like \
                 `cos(2*%pi*0.73)` symbolic because `%pi` is a symbolic constant. \
                 Wrap in `float()` to force numeric evaluation when building numeric \
                 lists or doing statistical computations.\n\n\
                 Multiple sessions are supported. Use `create_session` to create an \
                 isolated session, `list_sessions` to see all open sessions, and pass \
                 `session_id` to target a specific session. When omitted, tools target \
                 the default session.",
            )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rmcp::handler::server::wrapper::Parameters;
    use tokio::sync::Mutex;

    use aximar_core::catalog::packages::PackageCatalog;
    use aximar_core::catalog::search::Catalog;
    use aximar_core::maxima::backend::Backend;
    use aximar_core::registry::NotebookRegistry;
    use aximar_core::session_ops::SessionSettings;

    use super::*;

    fn server() -> SimpleMcpServer {
        let settings = SessionSettings {
            backend: Backend::Local,
            maxima_path: None,
            eval_timeout: 30,
        };
        SimpleMcpServer::new(ServerCore::new(
            Arc::new(Mutex::new(NotebookRegistry::new())),
            Arc::new(Catalog::load()),
            Arc::new(PackageCatalog::load()),
            settings.fixed(),
            false,
        ))
    }

    /// Run `expression` as cell `id` below `above`, as an editor's
    /// notebook view does.
    async fn run(
        s: &SimpleMcpServer,
        expression: &str,
        id: &str,
        above: &[&str],
    ) -> Result<serde_json::Value, String> {
        let params: SimpleEvaluateExpressionParams = serde_json::from_value(serde_json::json!({
            "expression": expression, "cell_id": id, "cells_above": above,
        }))
        .unwrap();
        s.evaluate_expression(Parameters(params))
            .await
            .map(|r| serde_json::from_str(&r).unwrap())
    }

    #[tokio::test]
    #[ignore = "requires Maxima to be installed"]
    async fn a_client_cell_with_an_unknown_label_fails_in_its_cell() {
        // Refused before Maxima evaluates it; the cell still gets its number
        let s = server();
        let v = run(&s, "%o2 + 1;", "c1", &[]).await.unwrap();
        assert_eq!(v["is_error"], true);
        assert_eq!(v["execution_count"], 1);
        assert!(
            v["error"]
                .as_str()
                .unwrap()
                .contains("Out[2] (%o2) has no value in this session"),
            "{v}"
        );
        // A bare % with nothing above has no value either
        let v = run(&s, "% + 1;", "c2", &["c1"]).await.unwrap();
        assert_eq!(
            (v["is_error"].as_bool(), v["execution_count"].as_u64()),
            (Some(true), Some(2)),
            "{v}"
        );
    }

    #[tokio::test]
    #[ignore = "requires Maxima to be installed"]
    async fn client_cells_follow_the_windows_label_rules() {
        let s = server();
        let a = run(&s, "10;", "a", &[]).await.unwrap();
        assert_eq!(a["execution_count"], 1);
        let b = run(&s, "5;", "b", &["a"]).await.unwrap();
        assert_eq!(b["execution_count"], 2);
        // A cell that fails is skipped by a bare % below it
        let c = run(&s, "1/0;", "c", &["a", "b"]).await.unwrap();
        assert_eq!(c["is_error"], true);
        let d = run(&s, "%o1 + %;", "d", &["a", "b", "c"]).await.unwrap();
        assert_eq!(
            (d["execution_count"].as_u64(), d["latex"].as_str()),
            (Some(4), Some("15")),
            "{d}"
        );
        // A cell this session never ran, such as one reopened from a file,
        // isn't an output above
        let e = run(&s, "%;", "e", &["from-file"]).await.unwrap();
        assert_eq!(e["is_error"], true, "{e}");
    }

    #[tokio::test]
    #[ignore = "requires Maxima to be installed"]
    async fn an_expression_without_a_cell_takes_no_number() {
        let s = server();
        let params: SimpleEvaluateExpressionParams =
            serde_json::from_value(serde_json::json!({ "expression": "7;" })).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&s.evaluate_expression(Parameters(params)).await.unwrap())
                .unwrap();
        assert!(v["execution_count"].is_null(), "{v}");
        assert_eq!(run(&s, "1;", "a", &[]).await.unwrap()["execution_count"], 1);
    }

    #[tokio::test]
    #[ignore = "requires Maxima to be installed"]
    async fn a_result_comes_with_its_one_line_form() {
        // Clients write it as the result's text/plain in notebook files
        let s = server();
        let v = run(&s, "x^2/2;", "a", &[]).await.unwrap();
        assert_eq!(v["text_plain"], "x^2/2", "{v}");
        assert!(v["latex"].is_string(), "{v}");
        let v = run(&s, "x: 1$", "b", &["a"]).await.unwrap();
        assert!(v["text_plain"].is_null(), "{v}");
    }

    #[tokio::test]
    #[ignore = "requires Maxima to be installed"]
    async fn a_plain_expression_keeps_maxima_s_labels() {
        // As in a plain Maxima session, and as tools-v0.5.0 did: clients
        // that resolve labels themselves send Maxima's real labels
        let s = server();
        let plain = |e: &str| -> SimpleEvaluateExpressionParams {
            serde_json::from_value(serde_json::json!({ "expression": e })).unwrap()
        };
        let v: serde_json::Value = serde_json::from_str(
            &s.evaluate_expression(Parameters(plain("10;")))
                .await
                .unwrap(),
        )
        .unwrap();
        let label = v["output_label"].as_str().unwrap().to_string();
        let v: serde_json::Value = serde_json::from_str(
            &s.evaluate_expression(Parameters(plain(&format!("{label} + 1;"))))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(v["latex"], "11", "{v}");
        // Unicode is still translated
        let v: serde_json::Value = serde_json::from_str(
            &s.evaluate_expression(Parameters(plain("α: 2$ α + 1;")))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(v["latex"], "3", "{v}");
    }

    #[test]
    fn the_published_parameters_offer_client_cells() {
        // Clients detect support for client cells from the tool's input
        // schema, which works for builds of any version number
        let schema =
            serde_json::to_value(schemars::schema_for!(SimpleEvaluateExpressionParams)).unwrap();
        let properties = &schema["properties"];
        assert!(
            properties.get("cell_id").is_some() && properties.get("cells_above").is_some(),
            "{schema}"
        );
    }
}
