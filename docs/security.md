# Security Model

Aximar is a desktop application that opens user-provided notebook files and executes their contents via a local Maxima process. This document describes the threat model, known attack surfaces, and current mitigations.

## Threat classes

### 1. Malicious notebook files

The primary threat is a user opening an `.axm` notebook file from an untrusted source. Notebook files are JSON containing cell inputs and metadata. Opening a notebook **does not** automatically execute any cells — the user must explicitly run them. This is the most important mitigation and must be preserved.

However, social engineering ("run all cells to see the result") can still lead to exploitation once cells are executed.

### 2. Maxima command execution

Maxima is a full programming environment with access to the host system. The `system()` function executes arbitrary shell commands. There is no sandbox around the Maxima process.

**Impact:** A cell containing `system("rm -rf ~")$` would execute if the user runs it.

**Current mitigation:** None beyond requiring explicit execution for the Local backend. The Docker backend provides partial sandboxing:
- `--network none`: no network access from the container
- `--memory 512m`: memory limit prevents resource exhaustion
- Non-root user (`maxima`) inside the container
- Volume mount restricted to a dedicated temp directory
- Custom seccomp profile: GCL (the Lisp runtime used by Ubuntu's Maxima package) calls `personality(ADDR_NO_RANDOMIZE | READ_IMPLIES_EXEC)` which Docker's default seccomp profile blocks. Rather than disabling seccomp entirely, a custom profile based on Docker's default is used that adds only the three specific `personality` argument values GCL requires (0x40000, 0x400000, 0x440000). All other seccomp restrictions remain in effect.

Possible future mitigations:
- Warn before executing cells containing `system()`, `load()`, `batchload()`, or similar dangerous functions
- Run Maxima in a restricted sandbox (e.g. seccomp on Linux, sandbox-exec on macOS)
- Display untrusted-file warnings when opening notebooks not created by the user

### 3. Cross-site scripting (XSS)

Even within a Tauri app, XSS is dangerous because JavaScript runs in the app's WebView context and can invoke Tauri IPC commands.

#### SVG output injection

**File:** `src/components/CellOutput.tsx`

Plot output SVGs originate from files that Maxima writes during plot operations. The Rust parser reads SVG files from paths extracted from Maxima's text output (`src-tauri/src/maxima/parser.rs`).

**Current mitigation:**
- SVG content is sanitized via `sanitizeSvg()` which strips dangerous elements (`<script>`, `<foreignObject>`, `<iframe>`), event handler attributes, and data URIs
- Sanitized SVGs are rendered as `<img src="blob:...">` — the `<img>` context natively blocks all script execution and event handlers
- SVG file paths are canonicalized and validated: must have a `.svg` extension and reside within the system temp directory (`is_safe_svg_path()` in `parser.rs`)
- CSP blocks inline scripts as an additional layer

#### KaTeX trust mode

**Files:** `src/components/KatexOutput.tsx`, `src/components/MathText.tsx`

**Current mitigation:** KaTeX is configured with `trust: false` in both components, which blocks macros like `\href{javascript:...}{text}` that could otherwise execute JavaScript when clicked.

#### Markdown rendering

**File:** `src/components/MarkdownCell.tsx`

Markdown cells are rendered with `react-markdown`, which sanitizes HTML by default. This is currently safe, but the lack of explicit sanitization configuration means a future library update changing defaults could introduce a vulnerability.

**Recommendation:** Explicitly configure `rehype-sanitize` to make the safe behaviour intentional rather than incidental.

### 4. Arbitrary file access

#### File read via SVG path injection

**File:** `src-tauri/src/maxima/parser.rs`

The parser extracts file paths from Maxima's text output using a regex and reads them with `fs::read_to_string`.

**Current mitigation:** The `is_safe_svg_path()` function canonicalizes extracted paths (resolving symlinks and `..`) and validates that they have a `.svg` extension and reside within the system temp directory. For the Docker backend, the Docker host temp directory (`aximar-docker`) is also allowed.

#### File write via `write_plot_svg` command

**File:** `src-tauri/src/commands/plot.rs`

The `write_plot_svg` Tauri command accepts a file path and writes content to it. The UI gates this behind a save dialog.

**Current mitigation:** The backend rejects paths containing `..` segments (directory traversal) and enforces a `.svg` extension.

### 5. MCP server (connected mode)

When enabled, the embedded MCP server listens on a local TCP port (default `127.0.0.1:19542`) and exposes tools that can read/write notebook cells, evaluate Maxima expressions, and manage sessions.

**Impact:** An unauthenticated attacker on the same machine (or on the network, if the listen address is changed to `0.0.0.0`) could execute arbitrary Maxima expressions, including `system()` calls, leading to RCE.

**Current mitigation:**
- **Bearer token authentication.** Every HTTP request must include an `Authorization: Bearer <token>` header. The token is a cryptographically random 256-bit hex string generated on first launch, stored in the app config, and visible/regenerable in Settings. Requests without a valid token receive HTTP 401.
- **Localhost binding.** The default listen address is `127.0.0.1`, which restricts access to the local machine. Users can change this in Settings but should understand the risk of binding to a non-loopback address.
- The token is compared in constant-length string comparison (both sides are always 64 hex chars), though timing attacks are low-risk for a localhost service.

**Limitations:**
- The token is stored in plaintext in the app config JSON file. Anyone with read access to the user's config directory can extract it.
- There is no TLS — the token is sent in cleartext over HTTP. This is acceptable for localhost but would be insecure over a network.
- There is no rate limiting on authentication failures.

### 6. Denial of service

- **Large notebooks:** No size limits on notebook JSON. A multi-gigabyte file could exhaust memory during parsing.
- **Expensive Maxima expressions:** Cell execution has a configurable timeout (default 30 seconds, max 600 seconds) enforced via `tokio::time::timeout` in the protocol layer. However, a user can set a long timeout, and expressions can still consume significant CPU within the allowed window.
- **Rapid evaluation requests:** No rate limiting on the `evaluate_expression` Tauri command.

These are low-severity for a desktop application since the user can kill the process.

## Summary of attack surfaces

| Surface | Trigger | Impact | Status | Mitigation |
|---------|---------|--------|--------|------------|
| `system()` in Maxima | User runs cell | RCE | No mitigation | — |
| SVG rendering | User runs plot cell | XSS / IPC access | Mitigated | SVG sanitized via `sanitizeSvg()` then rendered as `<img src="blob:...">`. The `<img>` context natively blocks all script execution and event handlers. Sanitization is defence-in-depth. CSP blocks inline scripts as an additional layer. |
| SVG file path reading | User runs plot cell | Arbitrary file read | Mitigated | Path canonicalized and validated: must have `.svg` extension and reside within the system temp directory. |
| KaTeX trust mode | User runs cell producing LaTeX | XSS via `\href` | Mitigated | `trust` set to `false` in both `KatexOutput` and `MathText`. |
| `write_plot_svg` IPC | User clicks "Save SVG" | Arbitrary file write | Mitigated | Backend enforces `.svg` extension and rejects paths containing `..` segments. |
| MCP server (connected mode) | MCP enabled in Settings | RCE via Maxima | Mitigated | Bearer token auth (256-bit random token). Localhost-only by default. |
| Notebook JSON parsing | User opens file | DoS (memory) | No mitigation | — |
| Maxima execution time | User runs cell | DoS (CPU) | Partial | Configurable eval timeout (default 30s, max 600s). |

## Design principles

1. **Never auto-execute.** Opening a notebook must never run any cell. This is the most important safety property.
2. **Sanitize all rendered output.** Anything originating from Maxima or notebook files must be treated as untrusted before rendering in the WebView.
3. **Restrict file access.** File reads and writes initiated by the backend should be scoped to expected directories.
4. **Warn on dangerous operations.** Consider flagging cells that contain known-dangerous Maxima functions before execution.
