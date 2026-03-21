use std::time::Instant;

use crate::catalog::search::Catalog;
use crate::error::AppError;
use crate::maxima::parser;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::EvalResult;

const EVAL_SENTINEL: &str = "__AXIMAR_EVAL_END__";
const VARS_SENTINEL: &str = "__AXIMAR_VARS_END__";
const VARS_START: &str = "__AXIMAR_VARS__";
const EVAL_TIMEOUT_SECS: u64 = 30;
const VARS_TIMEOUT_SECS: u64 = 5;

pub async fn evaluate(
    process: &mut MaximaProcess,
    cell_id: &str,
    expression: &str,
    catalog: &Catalog,
) -> Result<EvalResult, AppError> {
    let start = Instant::now();

    // Ensure the expression is properly terminated for Maxima
    let expr = expression.trim();
    let expr = if expr.ends_with(';') || expr.ends_with('$') {
        expr.to_string()
    } else {
        format!("{};", expr)
    };

    let input = format!(
        "{}\ntex(%);\nprint(\"__AXIMAR_LABEL__\", linenum)$\nprint(\"{}\");\n",
        expr,
        EVAL_SENTINEL
    );

    process.write_stdin(&input).await?;

    let lines = tokio::time::timeout(
        std::time::Duration::from_secs(EVAL_TIMEOUT_SECS),
        process.read_until_sentinel(EVAL_SENTINEL),
    )
    .await
    .map_err(|_| AppError::Timeout(EVAL_TIMEOUT_SECS))??;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(parser::parse_output(cell_id, &lines, duration_ms, catalog))
}

pub async fn query_variables(process: &mut MaximaProcess) -> Result<Vec<String>, AppError> {
    let input = format!(
        "print(\"{}\", values)$\nprint(\"{}\");\n",
        VARS_START, VARS_SENTINEL
    );

    process.write_stdin(&input).await?;

    let lines = tokio::time::timeout(
        std::time::Duration::from_secs(VARS_TIMEOUT_SECS),
        process.read_until_sentinel(VARS_SENTINEL),
    )
    .await
    .map_err(|_| AppError::Timeout(VARS_TIMEOUT_SECS))??;

    // Find the line containing __AXIMAR_VARS__ and parse the variable list
    // Maxima outputs: __AXIMAR_VARS__ [a,b,c] or __AXIMAR_VARS__ [] if none
    let mut vars = Vec::new();
    for line in &lines {
        if let Some(pos) = line.find(VARS_START) {
            let rest = &line[pos + VARS_START.len()..];
            // Extract content between [ and ]
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

    tokio::time::timeout(
        std::time::Duration::from_secs(VARS_TIMEOUT_SECS),
        process.read_until_sentinel(VARS_SENTINEL),
    )
    .await
    .map_err(|_| AppError::Timeout(VARS_TIMEOUT_SECS))??;

    Ok(())
}

pub async fn kill_all_variables(process: &mut MaximaProcess) -> Result<(), AppError> {
    let input = format!(
        "kill(values)$\nprint(\"{}\");\n",
        VARS_SENTINEL
    );

    process.write_stdin(&input).await?;

    tokio::time::timeout(
        std::time::Duration::from_secs(VARS_TIMEOUT_SECS),
        process.read_until_sentinel(VARS_SENTINEL),
    )
    .await
    .map_err(|_| AppError::Timeout(VARS_TIMEOUT_SECS))??;

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
    INTERNAL_VARS.contains(&name)
}
