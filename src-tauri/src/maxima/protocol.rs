use std::time::Instant;

use crate::error::AppError;
use crate::maxima::parser;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::EvalResult;

const EVAL_SENTINEL: &str = "__AXIMAR_EVAL_END__";
const EVAL_TIMEOUT_SECS: u64 = 30;

pub async fn evaluate(
    process: &mut MaximaProcess,
    cell_id: &str,
    expression: &str,
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
        "{}\ntex(%);\nprint(\"{}\");\n",
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

    Ok(parser::parse_output(cell_id, &lines, duration_ms))
}
