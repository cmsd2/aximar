use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
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
