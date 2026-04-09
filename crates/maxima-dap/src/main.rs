//! Binary entry point for the Maxima DAP server.

use maxima_dap::server::DapServer;
use maxima_dap::transport::DapTransport;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize tracing to stderr (stdout is used for DAP transport)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("maxima-dap starting");

    let transport = DapTransport::stdio();
    let mut server = DapServer::new(transport);

    if let Err(e) = server.run().await {
        tracing::error!("server error: {}", e);
        std::process::exit(1);
    }
}
