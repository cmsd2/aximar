mod config;

use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::ServiceExt;

use aximar_core::catalog::docs::Docs;
use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::registry::NotebookRegistry;

use aximar_mcp::server::AximarMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init tracing to stderr (stdout is used for MCP stdio transport)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting aximar-mcp server v{}", env!("CARGO_PKG_VERSION"));

    // Load catalog, docs, and packages
    let catalog = Arc::new(Catalog::load());
    let docs = Arc::new(Docs::load());
    let packages = Arc::new(PackageCatalog::load());
    tracing::info!("Loaded function catalog, documentation, and packages");

    // Read configuration from environment
    let backend = config::backend_from_env();
    let maxima_path = config::maxima_path_from_env();
    let eval_timeout = config::eval_timeout_from_env();

    // Create registry with one default notebook
    let registry = Arc::new(Mutex::new(NotebookRegistry::new()));

    // Build MCP server
    let server = AximarMcpServer::new(
        registry,
        catalog,
        docs,
        packages,
        backend,
        maxima_path,
        eval_timeout,
    );

    // Serve over stdio
    tracing::info!("Serving MCP over stdio");
    let transport = rmcp::transport::io::stdio();

    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
