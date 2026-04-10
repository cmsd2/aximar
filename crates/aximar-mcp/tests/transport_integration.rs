//! Integration tests for MCP stdio and HTTP transports.
//!
//! These tests verify end-to-end MCP handshakes over both transports and that
//! responses are well-formed JSON with no stray empty lines.
//! They require the `aximar-mcp` binary to be built (`cargo build -p aximar-mcp`).
//! Run with: `cargo test -p aximar-mcp --test transport_integration -- --ignored`

use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

// ---------- Helpers ----------

fn binary_path() -> String {
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    path.push("aximar-mcp");
    path.to_string_lossy().to_string()
}

fn initialize_request(id: u64) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" }
        }
    })
    .to_string()
}

fn initialized_notification() -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    })
    .to_string()
}

fn tool_call_request(id: u64, name: &str, args: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": name, "arguments": args }
    })
    .to_string()
}

/// Send a message over stdio and read one response line.
async fn stdio_send_recv(
    stdin: &mut tokio::process::ChildStdin,
    reader: &mut BufReader<tokio::process::ChildStdout>,
    msg: &str,
) -> String {
    stdin
        .write_all(format!("{msg}\n").as_bytes())
        .await
        .unwrap();
    stdin.flush().await.unwrap();

    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line))
        .await
        .expect("timeout reading stdio response")
        .expect("io error reading stdio response");
    line
}

/// Send a notification (no response expected).
async fn stdio_send_notification(stdin: &mut tokio::process::ChildStdin, msg: &str) {
    stdin
        .write_all(format!("{msg}\n").as_bytes())
        .await
        .unwrap();
    stdin.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
}

/// Assert a line is non-empty valid JSON and return the parsed value.
fn assert_json(line: &str, context: &str) -> serde_json::Value {
    let trimmed = line.trim();
    assert!(
        !trimmed.is_empty(),
        "{context}: empty line — stray newline detected"
    );
    serde_json::from_str(trimmed)
        .unwrap_or_else(|e| panic!("{context}: not valid JSON: {trimmed:?}\nerror: {e}"))
}

// ---------- Stdio Tests ----------

/// Full stdio round-trip: initialize, notification, tool call.
/// Verifies every response is non-empty valid JSON with correct ids.
#[tokio::test]
#[ignore]
async fn stdio_round_trip() {
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn aximar-mcp");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let line = stdio_send_recv(&mut stdin, &mut reader, &initialize_request(1)).await;
    let resp = assert_json(&line, "initialize");
    assert_eq!(resp["id"], 1);
    assert!(resp["result"]["serverInfo"]["name"].is_string());

    // Initialized notification
    stdio_send_notification(&mut stdin, &initialized_notification()).await;

    // Tool call
    let line = stdio_send_recv(
        &mut stdin,
        &mut reader,
        &tool_call_request(2, "get_session_status", serde_json::json!({})),
    )
    .await;
    let resp = assert_json(&line, "tool call");
    assert_eq!(resp["id"], 2);
    assert!(resp["result"]["content"].is_array());

    drop(stdin);
    let _ = child.kill().await;
}

// ---------- HTTP Tests ----------

/// Spawn aximar-mcp in HTTP mode and return (child, base_url).
/// Drains stderr in the background to prevent SIGPIPE.
async fn spawn_http_server(extra_args: &[&str]) -> (tokio::process::Child, String) {
    let mut cmd = Command::new(binary_path());
    cmd.args(["--http", "--port", "0"])
        .args(extra_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn aximar-mcp --http");

    let stderr = child.stderr.take().unwrap();
    let mut reader = BufReader::new(stderr);
    let url;

    // Read stderr until we see the listen URL
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let mut line = String::new();
        match tokio::time::timeout_at(deadline, reader.read_line(&mut line)).await {
            Ok(Ok(0)) => panic!("aximar-mcp exited before printing listen address"),
            Ok(Ok(_)) => {
                if let Some(start) = line.find("http://") {
                    let rest = &line[start..];
                    let end = rest
                        .find(|c: char| c.is_whitespace())
                        .unwrap_or(rest.len());
                    url = rest[..end].to_string();
                    break;
                }
            }
            Ok(Err(e)) => panic!("stderr read error: {e}"),
            Err(_) => panic!("timeout waiting for aximar-mcp to start"),
        }
    }

    // Keep draining stderr so the server doesn't get SIGPIPE
    tokio::spawn(async move {
        let mut buf = String::new();
        loop {
            buf.clear();
            match reader.read_line(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    (child, url)
}

/// MCP HTTP client helper that tracks session ID and handles SSE responses.
struct McpHttpClient {
    client: reqwest::Client,
    base_url: String,
    session_id: Option<String>,
}

impl McpHttpClient {
    fn new(base_url: String) -> Self {
        Self {
            client: Self::build_client(None),
            base_url,
            session_id: None,
        }
    }

    fn with_token(base_url: String, token: &str) -> Self {
        Self {
            client: Self::build_client(Some(token)),
            base_url,
            session_id: None,
        }
    }

    fn build_client(token: Option<&str>) -> reqwest::Client {
        let mut builder = reqwest::Client::builder()
            .pool_max_idle_per_host(0)
            .http1_only();
        if let Some(token) = token {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {token}").parse().unwrap(),
            );
            builder = builder.default_headers(headers);
        }
        builder.build().unwrap()
    }

    async fn send(&mut self, body: &str) -> reqwest::Response {
        let mut req = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("Connection", "close");

        if let Some(ref sid) = self.session_id {
            req = req.header("Mcp-Session-Id", sid);
        }

        let resp = req
            .body(body.to_string())
            .send()
            .await
            .expect("HTTP request failed");

        if let Some(sid) = resp.headers().get("mcp-session-id") {
            self.session_id = Some(sid.to_str().unwrap().to_string());
        }

        resp
    }

    /// Send a request and extract the JSON-RPC response, handling SSE streams.
    async fn send_json(&mut self, body: &str) -> (u16, String) {
        let resp = self.send(body).await;
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("text/event-stream") {
            let mut stream = resp.bytes_stream();
            use futures_util::StreamExt;
            let mut buffer = String::new();

            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            loop {
                match tokio::time::timeout_at(deadline, stream.next()).await {
                    Ok(Some(Ok(chunk))) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].to_string();
                            buffer = buffer[pos + 1..].to_string();

                            if let Some(data) = line.strip_prefix("data:") {
                                let data = data.trim();
                                if !data.is_empty() {
                                    if let Ok(v) =
                                        serde_json::from_str::<serde_json::Value>(data)
                                    {
                                        if v.get("result").is_some() || v.get("error").is_some() {
                                            return (status, data.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(Some(Err(e))) => panic!("SSE stream error: {e}"),
                    Ok(None) | Err(_) => break,
                }
            }
            (status, String::new())
        } else {
            let body_text = resp.text().await.unwrap();
            (status, body_text)
        }
    }
}

/// Full HTTP round-trip: initialize, notification, tool call.
#[tokio::test]
#[ignore]
async fn http_round_trip() {
    let (mut child, base_url) = spawn_http_server(&["--no-auth"]).await;
    let mut client = McpHttpClient::new(base_url);

    // Initialize
    let (status, body) = client.send_json(&initialize_request(1)).await;
    assert_eq!(status, 200);
    let resp = assert_json(&body, "initialize");
    assert_eq!(resp["id"], 1);
    assert!(resp["result"]["serverInfo"]["name"].is_string());

    // Initialized notification
    let resp = client.send(&initialized_notification()).await;
    assert!(resp.status().is_success());

    // Tool call
    let (status, body) = client
        .send_json(&tool_call_request(2, "get_session_status", serde_json::json!({})))
        .await;
    assert_eq!(status, 200);
    let resp = assert_json(&body, "tool call");
    assert_eq!(resp["id"], 2);

    child.kill().await.unwrap();
}

/// Verify bearer token auth: 401 without/wrong token, 200 with correct token.
#[tokio::test]
#[ignore]
async fn http_auth_required() {
    let (mut child, base_url) = spawn_http_server(&["--token", "test-secret-token"]).await;

    // No token → 401
    let no_auth = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .http1_only()
        .build()
        .unwrap();
    let resp = no_auth
        .post(&base_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(initialize_request(1))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "expected 401 without auth");

    // Wrong token → 401
    let resp = no_auth
        .post(&base_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", "Bearer wrong-token")
        .body(initialize_request(1))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "expected 401 with wrong token");

    // Correct token → 200
    let mut authed = McpHttpClient::with_token(base_url, "test-secret-token");
    let (status, _) = authed.send_json(&initialize_request(1)).await;
    assert_eq!(status, 200, "expected 200 with correct token");

    child.kill().await.unwrap();
}
