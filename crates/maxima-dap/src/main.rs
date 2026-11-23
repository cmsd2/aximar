//! Binary entry point for the Maxima DAP server.

use std::io::IsTerminal;

use clap::Parser;
use maxima_dap::server::DapServer;
use maxima_dap::transport::DapTransport;
use tracing_subscriber::EnvFilter;

/// Debug adapter for Maxima. An editor starts it and speaks the Debug
/// Adapter Protocol over stdin and stdout.
#[derive(Parser)]
#[command(
    version,
    after_help = "Environment:\n  MAXIMA_DAP_LOG  write the log to this file instead of stderr\n  RUST_LOG        log filter, e.g. debug or maxima_dap=trace (default info)"
)]
struct Cli {
    /// Accepted for editors that pass it; stdio is the only transport
    #[arg(long, hide = true)]
    stdio: bool,
}

#[tokio::main]
async fn main() {
    Cli::parse();
    if std::io::stdin().is_terminal() {
        eprintln!(
            "maxima-dap speaks the Debug Adapter Protocol on stdin and stdout; \
             an editor starts it. See maxima-dap --help."
        );
    }

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
