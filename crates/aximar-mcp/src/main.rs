use std::sync::Arc;
use tokio::sync::Mutex;

use clap::{Parser, Subcommand};
use rmcp::handler::server::ServerHandler;
use rmcp::ServiceExt;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::registry::NotebookRegistry;

use aximar_mcp::server::{AximarMcpServer, ServerCore};
use aximar_mcp::simple_server::SimpleMcpServer;

/// Maxima CAS MCP server and notebook runner.
#[derive(Parser)]
#[command(name = "aximar-mcp", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Use HTTP transport instead of stdio.
    #[arg(long)]
    http: bool,

    /// Notebook mode (full tool set).
    #[arg(long)]
    notebook: bool,

    /// HTTP port.
    #[arg(long, default_value_t = 19542)]
    port: u16,

    /// HTTP bind address.
    #[arg(long, default_value = "127.0.0.1")]
    address: String,

    /// Auth token for HTTP (auto-generated if omitted).
    #[arg(long)]
    token: Option<String>,

    /// Disable HTTP authentication.
    #[arg(long)]
    no_auth: bool,

    /// Allow dangerous functions (system, batch, etc.) without approval.
    #[arg(long)]
    allow_dangerous: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Execute all cells in a notebook and save with outputs.
    Run {
        /// Input notebook file (.macnb or .ipynb).
        path: String,

        /// Output path [default: overwrite input].
        #[arg(short)]
        o: Option<String>,

        /// Allow dangerous functions (system, batch, etc.).
        #[arg(long)]
        allow_dangerous: bool,
    },
}

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

    let cli = Cli::parse();

    tracing::info!("Starting aximar-mcp v{}", env!("CARGO_PKG_VERSION"));

    // Subcommand: `run` for batch execution
    if let Some(Command::Run { path, o, allow_dangerous }) = cli.command {
        return aximar_mcp::batch::run(&path, o.as_deref(), allow_dangerous).await;
    }

    if cli.allow_dangerous {
        tracing::warn!("--allow-dangerous: dangerous functions (system, batch, etc.) will be allowed without approval");
    }

    // Load catalog (wraps doc-index) and packages
    let catalog = Arc::new(Catalog::load());
    let packages = Arc::new(PackageCatalog::load());
    tracing::info!("Loaded function catalog and packages");

    // Read configuration from environment
    let backend = aximar_mcp::config::backend_from_env();
    let maxima_path = aximar_mcp::config::maxima_path_from_env();
    let eval_timeout = aximar_mcp::config::eval_timeout_from_env();

    // Create registry with one default notebook/session
    let registry = Arc::new(Mutex::new(NotebookRegistry::new()));

    // Build server core (shared state)
    let core = ServerCore::new(
        registry,
        catalog,
        packages,
        backend,
        maxima_path,
        eval_timeout,
        cli.allow_dangerous,
    );

    if cli.notebook {
        tracing::info!("Running in notebook mode (full tool set)");
        let server = AximarMcpServer::from_core(core);
        if cli.http {
            serve_http(server, &cli.address, cli.port, cli.token, cli.no_auth).await
        } else {
            serve_stdio(server).await
        }
    } else {
        tracing::info!("Running in simple mode (session-oriented tools)");
        let server = SimpleMcpServer::new(core);
        if cli.http {
            serve_http(server, &cli.address, cli.port, cli.token, cli.no_auth).await
        } else {
            serve_stdio(server).await
        }
    }
}

async fn serve_stdio<S>(server: S) -> anyhow::Result<()>
where
    S: ServerHandler + Send + Sync + 'static,
{
    tracing::info!("Serving MCP over stdio");
    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

async fn serve_http<S>(
    server: S,
    address: &str,
    port: u16,
    token: Option<String>,
    no_auth: bool,
) -> anyhow::Result<()>
where
    S: ServerHandler + Clone + Send + Sync + 'static,
{
    use tokio_util::sync::CancellationToken;

    // Determine auth token
    let auth_token = if no_auth {
        None
    } else if let Some(t) = token {
        Some(t)
    } else {
        // Auto-generate a random token
        use rand::Rng;
        let bytes: [u8; 32] = rand::rng().random();
        let t: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Some(t)
    };

    let bind_addr = format!("{address}:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    let local_addr = listener.local_addr()?;

    // Machine-readable JSON on stdout (safe: HTTP mode doesn't use stdout for MCP protocol)
    {
        let token_json = match &auth_token {
            Some(t) => format!("\"{}\"", t),
            None => "null".to_string(),
        };
        println!(
            "{{\"port\":{},\"token\":{}}}",
            local_addr.port(),
            token_json
        );
    }

    if let Some(ref token) = auth_token {
        tracing::info!("MCP HTTP server listening on http://{local_addr}/mcp (token: {token})");
        eprintln!("Listening on http://{local_addr}/mcp");
        eprintln!("Token: {token}");
    } else {
        tracing::info!("MCP HTTP server listening on http://{local_addr}/mcp (no auth)");
        eprintln!("Listening on http://{local_addr}/mcp (no auth)");
    }

    let ct = CancellationToken::new();
    let ct2 = ct.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
        ct2.cancel();
    });

    aximar_mcp::http::serve_mcp_http(server, listener, auth_token, ct).await
}
