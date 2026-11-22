use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use tokio::sync::Mutex;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::maxima::backend::Backend;
use aximar_core::registry::NotebookRegistry;

use aximar_core::session_ops::{SessionSettings, SessionSettingsProvider};

use crate::params::*;
use crate::server::AximarMcpServer;

/// The local Maxima on the PATH, with a 30-second timeout.
fn local_settings() -> SessionSettingsProvider {
    SessionSettings { backend: Backend::Local, maxima_path: None, eval_timeout: 30 }.fixed()
}

fn build_server() -> AximarMcpServer {
    AximarMcpServer::new(
        Arc::new(Mutex::new(NotebookRegistry::new())),
        Arc::new(Catalog::load()),
        Arc::new(PackageCatalog::load()),
        local_settings(),
        false,
    )
}

fn build_server_allow_dangerous() -> AximarMcpServer {
    AximarMcpServer::new(
        Arc::new(Mutex::new(NotebookRegistry::new())),
        Arc::new(Catalog::load()),
        Arc::new(PackageCatalog::load()),
        local_settings(),
        true,
    )
}

/// Parse a JSON result, panic on error.
fn parse_ok(result: Result<String, String>) -> serde_json::Value {
    let json = result.expect("tool returned error");
    serde_json::from_str(&json).expect("invalid JSON")
}

// ── Catalog tools ────────────────────────────────────────────────────

#[tokio::test]
async fn search_functions_returns_results() {
    let s = build_server();
    let v = parse_ok(
        s.search_functions(Parameters(SearchFunctionsParams {
            query: "integrate".into(),
        }))
        .await,
    );
    let results = v["results"].as_array().unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0]["name"], "integrate");
}

#[tokio::test]
async fn get_function_docs_found() {
    let s = build_server();
    let result = s
        .get_function_docs(Parameters(GetFunctionDocsParams {
            name: "diff".into(),
        }))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_function_docs_not_found_suggests() {
    let s = build_server();
    let result = s
        .get_function_docs(Parameters(GetFunctionDocsParams {
            name: "integarte".into(),
        }))
        .await;
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("Did you mean"));
}

#[tokio::test]
async fn complete_function_returns_matches() {
    let s = build_server();
    let v = parse_ok(
        s.complete_function(Parameters(CompleteFunctionParams {
            prefix: "integ".into(),
        }))
        .await,
    );
    let completions = v["completions"].as_array().unwrap();
    assert!(completions.iter().any(|c| c["name"] == "integrate"));
}

#[tokio::test]
async fn list_deprecated_returns_array() {
    let s = build_server();
    let v = parse_ok(s.list_deprecated().await);
    assert!(v["deprecated"].is_array());
}

// ── Package tools ────────────────────────────────────────────────────

#[tokio::test]
async fn search_packages_returns_results() {
    let s = build_server();
    let v = parse_ok(
        s.search_packages(Parameters(SearchPackagesParams {
            query: "distrib".into(),
        }))
        .await,
    );
    assert!(!v["results"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_packages_returns_all() {
    let s = build_server();
    let v = parse_ok(s.list_packages().await);
    assert!(v["count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn get_package_found() {
    let s = build_server();
    let v = parse_ok(
        s.get_package(Parameters(GetPackageParams {
            name: "distrib".into(),
        }))
        .await,
    );
    assert_eq!(v["name"], "distrib");
}

#[tokio::test]
async fn get_package_not_found() {
    let s = build_server();
    let result = s
        .get_package(Parameters(GetPackageParams {
            name: "nonexistent_pkg".into(),
        }))
        .await;
    assert!(result.is_err());
}

// ── Notebook lifecycle tools ─────────────────────────────────────────

#[tokio::test]
async fn list_notebooks_has_default() {
    let s = build_server();
    let v = parse_ok(s.list_notebooks().await);
    assert!(v["active_notebook_id"].is_string());
    assert!(!v["notebooks"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn create_and_switch_notebook() {
    let s = build_server();
    let v = parse_ok(s.create_notebook().await);
    let new_id = v["notebook_id"].as_str().unwrap().to_string();

    let v = parse_ok(
        s.switch_notebook(Parameters(SwitchNotebookParams {
            notebook_id: new_id.clone(),
        }))
        .await,
    );
    assert_eq!(v["active_notebook_id"], new_id);
}

#[tokio::test]
async fn close_notebook_standalone() {
    let s = build_server();
    // Create a second notebook so we can close it
    let v = parse_ok(s.create_notebook().await);
    let new_id = v["notebook_id"].as_str().unwrap().to_string();

    let v = parse_ok(
        s.close_notebook(Parameters(CloseNotebookParams {
            notebook_id: new_id,
        }))
        .await,
    );
    assert_eq!(v["closed"], true);
}

// ── Cell management tools ────────────────────────────────────────────

#[tokio::test]
async fn cell_crud_lifecycle() {
    let s = build_server();

    // List cells — should have one default cell
    let v = parse_ok(
        s.list_cells(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    let cells = v.as_array().unwrap();
    assert_eq!(cells.len(), 1);
    let first_cell_id = cells[0]["id"].as_str().unwrap().to_string();

    // Add a cell
    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some("x^2 + 1".into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );
    let new_id = v["cell_id"].as_str().unwrap().to_string();

    // Get cell
    let v = parse_ok(
        s.get_cell(Parameters(CellIdParams {
            cell_id: new_id.clone(),
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["input"], "x^2 + 1");
    assert_eq!(v["cell_type"], "code");

    // Update cell
    parse_ok(
        s.update_cell(Parameters(UpdateCellParams {
            cell_id: new_id.clone(),
            input: Some("x^3".into()),
            cell_type: Some("markdown".into()),
            notebook_id: None,
        }))
        .await,
    );
    let v = parse_ok(
        s.get_cell(Parameters(CellIdParams {
            cell_id: new_id.clone(),
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["input"], "x^3");
    assert_eq!(v["cell_type"], "markdown");

    // Move cell up
    let v = parse_ok(
        s.move_cell(Parameters(MoveCellParams {
            cell_id: new_id.clone(),
            direction: "up".into(),
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["moved"], true);

    // Verify order changed
    let v = parse_ok(
        s.list_cells(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    let cells = v.as_array().unwrap();
    assert_eq!(cells[0]["id"].as_str().unwrap(), new_id);
    assert_eq!(cells[1]["id"].as_str().unwrap(), first_cell_id);

    // Delete cell
    let v = parse_ok(
        s.delete_cell(Parameters(CellIdParams {
            cell_id: new_id.clone(),
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["deleted"], true);

    // Verify deleted
    let v = parse_ok(
        s.list_cells(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn add_cell_with_position() {
    let s = build_server();

    // Get the default cell ID
    let v = parse_ok(
        s.list_cells(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    let first_id = v.as_array().unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Add a cell after the first
    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: None,
            input: Some("second".into()),
            after_cell_id: Some(first_id.clone()),
            notebook_id: None,
        }))
        .await,
    );
    let second_id = v["cell_id"].as_str().unwrap().to_string();

    // Add a cell after the first (before the second)
    parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: None,
            input: Some("middle".into()),
            after_cell_id: Some(first_id.clone()),
            notebook_id: None,
        }))
        .await,
    );

    // Verify order: first, middle, second
    let v = parse_ok(
        s.list_cells(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    let cells = v.as_array().unwrap();
    assert_eq!(cells.len(), 3);
    assert_eq!(cells[0]["id"].as_str().unwrap(), first_id);
    assert_eq!(cells[1]["input"], "middle");
    assert_eq!(cells[2]["id"].as_str().unwrap(), second_id);
}

// ── Template and notebook I/O tools ──────────────────────────────────

#[tokio::test]
async fn list_templates_returns_entries() {
    let s = build_server();
    let v = parse_ok(s.list_templates().await);
    let templates = v["templates"].as_array().unwrap();
    assert!(!templates.is_empty());
    assert!(templates.iter().any(|t| t["id"] == "welcome"));
}

#[tokio::test]
async fn load_template_populates_cells() {
    let s = build_server();
    parse_ok(
        s.load_template(Parameters(LoadTemplateParams {
            template_id: "calculus".into(),
            notebook_id: None,
        }))
        .await,
    );

    let v = parse_ok(
        s.list_cells(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    let cells = v.as_array().unwrap();
    assert!(cells.len() > 5);
}

#[tokio::test]
async fn load_template_not_found() {
    let s = build_server();
    let result = s
        .load_template(Parameters(LoadTemplateParams {
            template_id: "nonexistent".into(),
            notebook_id: None,
        }))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn save_and_open_notebook() {
    let s = build_server();

    // Add a cell with content
    parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some("diff(x^3, x)".into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );

    // Save
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.ipynb");
    let path_str = path.to_str().unwrap().to_string();

    parse_ok(
        s.save_notebook(Parameters(NotebookPathParams {
            path: path_str.clone(),
            notebook_id: None,
        }))
        .await,
    );
    assert!(path.exists());

    // Create a new notebook and open the saved file into it
    let v = parse_ok(s.create_notebook().await);
    let nb_id = v["notebook_id"].as_str().unwrap().to_string();

    let v = parse_ok(
        s.open_notebook(Parameters(NotebookPathParams {
            path: path_str,
            notebook_id: Some(nb_id.clone()),
        }))
        .await,
    );
    assert_eq!(v["opened"], true);
    assert_eq!(v["cell_count"], 2); // default + added cell

    // Verify the cell content was preserved
    let v = parse_ok(
        s.list_cells(Parameters(NotebookIdParam {
            notebook_id: Some(nb_id),
        }))
        .await,
    );
    let cells = v.as_array().unwrap();
    assert!(cells.iter().any(|c| c["input"] == "diff(x^3, x)"));
}

// ── Session tools (no Maxima needed) ─────────────────────────────────

#[tokio::test]
async fn get_session_status_stopped() {
    let s = build_server();
    let v = parse_ok(
        s.get_session_status(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["status"], "Stopped");
}

// ── Log tools ────────────────────────────────────────────────────────

#[tokio::test]
async fn get_server_log_empty() {
    let s = build_server();
    let v = parse_ok(
        s.get_server_log(Parameters(GetServerLogParams {
            stream: None,
            limit: None,
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["count"], 0);
    assert!(v["log"].as_array().unwrap().is_empty());
}

// ── Evaluation tools (require Maxima) ────────────────────────────────

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn run_cell_evaluates() {
    let s = build_server();

    // Add a code cell
    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some("2 + 3".into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );
    let cell_id = v["cell_id"].as_str().unwrap().to_string();

    // Run it
    let v = parse_ok(
        s.run_cell(Parameters(CellIdParams {
            cell_id,
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["is_error"], false);
    // Single expression: 1D display is suppressed, result is in LaTeX only
    assert!(v["latex"].as_str().unwrap().contains("5"), "latex should contain the result");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn run_cell_markdown_returns_message() {
    let s = build_server();
    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("markdown".into()),
            input: Some("# Title".into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );
    let cell_id = v["cell_id"].as_str().unwrap().to_string();

    let v = parse_ok(
        s.run_cell(Parameters(CellIdParams {
            cell_id,
            notebook_id: None,
        }))
        .await,
    );
    assert!(v["message"].as_str().unwrap().contains("Markdown"));
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn run_all_cells_sequential() {
    let s = build_server();

    // Set up two code cells
    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some("a: 10".into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );
    let _cell1 = v["cell_id"].as_str().unwrap().to_string();

    parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some("a + 5".into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );

    let v = parse_ok(
        s.run_all_cells(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    // Default empty cell + 2 code cells = 3, but run_all_cells only runs Code cells
    // The default cell is empty so it will fail; let's just check we got results
    assert!(v["cells_run"].as_u64().unwrap() >= 1);
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn evaluate_expression_quick() {
    let s = build_server();
    let v = parse_ok(
        s.evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "factor(x^2 - 1)".into(),
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["is_error"], false);
    // Single expression: 1D display is suppressed, result is in LaTeX only
    let latex = v["latex"].as_str().unwrap();
    assert!(latex.contains("x-1") && latex.contains("x+1"), "latex should contain factored result: {}", latex);
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn list_and_kill_variable() {
    let s = build_server();

    // Define a variable
    parse_ok(
        s.evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "myvar: 42".into(),
            notebook_id: None,
        }))
        .await,
    );

    // List variables — should contain myvar
    let v = parse_ok(
        s.list_variables(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    let vars = v["variables"].as_array().unwrap();
    assert!(vars.iter().any(|v| v.as_str() == Some("myvar")));

    // Kill variable
    parse_ok(
        s.kill_variable(Parameters(KillVariableParams {
            name: "myvar".into(),
            notebook_id: None,
        }))
        .await,
    );

    // Verify it's gone
    let v = parse_ok(
        s.list_variables(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    let vars = v["variables"].as_array().unwrap();
    assert!(!vars.iter().any(|v| v.as_str() == Some("myvar")));
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn restart_session_works() {
    let s = build_server();

    // Ensure session is running first
    parse_ok(
        s.evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "1 + 1".into(),
            notebook_id: None,
        }))
        .await,
    );

    let v = parse_ok(
        s.restart_session(Parameters(NotebookIdParam {
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["restarted"], true);
}

// ── Safety gate tests ─────────────────────────────────────────────────

#[tokio::test]
async fn evaluate_expression_rejects_dangerous_input() {
    let s = build_server();
    let result = s
        .evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "system(\"ls\")".into(),
            notebook_id: None,
        }))
        .await;
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("Dangerous function(s) blocked"));
    assert!(msg.contains("system"));
}

#[tokio::test]
async fn evaluate_expression_allows_safe_input() {
    let s = build_server();
    // Safe expression should not be blocked (will fail because no Maxima, but not from safety)
    let result = s
        .evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "integrate(x^2, x)".into(),
            notebook_id: None,
        }))
        .await;
    // It will error due to no Maxima session, but NOT from safety gate
    if let Err(msg) = &result {
        assert!(!msg.contains("Dangerous function"));
    }
}

#[tokio::test]
async fn evaluate_expression_allows_dangerous_with_flag() {
    let s = build_server_allow_dangerous();
    // Should not be blocked by safety (will fail from no Maxima, not safety)
    let result = s
        .evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "system(\"ls\")".into(),
            notebook_id: None,
        }))
        .await;
    if let Err(msg) = &result {
        assert!(!msg.contains("Dangerous function"), "Should not be blocked by safety: {msg}");
    }
}

#[tokio::test]
async fn run_cell_rejects_dangerous_in_headless() {
    let s = build_server();

    // Add a cell with dangerous content
    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some("system(\"ls\")".into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );
    let cell_id = v["cell_id"].as_str().unwrap().to_string();

    // Run it — should be blocked
    let result = s
        .run_cell(Parameters(CellIdParams {
            cell_id,
            notebook_id: None,
        }))
        .await;
    // New notebooks start trusted, but standalone ignores the flag
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("Dangerous function(s) blocked"));
    assert!(msg.contains("--allow-dangerous"), "{msg}");
}

#[tokio::test]
async fn run_cell_allows_safe_input_in_headless() {
    let s = build_server();

    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some("1 + 1".into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );
    let cell_id = v["cell_id"].as_str().unwrap().to_string();

    // Should not be blocked by safety (will fail from no Maxima, not safety)
    let result = s
        .run_cell(Parameters(CellIdParams {
            cell_id,
            notebook_id: None,
        }))
        .await;
    if let Err(msg) = &result {
        assert!(!msg.contains("Dangerous function"), "Should not be blocked by safety: {msg}");
    }
}

#[tokio::test]
async fn evaluate_expression_allows_known_package_load() {
    let s = build_server();
    // load("distrib") is a known package — should not be blocked
    let result = s
        .evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "load(\"distrib\")".into(),
            notebook_id: None,
        }))
        .await;
    if let Err(msg) = &result {
        assert!(!msg.contains("Dangerous function"), "Known package should not be blocked: {msg}");
    }
}

#[tokio::test]
async fn evaluate_expression_blocks_unknown_load() {
    let s = build_server();
    let result = s
        .evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "load(\"/tmp/evil.mac\")".into(),
            notebook_id: None,
        }))
        .await;
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("Dangerous function(s) blocked"));
    assert!(msg.contains("load"));
}

// ── run_all_cells status reporting ───────────────────────────────────

use crate::server::{run_all_summary, summarize_cell_run, CellRunStatus};

fn build_server_connected() -> AximarMcpServer {
    let process_sink_factory: crate::server::ProcessSinkFactory =
        Arc::new(|_id, capture| capture.clone() as Arc<dyn aximar_core::maxima::output::OutputSink>);
    AximarMcpServer::new_connected(
        Arc::new(Mutex::new(NotebookRegistry::new())),
        Arc::new(Catalog::load()),
        Arc::new(PackageCatalog::load()),
        local_settings(),
        process_sink_factory,
        Arc::new(|_, _| {}),
        Arc::new(|_, _| {}),
        Arc::new(|_, _| {}),
    )
}

async fn add_code_cell(s: &AximarMcpServer, input: &str) -> String {
    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some(input.into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );
    v["cell_id"].as_str().unwrap().to_string()
}

fn ok_json(v: serde_json::Value) -> Result<String, String> {
    Ok(v.to_string())
}

#[test]
fn summarize_needs_trust_is_not_success() {
    let (status, entry) = summarize_cell_run(
        "c1",
        &ok_json(serde_json::json!({
            "cell_id": "c1",
            "needs_notebook_trust": true,
            "dangerous_functions": ["load"],
            "message": "Notebook is untrusted.",
        })),
    );
    assert_eq!(status, CellRunStatus::NeedsTrust);
    assert!(status.stops_run());
    assert_eq!(entry["status"], "needs_trust");
    assert_eq!(entry["success"], false);
    assert_eq!(entry["dangerous_functions"], serde_json::json!(["load"]));
}

#[test]
fn summarize_is_error_is_not_success() {
    let (status, entry) = summarize_cell_run(
        "c2",
        &ok_json(serde_json::json!({
            "cell_id": "c2",
            "execution_count": 3,
            "text_output": "inpart: invalid index 4 of list or matrix.\n",
            "error": null,
            "is_error": true,
        })),
    );
    assert_eq!(status, CellRunStatus::Error);
    assert!(status.stops_run());
    assert_eq!(entry["status"], "error");
    assert_eq!(entry["success"], false);
    assert_eq!(entry["error"], "inpart: invalid index 4 of list or matrix.");
}

#[test]
fn summarize_prefers_error_field_when_set() {
    let (_, entry) = summarize_cell_run(
        "c2",
        &ok_json(serde_json::json!({
            "execution_count": 1,
            "text_output": "",
            "error": "Evaluation timed out",
            "is_error": true,
        })),
    );
    assert_eq!(entry["error"], "Evaluation timed out");
}

#[test]
fn summarize_clean_and_empty_cells_succeed() {
    let (status, entry) = summarize_cell_run(
        "c3",
        &ok_json(serde_json::json!({ "execution_count": 4, "is_error": false, "error": null })),
    );
    assert_eq!(status, CellRunStatus::Ok);
    assert_eq!(entry["success"], true);
    assert_eq!(entry["execution_count"], 4);

    let (status, entry) = summarize_cell_run(
        "c4",
        &ok_json(serde_json::json!({ "message": "Empty cell — nothing to execute" })),
    );
    assert_eq!(status, CellRunStatus::Empty);
    assert!(!status.stops_run());
    assert_eq!(entry["success"], true);
}

#[test]
fn summarize_run_cell_err_is_failed() {
    let (status, entry) = summarize_cell_run("c5", &Err("Dangerous function(s) blocked: system".into()));
    assert_eq!(status, CellRunStatus::Failed);
    assert!(status.stops_run());
    assert_eq!(entry["success"], false);
    assert!(entry["error"].as_str().unwrap().contains("blocked"));
}

#[test]
fn run_all_summary_reports_stop() {
    let results = vec![
        serde_json::json!({ "cell_id": "a", "status": "ok", "success": true }),
        serde_json::json!({ "cell_id": "b", "status": "needs_trust", "success": false }),
        serde_json::json!({ "cell_id": "c", "status": "not_run", "success": false }),
    ];
    let v = run_all_summary(3, results, Some(("b".into(), CellRunStatus::NeedsTrust)));
    assert_eq!(v["success"], false);
    assert_eq!(v["cells_total"], 3);
    assert_eq!(v["cells_run"], 1);
    assert_eq!(v["cells_not_run"], 2);
    assert_eq!(v["stopped_at"], "b");
    assert_eq!(v["stop_reason"], "needs_trust");
    assert!(v["message"].as_str().unwrap().contains("untrusted"));
}

#[test]
fn run_all_summary_all_ok() {
    let results = vec![serde_json::json!({ "cell_id": "a", "status": "ok", "success": true })];
    let v = run_all_summary(1, results, None);
    assert_eq!(v["success"], true);
    assert_eq!(v["cells_run"], 1);
    assert_eq!(v["cells_not_run"], 0);
    assert!(v["stopped_at"].is_null());
    assert!(v["stop_reason"].is_null());
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn run_all_cells_stops_at_untrusted_load_in_connected_mode() {
    let s = build_server_connected();
    let first = add_code_cell(&s, "a: 1$").await;
    // A local file load is not a known package, so the safety scan flags it.
    let load_cell = add_code_cell(&s, "load(\"my_helpers.mac\")$").await;
    let after = add_code_cell(&s, "a + 1;").await;
    // New notebooks start trusted; opened files start untrusted.
    {
        let ctx = s.core.resolve_context(None).await.unwrap();
        ctx.notebook.lock().await.set_trusted(false);
    }

    let v = parse_ok(s.run_all_cells(Parameters(NotebookIdParam { notebook_id: None })).await);
    let results = v["results"].as_array().unwrap();
    let by_id = |id: &str| results.iter().find(|r| r["cell_id"] == id).unwrap().clone();

    assert_eq!(v["success"], false, "{v:#}");
    assert_eq!(v["stopped_at"], load_cell.as_str());
    assert_eq!(v["stop_reason"], "needs_trust");
    assert_eq!(by_id(&first)["status"], "ok");
    let skipped = by_id(&load_cell);
    assert_eq!(skipped["status"], "needs_trust");
    assert_eq!(skipped["success"], false);
    assert!(skipped["dangerous_functions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f == "load"));
    assert_eq!(by_id(&after)["status"], "not_run");
    assert_eq!(by_id(&after)["success"], false);
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn run_all_cells_runs_dangerous_cell_once_notebook_trusted() {
    let s = build_server_connected();
    add_code_cell(&s, "system(\"true\")$").await;
    add_code_cell(&s, "1 + 1;").await;
    {
        let ctx = s.core.resolve_context(None).await.unwrap();
        ctx.notebook.lock().await.set_trusted(true);
    }

    let v = parse_ok(s.run_all_cells(Parameters(NotebookIdParam { notebook_id: None })).await);
    assert_eq!(v["success"], true, "{v:#}");
    assert!(v["stopped_at"].is_null());
    for r in v["results"].as_array().unwrap() {
        assert_eq!(r["success"], true, "{r:#}");
    }
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn run_all_cells_stops_at_maxima_error() {
    let s = build_server();
    let first = add_code_cell(&s, "l: [1, 2, 3]$").await;
    let bad = add_code_cell(&s, "inpart(l, 4);").await;
    let after = add_code_cell(&s, "b: 5;").await;

    let v = parse_ok(s.run_all_cells(Parameters(NotebookIdParam { notebook_id: None })).await);
    let results = v["results"].as_array().unwrap();
    let by_id = |id: &str| results.iter().find(|r| r["cell_id"] == id).unwrap().clone();

    assert_eq!(v["success"], false, "{v:#}");
    assert_eq!(v["stopped_at"], bad.as_str());
    assert_eq!(v["stop_reason"], "error");
    assert_eq!(by_id(&first)["status"], "ok");
    let errored = by_id(&bad);
    assert_eq!(errored["status"], "error");
    assert_eq!(errored["success"], false);
    assert!(
        errored["error"].as_str().unwrap().contains("inpart"),
        "error text should name the failing call: {errored:#}"
    );
    assert_eq!(by_id(&after)["status"], "not_run");

    // The skipped cell really did not run: `b` stays unbound.
    let v = parse_ok(
        s.evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "b;".into(),
            notebook_id: None,
        }))
        .await,
    );
    assert_eq!(v["latex"], "b", "later cell should not have run: {v:#}");
}

// ── Label references ──────────────────────────────────────────────────

/// Add a code cell at the end of the notebook and run it.
async fn add_and_run(s: &AximarMcpServer, input: &str) -> serde_json::Value {
    let v = parse_ok(
        s.add_cell(Parameters(AddCellParams {
            cell_type: Some("code".into()),
            input: Some(input.into()),
            after_cell_id: None,
            notebook_id: None,
        }))
        .await,
    );
    let cell_id = v["cell_id"].as_str().unwrap().to_string();
    parse_ok(
        s.run_cell(Parameters(CellIdParams {
            cell_id,
            notebook_id: None,
        }))
        .await,
    )
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn label_references_follow_the_gutter_numbers() {
    let s = build_server();
    add_and_run(&s, "a: 10;").await; // Out[1]
    add_and_run(&s, "b: 20$").await; // Out[2]
    add_and_run(&s, "1/0;").await; // In[3], an error with no output
    let v = add_and_run(&s, "[%o1, %o2, %];").await;
    assert!(v["latex"].as_str().unwrap().contains("10 , 20 , 20"), "{v:#}");

    // % in a later statement is Maxima's own: the statement before it
    let v = add_and_run(&s, "x: 5; % + 1;").await;
    assert_eq!(v["latex"], "6", "{v:#}");

    // Out[3] has no value: the cell reports it instead of reading %o3
    let v = add_and_run(&s, "%o3 + 1;").await;
    assert_eq!(v["is_error"], true, "{v:#}");
    assert!(v["error"].as_str().unwrap().starts_with("Out[3] (%o3) has no value"), "{v:#}");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn labels_from_before_a_restart_have_no_value() {
    let s = build_server();
    add_and_run(&s, "a: 10;").await; // Out[1]
    parse_ok(
        s.restart_session(Parameters(NotebookIdParam { notebook_id: None }))
            .await,
    );

    let v = add_and_run(&s, "%o1;").await;
    assert_eq!(v["is_error"], true, "{v:#}");
    assert!(v["error"].as_str().unwrap().starts_with("Out[1] (%o1) has no value"), "{v:#}");

    let v = add_and_run(&s, "% + 1;").await;
    assert_eq!(v["is_error"], true, "{v:#}");
    assert!(v["error"].as_str().unwrap().starts_with("% refers to the previous cell"), "{v:#}");

    // Cells run after the restart work as usual
    add_and_run(&s, "p: 7;").await;
    let v = add_and_run(&s, "% + 1;").await;
    assert_eq!(v["latex"], "8", "{v:#}");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn restart_then_run_all_numbers_cells_from_one() {
    let s = build_server();
    let mut ids = Vec::new();
    for input in ["a: 10;", "%o1 + 1;", "%i1;"] {
        let v = parse_ok(
            s.add_cell(Parameters(AddCellParams {
                cell_type: Some("code".into()),
                input: Some(input.into()),
                after_cell_id: None,
                notebook_id: None,
            }))
            .await,
        );
        ids.push(v["cell_id"].as_str().unwrap().to_string());
    }

    let all = || async { parse_ok(s.run_all_cells(Parameters(NotebookIdParam { notebook_id: None })).await) };
    let v = all().await;
    assert_eq!(v["success"], true, "{v:#}");
    parse_ok(s.restart_session(Parameters(NotebookIdParam { notebook_id: None })).await);
    let v = all().await;
    assert_eq!(v["success"], true, "{v:#}");

    let cell = |id: String| async {
        parse_ok(s.get_cell(Parameters(CellIdParams { cell_id: id, notebook_id: None })).await)
    };
    let second = cell(ids[1].clone()).await;
    assert_eq!(second["output"]["execution_count"], 2, "{second:#}");
    assert_eq!(second["output"]["latex"], "11", "{second:#}");
    let third = cell(ids[2].clone()).await;
    assert_eq!(third["output"]["execution_count"], 3, "{third:#}");
    assert_eq!(third["output"]["latex"], "a:10", "{third:#}");
}

// ── Lifecycle events in connected mode ────────────────────────────────

#[tokio::test]
async fn connected_mode_reports_create_and_switch() {
    let events = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let recorded = events.clone();
    let process_sink_factory: crate::server::ProcessSinkFactory =
        Arc::new(|_id, capture| capture.clone() as Arc<dyn aximar_core::maxima::output::OutputSink>);
    let s = AximarMcpServer::new_connected(
        Arc::new(Mutex::new(NotebookRegistry::new())),
        Arc::new(Catalog::load()),
        Arc::new(PackageCatalog::load()),
        local_settings(),
        process_sink_factory,
        Arc::new(|_, _| {}),
        Arc::new(move |id: &str, event: &str| {
            recorded.lock().unwrap().push(format!("{id} {event}"));
        }),
        Arc::new(|_, _| {}),
    );

    let v = parse_ok(s.create_notebook().await);
    let id = v["notebook_id"].as_str().unwrap().to_string();
    parse_ok(
        s.switch_notebook(Parameters(SwitchNotebookParams { notebook_id: id.clone() }))
            .await,
    );

    let seen = events.lock().unwrap().clone();
    assert_eq!(seen, vec![format!("{id} created"), format!("{id} active_changed")]);
}

// ── Effects reach the window as they happen ───────────────────────────

/// A connected-mode server that records the effect names it sends to the window.
fn build_server_recording_effects() -> (AximarMcpServer, Arc<std::sync::Mutex<Vec<String>>>) {
    let seen = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let recorded = seen.clone();
    let process_sink_factory: crate::server::ProcessSinkFactory =
        Arc::new(|_id, capture| capture.clone() as Arc<dyn aximar_core::maxima::output::OutputSink>);
    let s = AximarMcpServer::new_connected(
        Arc::new(Mutex::new(NotebookRegistry::new())),
        Arc::new(Catalog::load()),
        Arc::new(PackageCatalog::load()),
        local_settings(),
        process_sink_factory,
        Arc::new(move |_id: &str, effect: aximar_core::commands::CommandEffect| {
            recorded.lock().unwrap().push(effect.effect_name().to_string());
        }),
        Arc::new(|_, _| {}),
        Arc::new(|_, _| {}),
    );
    (s, seen)
}

/// Run `input` in a new cell and return the effects sent while it ran, and
/// the cell's final status.
async fn effects_of_run(
    s: &AximarMcpServer,
    seen: &Arc<std::sync::Mutex<Vec<String>>>,
    input: &str,
) -> (Vec<String>, String) {
    let cell_id = add_code_cell(s, input).await;
    seen.lock().unwrap().clear();
    parse_ok(
        s.run_cell(Parameters(CellIdParams { cell_id: cell_id.clone(), notebook_id: None }))
            .await,
    );
    let effects = seen.lock().unwrap().clone();
    let cell = parse_ok(s.get_cell(Parameters(CellIdParams { cell_id, notebook_id: None })).await);
    (effects, cell["status"].as_str().unwrap().to_string())
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn mcp_run_reports_running_before_the_result() {
    let (s, seen) = build_server_recording_effects();
    let (effects, status) = effects_of_run(&s, &seen, "2 + 3;").await;
    assert_eq!(effects, vec!["cell_status_updated", "cell_output_updated"]);
    assert_eq!(status, "success");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn mcp_run_reports_running_before_an_error() {
    let (s, seen) = build_server_recording_effects();
    let (effects, status) = effects_of_run(&s, &seen, "1/0;").await;
    assert_eq!(effects, vec!["cell_status_updated", "cell_output_updated"]);
    assert_eq!(status, "error");
}

// ── Session settings ──────────────────────────────────────────────────

#[tokio::test]
async fn sessions_use_the_settings_in_force_when_they_start() {
    // The settings change after the server is built, as when the user
    // changes them in the app while MCP is running
    let current = Arc::new(std::sync::Mutex::new(SessionSettings {
        backend: Backend::Local,
        maxima_path: None,
        eval_timeout: 30,
    }));
    let provided = current.clone();
    let s = AximarMcpServer::new(
        Arc::new(Mutex::new(NotebookRegistry::new())),
        Arc::new(Catalog::load()),
        Arc::new(PackageCatalog::load()),
        Arc::new(move || provided.lock().unwrap().clone()),
        false,
    );
    let missing = "/nonexistent/aximar-test/bin/maxima";
    current.lock().unwrap().maxima_path = Some(missing.into());

    let cell_id = add_code_cell(&s, "1 + 1;").await;
    let err = s
        .run_cell(Parameters(CellIdParams { cell_id, notebook_id: None }))
        .await
        .expect_err("a missing Maxima can't start");
    assert!(err.contains(missing), "error should name the path: {err}");
}

// ── One trust check in connected mode ─────────────────────────────────

async fn set_trusted(s: &AximarMcpServer, trusted: bool) {
    let ctx = s.core.resolve_context(None).await.unwrap();
    ctx.notebook.lock().await.set_trusted(trusted);
}

#[tokio::test]
async fn connected_expression_in_untrusted_notebook_is_blocked() {
    let s = build_server_connected();
    set_trusted(&s, false).await;
    let msg = s
        .evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "system(\"ls\")".into(),
            notebook_id: None,
        }))
        .await
        .expect_err("an untrusted notebook blocks system");
    assert!(msg.contains("system") && msg.contains("untrusted"), "{msg}");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn connected_expression_in_trusted_notebook_runs_system() {
    let s = build_server_connected();
    set_trusted(&s, true).await;
    let v = parse_ok(
        s.evaluate_expression(Parameters(EvaluateExpressionParams {
            expression: "system(\"true\")$ 1 + 1;".into(),
            notebook_id: None,
        }))
        .await,
    );
    assert!(v["error"].is_null(), "{v:#}");
}

#[tokio::test]
async fn connected_cell_blocked_by_trust_shows_the_banner() {
    let (s, seen) = build_server_recording_effects();
    let cell_id = add_code_cell(&s, "load(\"helpers.mac\")$").await;
    set_trusted(&s, false).await;
    seen.lock().unwrap().clear();

    let v = parse_ok(s.run_cell(Parameters(CellIdParams { cell_id, notebook_id: None })).await);
    assert_eq!(v["needs_notebook_trust"], true);
    assert_eq!(v["dangerous_functions"], serde_json::json!(["load"]));
    // The window hears about the block, and the cell never starts running
    assert_eq!(*seen.lock().unwrap(), vec!["trust_required"]);
}
