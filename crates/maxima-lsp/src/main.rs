use std::io::IsTerminal;

use clap::Parser;
use tower_lsp::{LspService, Server};

/// Language server for Maxima. An editor starts it and speaks the Language
/// Server Protocol over stdin and stdout.
#[derive(Parser)]
#[command(
    version,
    after_help = "Environment:\n  RUST_LOG  log filter, e.g. debug or maxima_lsp=trace (default info)"
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
            "maxima-lsp speaks the Language Server Protocol on stdin and stdout; \
             an editor starts it. See maxima-lsp --help."
        );
    }

    // Init tracing to stderr (stdout is LSP JSON-RPC transport)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting maxima-lsp v{}", env!("CARGO_PKG_VERSION"));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        maxima_lsp::server::MaximaLsp::new(client)
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
