use std::time::Instant;

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::error::AppError;
#[cfg(unix)]
use crate::maxima::errors as error_enhance;
#[cfg(unix)]
use crate::maxima::events::Envelope;
use crate::maxima::parser;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::EvalResult;

/// Wait on `main` while concurrently draining any envelopes that
/// arrive on `events_rx`.  Returns the future's output and the
/// vector of envelopes collected during its lifetime.
///
/// Phase-A.1: we still terminate the eval on the legacy
/// `__AXIMAR_EVAL_END__` sentinel — envelopes are observed alongside,
/// not used as the terminator.  Phase B will start consuming them
/// to fill in EvalResult fields (errors first), at which point this
/// helper's caller will route the collected envelopes into the
/// parser.
#[cfg(unix)]
pub(crate) async fn drive_with_envelope_drain<F, T>(
    main: F,
    events_rx: &mut Option<tokio::sync::mpsc::UnboundedReceiver<Envelope>>,
) -> (T, Vec<Envelope>)
where
    F: std::future::Future<Output = T>,
{
    let mut envs = Vec::new();
    tokio::pin!(main);
    let output = loop {
        // `biased` so we prefer draining queued envelopes when both
        // arms are ready — keeps the channel from backing up while
        // the read future is still running.
        tokio::select! {
            biased;
            env = recv_maybe(events_rx) => {
                if let Some(e) = env {
                    envs.push(e);
                }
                // Receiver closed: stop polling the events arm but
                // keep waiting for the main future to finish.
            }
            out = &mut main => break out,
        }
    };
    // Final non-blocking drain: anything that arrived after the
    // main future resolved but before we stopped polling.
    if let Some(rx) = events_rx.as_mut() {
        while let Ok(e) = rx.try_recv() {
            envs.push(e);
        }
    }
    (output, envs)
}

/// Helper for `tokio::select!`: when the receiver is None (kernel-
/// events disabled or already taken), return a never-resolving future
/// so the other arm always wins.  When it's Some, defer to recv().
#[cfg(unix)]
async fn recv_maybe(
    rx: &mut Option<tokio::sync::mpsc::UnboundedReceiver<Envelope>>,
) -> Option<Envelope> {
    match rx {
        Some(r) => r.recv().await,
        None => std::future::pending().await,
    }
}

#[cfg(not(unix))]
pub(crate) async fn drive_with_envelope_drain<F, T>(main: F, _events_rx: &mut Option<()>) -> (T, Vec<()>)
where
    F: std::future::Future<Output = T>,
{
    (main.await, Vec::new())
}

const EVAL_SENTINEL: &str = "__AXIMAR_EVAL_END__";
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

    // Always run tex(%) so the parser can detect plot file paths from LaTeX
    // \mbox{} blocks, even when the user suppressed output with $.
    let input = format!(
        // `$` not `;` on the sentinel print: `;` makes Maxima display
        // the return value of print(), which is the printed string
        // itself.  That left an orphaned `"__AXIMAR_EVAL_END__"` line
        // in the BufReader after each eval, which could trigger the
        // next read_until_sentinel to return prematurely on a substring
        // match — leaking content between evaluations.
        "{}\ntex(%);\nprint(\"__AXIMAR_LABEL__\", linenum)$\nprint(\"{}\")$\n",
        expr, EVAL_SENTINEL
    );

    process.write_stdin(&input).await?;

    // Briefly remove the events receiver from the process so we can
    // poll it concurrently with `read_until_sentinel` (both want
    // `&mut process`).  Put it back when we're done; in-flight
    // envelopes received during the eval are collected for later use.
    let mut events_rx = process.take_events_rx();

    let timeout_result = tokio::time::timeout(
        std::time::Duration::from_secs(eval_timeout_secs),
        drive_with_envelope_drain(process.read_until_sentinel(EVAL_SENTINEL), &mut events_rx),
    )
    .await;

    let (read_result, envelopes) = match timeout_result {
        Ok((read_result, envs)) => (read_result, envs),
        Err(_) => {
            process.restore_events_rx(events_rx);
            process.interrupt_and_resync(EVAL_SENTINEL).await;
            return Err(AppError::Timeout(eval_timeout_secs));
        }
    };

    process.restore_events_rx(events_rx);
    let (lines, _prompt) = read_result?;

    log_envelope_summary(cell_id, &envelopes);

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut result = parser::parse_output(cell_id, &lines, duration_ms, catalog, process.backend());
    apply_error_envelopes(&mut result, &envelopes, catalog, None)?;
    apply_display_envelopes(&mut result, &envelopes);
    // If user suppressed output with $, clear the LaTeX (but plot detection
    // already happened in the parser using raw_latex).
    if !emit_latex {
        result.latex = None;
    }
    Ok(result)
}

/// Phase-A.1: log what came in on the events channel during an eval
/// so we can validate the drain end-to-end before any envelope type
/// starts feeding into EvalResult.  Off when no envelopes arrived
/// (kernel-events not enabled / not installed) to keep stderr quiet.
#[cfg(unix)]
fn log_envelope_summary(cell_id: &str, envelopes: &[Envelope]) {
    if envelopes.is_empty() {
        return;
    }
    let mut kinds: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for env in envelopes {
        *kinds.entry(env.kind_label()).or_insert(0) += 1;
    }
    eprintln!("[events] cell={} envelopes={:?}", cell_id, kinds);
}

#[cfg(not(unix))]
fn log_envelope_summary(_cell_id: &str, _envelopes: &[()]) {}

/// Phase B / B.1: when an `error` envelope arrived during the eval,
/// take the kernel-events view as authoritative.  kernel-events
/// captures the merror() message verbatim and tags it with one of
/// maxima_error / lisp_error / parser_error / timeout / cancelled —
/// information the stdout scrape can't reliably recover (lisp errors
/// in particular often lack the markers the regex keys on).
///
/// Returns `Err(AppError::EvalCancelled)` when the first error
/// envelope has `kind: cancelled` so the caller surfaces it through
/// a dedicated UI affordance rather than the generic error path.
/// Other kinds are flattened into `EvalResult.error` and enhanced
/// through the existing pattern-match suggestions pipeline.
///
/// No-op (returns `Ok`) when no error envelope is present, so the
/// legacy parser scrape stays authoritative on builds where
/// kernel-events is disabled.
#[cfg(unix)]
fn apply_error_envelopes(
    result: &mut EvalResult,
    envelopes: &[Envelope],
    catalog: &Catalog,
    packages: Option<&PackageCatalog>,
) -> Result<(), AppError> {
    let first = envelopes.iter().find_map(|e| match e {
        Envelope::Error(err) => Some(err),
        _ => None,
    });
    let Some(err) = first else { return Ok(()) };

    if matches!(err.kind, crate::maxima::events::ErrorKind::Cancelled) {
        // Short-circuit: cancellation isn't an evaluation error
        // proper — the user (or a host-side cancel transport) asked
        // for it.  Caller maps to a distinct UI state.
        return Err(AppError::EvalCancelled(err.message.clone()));
    }

    result.error = Some(err.message.clone());
    result.is_error = true;
    result.error_info = error_enhance::enhance_error_with_packages(
        &err.message,
        catalog,
        packages,
    );
    // Output label has no meaning when the eval errored — no %oN was
    // assigned.  The legacy parser also clears this in the same case.
    result.output_label = None;
    Ok(())
}

#[cfg(not(unix))]
fn apply_error_envelopes(
    _result: &mut EvalResult,
    _envelopes: &[()],
    _catalog: &Catalog,
    _packages: Option<&PackageCatalog>,
) -> Result<(), AppError> {
    Ok(())
}

/// Phase C: when a `display` envelope carrying a structured plot
/// arrived during the eval, take its inline JSON as authoritative
/// over the parser's `.plotly.json` path scrape.  The envelope path
/// avoids two failure modes the legacy scrape is exposed to: 1) the
/// path landing in a LaTeX `\mbox{}` block when display is suppressed
/// (where it's easy to miss); 2) reading a stale file when temp-name
/// collision happens.
///
/// Only fires when at least one display envelope carries the
/// `application/x-maxima-plotly` mime; absent envelopes (kernel-events
/// disabled, ax-plots predating Phase C, …) leave the parser's
/// legacy path-read authoritative — a strict compatibility superset.
///
/// First display envelope wins.  Multiple plots in one cell stay
/// represented through the legacy path-print + parser scrape until a
/// future phase teaches EvalResult to carry an array.
#[cfg(unix)]
fn apply_display_envelopes(result: &mut EvalResult, envelopes: &[Envelope]) {
    for env in envelopes {
        if let Envelope::Display(d) = env {
            if let Some(value) = d.mime_bundle.get("application/x-maxima-plotly") {
                let json_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                if !json_str.is_empty() {
                    result.plot_data = Some(json_str);
                    return;
                }
            }
        }
    }
}

#[cfg(not(unix))]
fn apply_display_envelopes(_result: &mut EvalResult, _envelopes: &[()]) {}

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

    // Always run tex(%) so the parser can detect plot file paths from LaTeX
    // \mbox{} blocks, even when the user suppressed output with $.
    let input = format!(
        // `$` not `;` on the sentinel print: `;` makes Maxima display
        // the return value of print(), which is the printed string
        // itself.  That left an orphaned `"__AXIMAR_EVAL_END__"` line
        // in the BufReader after each eval, which could trigger the
        // next read_until_sentinel to return prematurely on a substring
        // match — leaking content between evaluations.
        "{}\ntex(%);\nprint(\"__AXIMAR_LABEL__\", linenum)$\nprint(\"{}\")$\n",
        expr, EVAL_SENTINEL
    );

    process.write_stdin(&input).await?;

    let mut events_rx = process.take_events_rx();

    let timeout_result = tokio::time::timeout(
        std::time::Duration::from_secs(eval_timeout_secs),
        drive_with_envelope_drain(process.read_until_sentinel(EVAL_SENTINEL), &mut events_rx),
    )
    .await;

    let (read_result, envelopes) = match timeout_result {
        Ok((read_result, envs)) => (read_result, envs),
        Err(_) => {
            process.restore_events_rx(events_rx);
            process.interrupt_and_resync(EVAL_SENTINEL).await;
            return Err(AppError::Timeout(eval_timeout_secs));
        }
    };

    process.restore_events_rx(events_rx);
    let (lines, _prompt) = read_result?;

    log_envelope_summary(cell_id, &envelopes);

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut result = parser::parse_output_with_packages(
        cell_id, &lines, duration_ms, catalog, packages, process.backend(),
    );
    apply_error_envelopes(&mut result, &envelopes, catalog, Some(packages))?;
    apply_display_envelopes(&mut result, &envelopes);
    if !emit_latex {
        result.latex = None;
    }
    Ok(result)
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
    use crate::maxima::events::ErrorKind;
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

pub async fn query_variables(process: &mut MaximaProcess) -> Result<Vec<String>, AppError> {
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
