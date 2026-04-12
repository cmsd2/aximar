use std::time::Instant;

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::error::AppError;
use crate::maxima::parser;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::EvalResult;

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

    // Ensure the expression is properly terminated for Maxima
    let expr = expression.trim();
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
        "{}\ntex(%);\nprint(\"__AXIMAR_LABEL__\", linenum)$\nprint(\"{}\");\n",
        expr, EVAL_SENTINEL
    );

    process.write_stdin(&input).await?;

    let (lines, _prompt) = match tokio::time::timeout(
        std::time::Duration::from_secs(eval_timeout_secs),
        process.read_until_sentinel(EVAL_SENTINEL),
    )
    .await
    {
        Ok(result) => result?,
        Err(_) => {
            process.interrupt_and_resync(EVAL_SENTINEL).await;
            return Err(AppError::Timeout(eval_timeout_secs));
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut result = parser::parse_output(cell_id, &lines, duration_ms, catalog, process.backend());
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

    let expr = expression.trim();
    let expr = if expr.ends_with(';') || expr.ends_with('$') {
        expr.to_string()
    } else {
        format!("{};", expr)
    };
    let (expr, emit_latex) = suppress_display(&expr);

    // Always run tex(%) so the parser can detect plot file paths from LaTeX
    // \mbox{} blocks, even when the user suppressed output with $.
    let input = format!(
        "{}\ntex(%);\nprint(\"__AXIMAR_LABEL__\", linenum)$\nprint(\"{}\");\n",
        expr, EVAL_SENTINEL
    );

    process.write_stdin(&input).await?;

    let (lines, _prompt) = match tokio::time::timeout(
        std::time::Duration::from_secs(eval_timeout_secs),
        process.read_until_sentinel(EVAL_SENTINEL),
    )
    .await
    {
        Ok(result) => result?,
        Err(_) => {
            process.interrupt_and_resync(EVAL_SENTINEL).await;
            return Err(AppError::Timeout(eval_timeout_secs));
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    let mut result = parser::parse_output_with_packages(
        cell_id, &lines, duration_ms, catalog, packages, process.backend(),
    );
    if !emit_latex {
        result.latex = None;
    }
    Ok(result)
}

pub async fn query_variables(process: &mut MaximaProcess) -> Result<Vec<String>, AppError> {
    let input = format!(
        "print(\"{}\", values)$\nprint(\"{}\");\n",
        VARS_START, VARS_SENTINEL
    );

    process.write_stdin(&input).await?;

    let (lines, _prompt) = match tokio::time::timeout(
        std::time::Duration::from_secs(VARS_TIMEOUT_SECS),
        process.read_until_sentinel(VARS_SENTINEL),
    )
    .await
    {
        Ok(result) => result?,
        Err(_) => {
            process.interrupt_and_resync(VARS_SENTINEL).await;
            return Err(AppError::Timeout(VARS_TIMEOUT_SECS));
        }
    };

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
        "kill({})$\nprint(\"{}\");\n",
        name, VARS_SENTINEL
    );

    process.write_stdin(&input).await?;

    match tokio::time::timeout(
        std::time::Duration::from_secs(VARS_TIMEOUT_SECS),
        process.read_until_sentinel(VARS_SENTINEL),
    )
    .await
    {
        Ok(result) => { result?; }
        Err(_) => {
            process.interrupt_and_resync(VARS_SENTINEL).await;
            return Err(AppError::Timeout(VARS_TIMEOUT_SECS));
        }
    }

    Ok(())
}

pub async fn kill_all_variables(process: &mut MaximaProcess) -> Result<(), AppError> {
    // Kill user variables but preserve ax__ internal variables used by
    // Aximar's plotting functions (ax__layout_option_names, etc.).
    // Uses ssearch from stringproc (loaded during session init by ax_plotting.mac).
    let input = format!(
        "block([ax__kill_list], ax__kill_list: sublist(values, lambda([v], not is(ssearch(\"ax__\", string(v)) = 1))), apply(kill, ax__kill_list))$\nprint(\"{}\");\n",
        VARS_SENTINEL
    );

    process.write_stdin(&input).await?;

    match tokio::time::timeout(
        std::time::Duration::from_secs(VARS_TIMEOUT_SECS),
        process.read_until_sentinel(VARS_SENTINEL),
    )
    .await
    {
        Ok(result) => { result?; }
        Err(_) => {
            process.interrupt_and_resync(VARS_SENTINEL).await;
            return Err(AppError::Timeout(VARS_TIMEOUT_SECS));
        }
    }

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
}
