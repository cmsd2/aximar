use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::error::AppError;

/// Build the stdin write for one cell evaluation.
///
/// On the kernel-events-wired path the input is just the user's
/// expression: latex, output_label, and plot-path detection all come
/// from envelopes (eval_result mime_bundle and display envelopes),
/// termination comes from eval_end envelope count, and the host
/// never reads stdout for content.  No housekeeping prints needed.
///
/// On the legacy stdout-parsing path the input adds three statements
/// the parser depends on:
///   - `tex(%);` so it can extract LaTeX from $$..$$ blocks and
///     plot file paths from \mbox{} blocks;
///   - `print("__AXIMAR_LABEL__", linenum)$` so it can correlate
///     the cell with Maxima's %oN output label;
///   - `print("<sentinel>")$` as the read terminator (per-eval
///     unique so user content can't accidentally trigger it).
fn build_eval_input(expr: &str, sentinel: &str, wired: bool) -> String {
    if wired {
        format!("{}\n", expr)
    } else {
        format!(
            "{}\ntex(%);\nprint(\"__AXIMAR_LABEL__\", linenum)$\nprint(\"{}\")$\n",
            expr, sentinel
        )
    }
}

/// Per-eval unique-sentinel counter.  Each call to `next_eval_sentinel`
/// returns a fresh string that cannot collide with a user's earlier
/// or current cell output — eliminates the substring-match leak that
/// the static `__AXIMAR_EVAL_END__` was vulnerable to (a user's own
/// `print("__AXIMAR_EVAL_END__")` would trip the legacy reader).
fn next_eval_sentinel() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("__AXIMAR_EVAL_END_{:016x}__", n)
}
#[cfg(unix)]
use crate::maxima::envelope::types::Envelope;
use crate::maxima::envelope::drain::drive_with_envelope_drain;
use crate::maxima::envelope::overlay::{
    apply_display_envelopes, apply_error_envelopes, apply_eval_result_envelopes,
    log_envelope_summary,
};
use crate::maxima::legacy::parser;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::EvalResult;

const VARS_SENTINEL: &str = "__AXIMAR_VARS_END__";
const VARS_START: &str = "__AXIMAR_VARS__";
const VARS_TIMEOUT_SECS: u64 = 5;

pub async fn evaluate(
    process: &mut MaximaProcess,
    cell_id: &str,
    expression: &str,
    catalog: &Catalog,
    eval_timeout_secs: u64,
) -> Result<EvalResult, AppError> {
    let start = Instant::now();

    // Strip trailing comments that follow the last terminator, then ensure
    // the expression ends with `;` or `$`.
    let expr = strip_trailing_comments(expression.trim());
    if expr.is_empty() {
        return Err(AppError::EmptyInput);
    }
    let expr = if expr.ends_with(';') || expr.ends_with('$') {
        expr.to_string()
    } else {
        format!("{};", expr)
    };
    // Suppress the last statement's 1D display (we render it as LaTeX instead).
    // If the user ended with `$`, they don't want any result shown.
    let (expr, emit_latex) = suppress_display(&expr);

    // Build the per-cell stdin.  On the wired (envelope) path, the
    // user's expression is the entire input — eval_result envelopes
    // carry latex and output_label, eval_end envelopes drive
    // termination, plot-path detection rides on
    // eval_result.text/plain.  On the legacy stdout path the
    // housekeeping prints stay because the parser needs the tex(%)
    // latex, the LABEL stdout line, and the EVAL_END sentinel.
    let sentinel = next_eval_sentinel();
    let input = build_eval_input(&expr, &sentinel, process.has_events_channel());

    process.write_stdin(&input).await?;

    let (lines, envelopes) =
        run_eval_read_with_envelope_drain(process, &input, &sentinel, eval_timeout_secs).await?;

    log_envelope_summary(cell_id, &envelopes);

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut result = parser::parse_output(cell_id, &lines, duration_ms, catalog, process.backend());
    apply_error_envelopes(&mut result, &envelopes, catalog, None)?;
    apply_display_envelopes(&mut result, &envelopes);
    apply_eval_result_envelopes(&mut result, &envelopes, emit_latex, process.backend());
    // If user suppressed output with $, clear the LaTeX (but plot detection
    // already happened in the parser using raw_latex).
    if !emit_latex {
        result.latex = None;
    }
    Ok(result)
}

pub async fn evaluate_with_packages(
    process: &mut MaximaProcess,
    cell_id: &str,
    expression: &str,
    catalog: &Catalog,
    packages: &PackageCatalog,
    eval_timeout_secs: u64,
) -> Result<EvalResult, AppError> {
    let start = Instant::now();

    let expr = strip_trailing_comments(expression.trim());
    if expr.is_empty() {
        return Err(AppError::EmptyInput);
    }
    let expr = if expr.ends_with(';') || expr.ends_with('$') {
        expr.to_string()
    } else {
        format!("{};", expr)
    };
    let (expr, emit_latex) = suppress_display(&expr);

    let sentinel = next_eval_sentinel();
    let input = build_eval_input(&expr, &sentinel, process.has_events_channel());

    process.write_stdin(&input).await?;

    let (lines, envelopes) =
        run_eval_read_with_envelope_drain(process, &input, &sentinel, eval_timeout_secs).await?;

    log_envelope_summary(cell_id, &envelopes);

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut result = parser::parse_output_with_packages(
        cell_id, &lines, duration_ms, catalog, packages, process.backend(),
    );
    apply_error_envelopes(&mut result, &envelopes, catalog, Some(packages))?;
    apply_display_envelopes(&mut result, &envelopes);
    apply_eval_result_envelopes(&mut result, &envelopes, emit_latex, process.backend());
    if !emit_latex {
        result.latex = None;
    }
    Ok(result)
}

/// Phase A.2: drive the eval read with a fence between stdout and
/// envelopes.  The eval input ends with `print("<sentinel>")$` where
/// `<sentinel>` is per-eval unique.  When that print's text lands on
/// stdout and its `eval_end` envelope lands on the mpsc, both streams
/// have caught up to the same point in Maxima's processing — that's
/// the fence.  No wall-clock timeouts needed for coordination; both
/// signals are structural ("Maxima eval-hooks emitted the envelope"
/// and "Maxima's print() wrote the line to stdout").
///
/// On non-wired sessions (kernel-events disabled, Docker / WSL),
/// falls back to the legacy `read_until_sentinel` path.
async fn run_eval_read_with_envelope_drain(
    process: &mut MaximaProcess,
    input: &str,
    sentinel: &str,
    eval_timeout_secs: u64,
) -> Result<(Vec<String>, Vec<Envelope>), AppError> {
    let mut events_rx = process.take_events_rx();

    if events_rx.is_some() {
        // Single-source envelope-only path.  Background drain owns
        // stdout/stderr; eval read only watches the envelope mpsc.
        // No fence needed — text content and termination signal
        // both come from one stream (kernel-events fd-3), ordered
        // by the kernel's serial emission.
        let expected_count = find_terminators(input).len();
        let timeout_result = tokio::time::timeout(
            std::time::Duration::from_secs(eval_timeout_secs),
            process.read_n_eval_ends_envelope_only(expected_count, &mut events_rx),
        )
        .await;
        let read_result = match timeout_result {
            Ok(res) => res,
            Err(_) => {
                process.restore_events_rx(events_rx);
                // Background drain owns stdout, so we can't use it
                // for the regroup marker — kill the process instead.
                // Cancel transport could be used if the user code
                // calls check_cancel; this is the timeout fallback.
                let _ = process.kill().await;
                return Err(AppError::Timeout(eval_timeout_secs));
            }
        };
        process.restore_events_rx(events_rx);
        read_result
    } else {
        // Legacy path: substring match on the sentinel in stdout.
        // No envelopes flow on this branch.
        let timeout_result = tokio::time::timeout(
            std::time::Duration::from_secs(eval_timeout_secs),
            drive_with_envelope_drain(process.read_until_sentinel(sentinel), &mut events_rx),
        )
        .await;
        let (read_result, envelopes) = match timeout_result {
            Ok((rr, env)) => (rr, env),
            Err(_) => {
                process.restore_events_rx(events_rx);
                process.interrupt_and_resync(sentinel).await;
                return Err(AppError::Timeout(eval_timeout_secs));
            }
        };
        process.restore_events_rx(events_rx);
        let (lines, _prompt) = read_result?;
        Ok((lines, envelopes))
    }
}

/// Read stdout until `sentinel` while concurrently draining the
/// kernel-events channel — the same pattern as `protocol::evaluate`,
/// but for internal-protocol commands (variables query, kill,
/// kill-all) where the host injected the Maxima code itself rather
/// than the user.  Envelopes that arrive during the internal command
/// are pulled out of the channel so they don't pollute the *next*
/// user evaluation's drain, and any `error` envelope is surfaced as
/// an `AppError` so internal command failures aren't silently lost.
async fn read_internal_until_sentinel(
    process: &mut MaximaProcess,
    sentinel: &str,
    timeout_secs: u64,
) -> Result<Vec<String>, AppError> {
    // Single-source: on the wired path, background drain owns
    // stdout/stderr, so we must read envelopes instead.  Internal
    // commands' inputs are at most 2 top-level evals (the kill or
    // values print + the sentinel print), so count eval_ends rather
    // than scanning stdout text.
    if process.has_events_channel() {
        let mut events_rx = process.take_events_rx();
        // Internal commands aren't perfectly statement-counted by
        // their callers, so use a defensive fixed count: 2 (the
        // primary action statement + the sentinel print).
        // query_variables and kill_* all match this shape.
        let expected_count = 2;
        let timeout_result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            process.read_n_eval_ends_envelope_only(expected_count, &mut events_rx),
        )
        .await;
        let read_result = match timeout_result {
            Ok(res) => res,
            Err(_) => {
                process.restore_events_rx(events_rx);
                let _ = process.kill().await;
                return Err(AppError::Timeout(timeout_secs));
            }
        };
        process.restore_events_rx(events_rx);
        let (lines, envelopes) = read_result?;
        check_internal_error_envelopes(&envelopes)?;
        return Ok(lines);
    }

    // Legacy path (non-wired sessions): substring match on stdout.
    let mut events_rx = process.take_events_rx();

    let timeout_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        drive_with_envelope_drain(process.read_until_sentinel(sentinel), &mut events_rx),
    )
    .await;

    let (read_result, envelopes) = match timeout_result {
        Ok((read_result, envs)) => (read_result, envs),
        Err(_) => {
            process.restore_events_rx(events_rx);
            process.interrupt_and_resync(sentinel).await;
            return Err(AppError::Timeout(timeout_secs));
        }
    };

    process.restore_events_rx(events_rx);
    let (lines, _prompt) = read_result?;

    check_internal_error_envelopes(&envelopes)?;
    Ok(lines)
}

/// If an `error` envelope arrived during an internal-protocol
/// command, lift it into an `AppError` so the caller can react.
/// `cancelled` becomes `EvalCancelled` for symmetry with user evals;
/// any other kind becomes `CommunicationError` since these are
/// failures of aximar's own injected Maxima code, not of user input.
#[cfg(unix)]
fn check_internal_error_envelopes(envelopes: &[Envelope]) -> Result<(), AppError> {
    use crate::maxima::envelope::types::ErrorKind;
    for env in envelopes {
        if let Envelope::Error(err) = env {
            return Err(match err.kind {
                ErrorKind::Cancelled => AppError::EvalCancelled(err.message.clone()),
                _ => AppError::CommunicationError(format!(
                    "internal protocol command failed: {}",
                    err.message
                )),
            });
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_internal_error_envelopes(_envelopes: &[()]) -> Result<(), AppError> {
    Ok(())
}

/// Public entry: dispatch to the envelope path when kernel-events
/// is wired; fall back to the legacy stdout-scrape otherwise.  Both
/// paths return the same filtered Vec<String> shape so the Tauri
/// command above doesn't care which one ran.
pub async fn query_variables(process: &mut MaximaProcess) -> Result<Vec<String>, AppError> {
    if process.has_events_channel() {
        query_variables_envelope(process).await
    } else {
        query_variables_legacy(process).await
    }
}

/// Envelope path (Phase B.3): call `emit_vars()` and consume the
/// resulting `vars` envelope directly.  No stdout parsing.
///
/// Still uses a sentinel print as the read terminator so we know when
/// emission is complete and we can stop draining envelopes — eventually
/// (Phase A.2) the `eval_end` envelope can play that role and the
/// VARS_SENTINEL goes away too.
#[cfg(unix)]
async fn query_variables_envelope(
    process: &mut MaximaProcess,
) -> Result<Vec<String>, AppError> {
    use crate::maxima::envelope::types::Envelope;

    // emit_vars() fires a `vars` envelope through the fd-3 sink.
    // The terminator print gives the envelope-count read a second
    // top-level eval to wait for.
    let input = format!("emit_vars()$\nprint(\"{}\")$\n", VARS_SENTINEL);
    process.write_stdin(&input).await?;

    let mut events_rx = process.take_events_rx();

    // Single-source: 2 eval_end envelopes — emit_vars() and the
    // sentinel print.  No stdout reads (background drain owns the
    // pipe on the wired path).
    let timeout_result = tokio::time::timeout(
        std::time::Duration::from_secs(VARS_TIMEOUT_SECS),
        process.read_n_eval_ends_envelope_only(2, &mut events_rx),
    )
    .await;

    let read_result = match timeout_result {
        Ok(res) => res,
        Err(_) => {
            process.restore_events_rx(events_rx);
            let _ = process.kill().await;
            return Err(AppError::Timeout(VARS_TIMEOUT_SECS));
        }
    };

    process.restore_events_rx(events_rx);
    let (_lines, envelopes) = read_result?;

    // Surface any error envelope so internal-command failures aren't
    // silently swallowed (matches read_internal_until_sentinel).
    check_internal_error_envelopes(&envelopes)?;

    // First `vars` envelope wins.  emit_vars() emits exactly one per
    // invocation; if a future kernel-events change emits multiples,
    // the first is the snapshot we asked for.
    //
    // Normalize names: kernel-events derives them from Lisp's
    // symbol-name (always upcased) and Maxima escapes
    // built-in-colliding names with a leading `%` (e.g. `beta:2`
    // binds `%BETA`).  The legacy stdout path gets lowercase
    // user-facing names because Maxima's print() outputs that form;
    // matching it keeps the variables-panel display consistent.
    for env in &envelopes {
        if let Envelope::Vars(v) = env {
            return Ok(v
                .vars
                .iter()
                .map(|n| n.to_lowercase())
                .filter(|n| !is_internal_variable(n))
                .collect());
        }
    }

    // Channel was advertised but no vars envelope arrived — kernel-
    // events loaded but its emit_vars went missing somehow.  Fall
    // back to legacy so the user still gets a populated panel.
    query_variables_legacy(process).await
}

#[cfg(not(unix))]
async fn query_variables_envelope(
    process: &mut MaximaProcess,
) -> Result<Vec<String>, AppError> {
    // has_events_channel is always false on non-unix, so this
    // branch never executes — defensive delegate to legacy.
    query_variables_legacy(process).await
}

/// Legacy path: print the values list to stdout with a marker and
/// scrape the bracketed list back out.  Used when kernel-events isn't
/// wired (Docker / WSL / non-Unix backend, env var off).
async fn query_variables_legacy(
    process: &mut MaximaProcess,
) -> Result<Vec<String>, AppError> {
    let input = format!(
        // `$` on the VARS_END print so its return value (the printed
        // string itself) isn't displayed and left as an orphan
        // sentinel-looking line for the next read to trip on.
        "print(\"{}\", values)$\nprint(\"{}\")$\n",
        VARS_START, VARS_SENTINEL
    );

    process.write_stdin(&input).await?;

    let lines =
        read_internal_until_sentinel(process, VARS_SENTINEL, VARS_TIMEOUT_SECS).await?;

    // Find __AXIMAR_VARS__ and parse the variable list.
    // Maxima may wrap long lists across multiple lines, so join them first.
    let joined = lines.join(" ");
    let mut vars = Vec::new();
    if let Some(pos) = joined.find(VARS_START) {
        let rest = &joined[pos + VARS_START.len()..];
        if let Some(start) = rest.find('[') {
            if let Some(end) = rest.find(']') {
                let content = rest[start + 1..end].trim();
                if !content.is_empty() {
                    for var in content.split(',') {
                        let name = var.trim().to_string();
                        if !name.is_empty() && !is_internal_variable(&name) {
                            vars.push(name);
                        }
                    }
                }
            }
        }
    }

    Ok(vars)
}

pub async fn kill_variable(process: &mut MaximaProcess, name: &str) -> Result<(), AppError> {
    // Validate name contains only alphanumeric and underscore chars to prevent injection
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '%') {
        return Err(AppError::CommunicationError(format!(
            "Invalid variable name: {}",
            name
        )));
    }

    let input = format!(
        // `$` on the sentinel print — see suppress_display rationale
        // in the VARS_START format above.
        "kill({})$\nprint(\"{}\")$\n",
        name, VARS_SENTINEL
    );

    process.write_stdin(&input).await?;
    read_internal_until_sentinel(process, VARS_SENTINEL, VARS_TIMEOUT_SECS).await?;
    Ok(())
}

pub async fn kill_all_variables(process: &mut MaximaProcess) -> Result<(), AppError> {
    // Kill user variables but preserve ax__ internal variables used by
    // Aximar's plotting functions (ax__layout_option_names, etc.).
    // Uses ssearch from stringproc (loaded during session init by ax_plotting.mac).
    let input = format!(
        // `$` on the sentinel print — see VARS_START format above.
        "block([ax__kill_list], ax__kill_list: sublist(values, lambda([v], not is(ssearch(\"ax__\", string(v)) = 1))), apply(kill, ax__kill_list))$\nprint(\"{}\")$\n",
        VARS_SENTINEL
    );

    process.write_stdin(&input).await?;
    read_internal_until_sentinel(process, VARS_SENTINEL, VARS_TIMEOUT_SECS).await?;
    Ok(())
}

/// Filter out Maxima-internal variables that appear in `values` but aren't
/// user-defined. These come from packages (draw, plot) and Maxima internals.
fn is_internal_variable(name: &str) -> bool {
    const INTERNAL_VARS: &[&str] = &[
        "draw_command",
        "gnuplot_command",
        "gnuplot_file_name",
        "gnuplot_term",
        "gnuplot_out_file",
        "gnuplot_preamble",
        "gnuplot_default_term_command",
        "gnuplot_dumb_term_command",
        "gnuplot_ps_term_command",
        "gnuplot_pdf_term_command",
        "gnuplot_png_term_command",
        "gnuplot_svg_term_command",
        "plot_options",
        "maxima_tempdir",
        "maxima_userdir",
        "maxima_objdir",
    ];
    INTERNAL_VARS.contains(&name) || name.starts_with("ax__")
}

/// Strip trailing block comments and whitespace that appear after the last
/// statement terminator. Without this, `det_H : determinant(H); /* comment */`
/// would cause a Maxima parse error because the comment is sent as unterminated
/// input.
fn strip_trailing_comments(expr: &str) -> &str {
    let mut s = expr.trim_end();
    loop {
        if s.ends_with("*/") {
            // Find matching /*
            if let Some(start) = s[..s.len() - 2].rfind("/*") {
                s = s[..start].trim_end();
            } else {
                break; // Unmatched */ — leave as-is
            }
        } else {
            break;
        }
    }
    s
}

/// Find positions of statement terminators (`;` and `$`) in a Maxima expression,
/// skipping those inside string literals and block comments.
fn find_terminators(expr: &str) -> Vec<usize> {
    let bytes = expr.as_bytes();
    let len = bytes.len();
    let mut positions = Vec::new();
    let mut i = 0;
    while i < len {
        match bytes[i] {
            b'"' => {
                // Skip string literal
                i += 1;
                while i < len && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1; // skip escaped char
                    }
                    i += 1;
                }
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                // Skip block comment /* ... */
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                if i + 1 < len {
                    i += 1; // advance past '/'
                }
            }
            b';' | b'$' => {
                positions.push(i);
            }
            _ => {}
        }
        i += 1;
    }
    positions
}

/// Replace only the **last** `;` terminator with `$` to suppress its
/// automatic 1D display, since we capture the final result via `tex(%)`.
/// Intermediate statements keep their original terminators: `;` shows the
/// result, `$` stays silent — matching the user's intent.
///
/// Returns `(modified_expr, emit_latex)`:
/// - `emit_latex = true` if the last terminator was `;` (user wanted display)
/// - `emit_latex = false` if the last terminator was `$` (user suppressed output)
fn suppress_display(expr: &str) -> (String, bool) {
    let terminators = find_terminators(expr);
    if terminators.is_empty() {
        return (expr.to_string(), true);
    }
    let last = *terminators.last().unwrap();
    let emit_latex = expr.as_bytes()[last] == b';';
    let mut result = expr.as_bytes().to_vec();
    if result[last] == b';' {
        result[last] = b'$';
    }
    // expr is valid UTF-8, and we only replaced ASCII bytes
    (String::from_utf8(result).expect("valid UTF-8"), emit_latex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_statement_semicolon() {
        // `;` → suppressed, emit LaTeX
        assert_eq!(suppress_display("x+1;"), ("x+1$".into(), true));
    }

    #[test]
    fn single_statement_dollar() {
        // `$` → no change, no LaTeX
        assert_eq!(suppress_display("x+1$"), ("x+1$".into(), false));
    }

    #[test]
    fn two_statements() {
        assert_eq!(suppress_display("a:5; b:10;"), ("a:5; b:10$".into(), true));
    }

    #[test]
    fn three_statements() {
        assert_eq!(
            suppress_display("a:5; b:10; c:a+b;"),
            ("a:5; b:10; c:a+b$".into(), true)
        );
    }

    #[test]
    fn mixed_terminators_last_semi() {
        assert_eq!(
            suppress_display("a:5; b:10$ c:15;"),
            ("a:5; b:10$ c:15$".into(), true)
        );
    }

    #[test]
    fn mixed_terminators_last_dollar() {
        assert_eq!(
            suppress_display("a:5; b:10$ c:15$"),
            ("a:5; b:10$ c:15$".into(), false)
        );
    }

    #[test]
    fn already_silent() {
        assert_eq!(suppress_display("a:5$ b:10$"), ("a:5$ b:10$".into(), false));
    }

    #[test]
    fn semicolon_in_string_ignored() {
        assert_eq!(
            suppress_display(r#"print("a;b"); x;"#),
            (r#"print("a;b"); x$"#.into(), true)
        );
    }

    #[test]
    fn semicolon_in_comment_ignored() {
        assert_eq!(
            suppress_display("/* a; */ x; y;"),
            ("/* a; */ x; y$".into(), true)
        );
    }

    #[test]
    fn no_terminator() {
        // No terminator → defaults to emit LaTeX
        assert_eq!(suppress_display("x+1"), ("x+1".into(), true));
    }

    #[test]
    fn newlines_between_statements() {
        assert_eq!(
            suppress_display("a:5;\nb:10;\nc:15;"),
            ("a:5;\nb:10;\nc:15$".into(), true)
        );
    }

    #[test]
    fn trailing_dollar() {
        assert_eq!(
            suppress_display("a:5; b:10$"),
            ("a:5; b:10$".into(), false)
        );
    }

    #[test]
    fn strip_trailing_comment_after_semi() {
        assert_eq!(
            strip_trailing_comments("det_H : determinant(H);\n/* saddle point */"),
            "det_H : determinant(H);"
        );
    }

    #[test]
    fn strip_trailing_comment_after_dollar() {
        assert_eq!(
            strip_trailing_comments("x : 5$ /* done */"),
            "x : 5$"
        );
    }

    #[test]
    fn strip_multiple_trailing_comments() {
        assert_eq!(
            strip_trailing_comments("x; /* a */ /* b */"),
            "x;"
        );
    }

    #[test]
    fn no_trailing_comment() {
        assert_eq!(strip_trailing_comments("x + 1;"), "x + 1;");
    }

    #[test]
    fn comment_only_cell() {
        // A cell with only a comment — nothing to strip, leave as-is
        assert_eq!(strip_trailing_comments("/* just a comment */"), "");
    }

    #[test]
    fn inline_comment_preserved() {
        assert_eq!(
            strip_trailing_comments("/* setup */ x : 5;"),
            "/* setup */ x : 5;"
        );
    }
}
