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

Plot output SVGs are rendered using `dangerouslySetInnerHTML` with no sanitization. If an SVG contains `<script>` tags or event handler attributes (`onload`, `onerror`, etc.), they execute in the app context.

The SVG content originates from files that Maxima writes during plot operations. The Rust parser reads SVG files from paths extracted from Maxima's text output (`src-tauri/src/maxima/parser.rs`). The path extraction uses a regex match with no validation, so a crafted Maxima output could cause the parser to read arbitrary files and inject their contents as SVG.

**Recommendations:**
- Sanitize SVG content before rendering (strip `<script>`, event handlers, `<foreignObject>`, `<iframe>`, data URIs)
- Restrict SVG path reading to the system temp directory
- Consider rendering SVGs in a sandboxed `<iframe>`

#### KaTeX trust mode

**Files:** `src/components/KatexOutput.tsx`, `src/components/MathText.tsx`

KaTeX is configured with `trust: true`, which allows macros like `\href{javascript:...}{text}` that execute JavaScript when clicked.

**Recommendation:** Set `trust: false`.

#### Markdown rendering

**File:** `src/components/MarkdownCell.tsx`

Markdown cells are rendered with `react-markdown`, which sanitizes HTML by default. This is currently safe, but the lack of explicit sanitization configuration means a future library update changing defaults could introduce a vulnerability.

**Recommendation:** Explicitly configure `rehype-sanitize` to make the safe behaviour intentional rather than incidental.

### 4. Arbitrary file access

#### File read via SVG path injection

**File:** `src-tauri/src/maxima/parser.rs`

The parser extracts file paths from Maxima's text output using a regex and reads them with `fs::read_to_string`. There is no validation that the path points to a legitimate SVG file or resides in an expected directory.

**Impact:** Read any file accessible to the Aximar process. Contents are embedded in the cell output and visible in the UI.

**Recommendation:** Only read SVG files from the expected temp directory. Reject paths containing `..` or pointing outside the allowed directory.

#### File write via `write_plot_svg` command

**File:** `src-tauri/src/commands/plot.rs`

The `write_plot_svg` Tauri command accepts an arbitrary file path and writes content to it. While the UI gates this behind a save dialog, the IPC command itself performs no path validation.

**Impact:** Arbitrary file write to any location writable by the process.

**Recommendation:** Validate that the path has a `.svg` extension. Consider using Tauri's file scope restrictions.

### 5. Denial of service

- **Large notebooks:** No size limits on notebook JSON. A multi-gigabyte file could exhaust memory during parsing.
- **Expensive Maxima expressions:** No timeout on cell execution. An expression like `factor(2^(2^20) - 1)$` could run indefinitely.
- **Rapid evaluation requests:** No rate limiting on the `evaluate_expression` Tauri command.

These are low-severity for a desktop application since the user can kill the process.

## Summary of attack surfaces

| Surface | Trigger | Impact | Status | Mitigation |
|---------|---------|--------|--------|------------|
| `system()` in Maxima | User runs cell | RCE | No mitigation | — |
| SVG rendering | User runs plot cell | XSS / IPC access | Mitigated | SVG sanitized via DOMParser then rendered as `<img src="blob:...">`. The `<img>` context natively blocks all script execution and event handlers. Sanitization is defence-in-depth. CSP blocks inline scripts as an additional layer. |
| SVG file path reading | User runs plot cell | Arbitrary file read | Mitigated | Path canonicalized and validated: must have `.svg` extension and reside within the system temp directory. |
| KaTeX `trust: true` | User runs cell producing LaTeX | XSS via `\href` | Mitigated | `trust` set to `false` in both `KatexOutput` and `MathText`. |
| `write_plot_svg` IPC | User clicks "Save SVG" | Arbitrary file write | Mitigated | Backend enforces `.svg` extension and rejects paths containing `..` segments. |
| Notebook JSON parsing | User opens file | DoS (memory) | No mitigation | — |
| Maxima execution time | User runs cell | DoS (CPU) | No mitigation | — |

## Design principles

1. **Never auto-execute.** Opening a notebook must never run any cell. This is the most important safety property.
2. **Sanitize all rendered output.** Anything originating from Maxima or notebook files must be treated as untrusted before rendering in the WebView.
3. **Restrict file access.** File reads and writes initiated by the backend should be scoped to expected directories.
4. **Warn on dangerous operations.** Consider flagging cells that contain known-dangerous Maxima functions before execution.
