//! Content-Length framed transport for the Debug Adapter Protocol.
//!
//! DAP uses the same framing as LSP: a `Content-Length: N\r\n\r\n` header
//! followed by N bytes of JSON body.

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

/// DAP transport over stdio with Content-Length framing.
pub struct DapTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
}

impl DapTransport {
    /// Create a transport connected to stdin/stdout.
    pub fn stdio() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
        }
    }

    /// Read the next DAP message from the transport.
    ///
    /// Returns `Ok(None)` on EOF (client disconnected).
    pub async fn read_message(&mut self) -> Result<Option<serde_json::Value>, TransportError> {
        // Read headers until we find Content-Length
        let mut content_length: Option<usize> = None;
        let mut header_line = String::new();

        loop {
            header_line.clear();
            let bytes_read = self
                .reader
                .read_line(&mut header_line)
                .await
                .map_err(TransportError::Io)?;

            if bytes_read == 0 {
                return Ok(None); // EOF
            }

            let trimmed = header_line.trim();
            if trimmed.is_empty() {
                // Empty line marks end of headers
                break;
            }

            if let Some(value) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    value
                        .trim()
                        .parse()
                        .map_err(|_| TransportError::InvalidHeader(trimmed.to_string()))?,
                );
            }
            // Ignore other headers (e.g. Content-Type)
        }

        let length = content_length.ok_or(TransportError::MissingContentLength)?;

        // Read exactly `length` bytes of JSON body
        let mut body = vec![0u8; length];
        self.reader
            .read_exact(&mut body)
            .await
            .map_err(TransportError::Io)?;

        let value: serde_json::Value =
            serde_json::from_slice(&body).map_err(TransportError::Json)?;

        tracing::debug!(
            "← {}",
            serde_json::to_string(&value).unwrap_or_default()
        );

        Ok(Some(value))
    }

    /// Write a DAP message to the transport.
    pub async fn write_message<T: Serialize>(&mut self, msg: &T) -> Result<(), TransportError> {
        let body = serde_json::to_string(msg).map_err(TransportError::Json)?;

        tracing::debug!("→ {}", body);

        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.writer
            .write_all(header.as_bytes())
            .await
            .map_err(TransportError::Io)?;
        self.writer
            .write_all(body.as_bytes())
            .await
            .map_err(TransportError::Io)?;
        self.writer.flush().await.map_err(TransportError::Io)?;

        Ok(())
    }
}

/// Transport-level errors.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("missing Content-Length header")]
    MissingContentLength,

    #[error("invalid header: {0}")]
    InvalidHeader(String),
}
