//! Binary entry point for the Maxima DAP server.

use maxima_dap::server::DapServer;
use maxima_dap::transport::DapTransport;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize tracing. When MAXIMA_DAP_LOG is set, write to that file
    // (useful when launched by VS Code, where stderr is not easily visible).
    // Otherwise write to stderr.
    let log_file = std::env::var("MAXIMA_DAP_LOG").ok();
    if let Some(ref path) = log_file {
        let file = std::fs::File::create(path).expect("failed to create log file");
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_writer(std::io::stderr)
            .init();
    }

    tracing::info!("maxima-dap starting");

    let transport = DapTransport::stdio();
    let mut server = DapServer::new(transport);

    if let Err(e) = server.run().await {
        tracing::error!("server error: {}", e);
        std::process::exit(1);
    }
}
