use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

use super::code_spans::{spans, Span};

/// Maps display execution counts to real Maxima labels, and tracks
/// the previous cell's output label for bare `%` resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelContext {
    /// Display execution count → real Maxima output label (e.g. 1 → "%o6")
    pub label_map: HashMap<u32, String>,
    /// The real output label of the most recent previous cell (for bare `%`)
    pub previous_output_label: Option<String>,
}

/// A label reference with no value in the current Maxima session.  The
/// message is shown to the user as the cell's error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedLabel(pub String);

static DISPLAY_LABEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%([oi])(\d+)").unwrap());

/// Rewrite label references in a Maxima expression.
///
/// 1. Replace display `%oN`/`%iN` labels using `label_map` (first).
/// 2. Replace bare `%` in the cell's first statement with
///    `previous_output_label` (second).  In later statements Maxima's own
///    `%` already means the result of the statement before.
///
/// Order matters: display labels are replaced first on the original input,
/// then bare `%` is expanded. This prevents double-rewriting where bare `%`
/// expands to e.g. `%o6` and then the display label regex re-matches it.
///
/// String literals and comments are left as typed.  A display label that
/// isn't in `label_map`, or a bare `%` with no previous output, is an error:
/// passed through unchanged, it would make Maxima read one of its own
/// labels, which hold unrelated values.
pub fn rewrite_labels(input: &str, ctx: &LabelContext) -> Result<String, UnresolvedLabel> {
    let mut result = String::with_capacity(input.len());
    let mut first_statement = true;
    for span in spans(input) {
        match span {
            Span::Text(text) => result.push_str(text),
            Span::Code(code) if first_statement => match code.find([';', '$']) {
                Some(end) => {
                    result.push_str(&rewrite_code(&code[..=end], ctx, true)?);
                    result.push_str(&rewrite_code(&code[end + 1..], ctx, false)?);
                    first_statement = false;
                }
                None => result.push_str(&rewrite_code(code, ctx, true)?),
            },
            Span::Code(code) => result.push_str(&rewrite_code(code, ctx, false)?),
        }
    }
    Ok(result)
}

/// Rewrite labels in a piece of code that contains no strings or comments.
/// `bare_percent` says whether a bare `%` here means the previous cell.
fn rewrite_code(
    input: &str,
    ctx: &LabelContext,
    bare_percent: bool,
) -> Result<String, UnresolvedLabel> {
    // Step 1: replace display %oN / %iN
    let mut result = String::with_capacity(input.len());
    let mut copied = 0;
    for caps in DISPLAY_LABEL_RE.captures_iter(input) {
        let whole = caps.get(0).expect("group 0 always matches");
        // Part of a longer name such as x%o1 or %o1a: not a label
        let before = input[..whole.start()].chars().next_back();
        let after = input[whole.end()..].chars().next();
        if before.is_some_and(is_name_char) || after.is_some_and(is_name_char) {
            continue;
        }
        let kind = &caps[1]; // "o" or "i"
        let real_label = caps[2]
            .parse::<u32>()
            .ok()
            .and_then(|num| ctx.label_map.get(&num))
            .ok_or_else(|| {
                let shown = if kind == "o" { "Out" } else { "In" };
                UnresolvedLabel(format!(
                    "{shown}[{}] ({}) has no value in this session. Run that cell \
                     again and use its new number.",
                    &caps[2],
                    whole.as_str()
                ))
            })?;
        // Extract the number from the real label (e.g. "%o6" → "6")
        let real_num = real_label.trim_start_matches("%o");
        result.push_str(&input[copied..whole.start()]);
        result.push_str(&format!("%{kind}{real_num}"));
        copied = whole.end();
    }
    result.push_str(&input[copied..]);

    // Step 2: replace bare %
    if !bare_percent {
        return Ok(result);
    }
    let previous = ctx.previous_output_label.as_deref();
    match replace_bare_percent(&result, previous.unwrap_or("")) {
        None => Ok(result),
        Some(replaced) if previous.is_some() => Ok(replaced),
        Some(_) => Err(UnresolvedLabel(
            "% refers to the previous cell's output, but no cell above has an output \
             in this session."
                .to_string(),
        )),
    }
}

/// Characters that can continue a Maxima name.
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '%'
}

/// Replace bare `%` (not followed by `%`, letter, digit, or `_`) with
/// `replacement`.  Returns `None` when the input has no bare `%`.
fn replace_bare_percent(input: &str, replacement: &str) -> Option<String> {
    // The regex crate doesn't support lookahead, so scan by hand.
    let mut result = String::with_capacity(input.len());
    let mut found = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.peek() {
                // %% is a Maxima construct — emit both and skip past
                Some(&'%') => {
                    result.push_str("%%");
                    chars.next();
                }
                // Followed by ASCII letter, digit, or underscore — not bare
                Some(&c) if c.is_ascii_alphanumeric() || c == '_' => {
                    result.push('%');
                }
                // Bare % (followed by non-identifier char or end of string)
                _ => {
                    result.push_str(replacement);
                    found = true;
                }
            }
        } else {
            result.push(ch);
        }
    }
    found.then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_bare(prev: Option<&str>) -> LabelContext {
        LabelContext {
            label_map: HashMap::new(),
            previous_output_label: prev.map(String::from),
        }
    }

    fn ctx_map(map: &[(u32, &str)]) -> LabelContext {
        LabelContext {
            label_map: map.iter().map(|(k, v)| (*k, v.to_string())).collect(),
            previous_output_label: None,
        }
    }

    fn ctx_full(prev: Option<&str>, map: &[(u32, &str)]) -> LabelContext {
        LabelContext {
            label_map: map.iter().map(|(k, v)| (*k, v.to_string())).collect(),
            previous_output_label: prev.map(String::from),
        }
    }

    /// Rewrite and expect success.
    fn rw(input: &str, ctx: &LabelContext) -> String {
        rewrite_labels(input, ctx).unwrap()
    }

    /// Rewrite and expect an unresolved-label error; returns its message.
    fn rw_err(input: &str, ctx: &LabelContext) -> String {
        rewrite_labels(input, ctx).unwrap_err().0
    }

    // ── Bare % replacement ──────────────────────────────────────────

    #[test]
    fn bare_percent_replaced() {
        assert_eq!(rw("% + 1", &ctx_bare(Some("%o6"))), "%o6 + 1");
    }

    #[test]
    fn bare_percent_at_end() {
        assert_eq!(rw("1 + %", &ctx_bare(Some("%o6"))), "1 + %o6");
    }

    #[test]
    fn bare_percent_standalone() {
        assert_eq!(rw("%", &ctx_bare(Some("%o6"))), "%o6");
    }

    #[test]
    fn bare_percent_in_parens() {
        assert_eq!(rw("f(%)", &ctx_bare(Some("%o6"))), "f(%o6)");
    }

    #[test]
    fn bare_percent_after_comma() {
        assert_eq!(
            rw("[%, %]", &ctx_bare(Some("%o6"))),
            "[%o6, %o6]"
        );
    }

    #[test]
    fn bare_percent_no_previous() {
        assert!(rw_err("% + 1", &ctx_bare(None)).starts_with("% refers to the previous cell"));
    }

    // ── Bare % only in the first statement ──────────────────────────

    #[test]
    fn later_statements_keep_their_percent() {
        // Maxima's own % in "% + 1" is the result of "x: 5"
        assert_eq!(rw("x: 5; % + 1;", &ctx_bare(Some("%o6"))), "x: 5; % + 1;");
    }

    #[test]
    fn first_statement_ending_in_dollar() {
        assert_eq!(rw("y: %$ % * 2;", &ctx_bare(Some("%o6"))), "y: %o6$ % * 2;");
    }

    #[test]
    fn later_percent_needs_no_previous_cell() {
        assert_eq!(rw("x: 5; % + 1;", &ctx_bare(None)), "x: 5; % + 1;");
    }

    #[test]
    fn terminator_in_string_does_not_end_first_statement() {
        assert_eq!(
            rw(r#"print("a;b", %);"#, &ctx_bare(Some("%o6"))),
            r#"print("a;b", %o6);"#
        );
    }

    #[test]
    fn display_labels_rewritten_in_every_statement() {
        assert_eq!(
            rw("x: %o1; y: %o1 + %;", &ctx_full(Some("%o9"), &[(1, "%o6")])),
            "x: %o6; y: %o6 + %;"
        );
    }

    #[test]
    fn multiple_bare_percent() {
        assert_eq!(
            rw("% * % + %", &ctx_bare(Some("%o6"))),
            "%o6 * %o6 + %o6"
        );
    }

    // ── Special % forms preserved ───────────────────────────────────

    #[test]
    fn percent_e_preserved() {
        assert_eq!(rw("%e + 1", &ctx_bare(Some("%o6"))), "%e + 1");
    }

    #[test]
    fn percent_pi_preserved() {
        assert_eq!(
            rw("%pi * 2", &ctx_bare(Some("%o6"))),
            "%pi * 2"
        );
    }

    #[test]
    fn percent_i_preserved() {
        assert_eq!(rw("%i", &ctx_bare(Some("%o6"))), "%i");
    }

    #[test]
    fn percent_percent_preserved() {
        assert_eq!(rw("%%", &ctx_bare(Some("%o6"))), "%%");
    }

    #[test]
    fn percent_th_preserved() {
        assert_eq!(
            rw("%th(1)", &ctx_bare(Some("%o6"))),
            "%th(1)"
        );
    }

    #[test]
    fn percent_gamma_preserved() {
        assert_eq!(
            rw("%gamma", &ctx_bare(Some("%o6"))),
            "%gamma"
        );
    }

    #[test]
    fn percent_phi_preserved() {
        assert_eq!(rw("%phi", &ctx_bare(Some("%o6"))), "%phi");
    }

    #[test]
    fn percent_with_underscore() {
        assert_eq!(
            rw("%_var", &ctx_bare(Some("%o6"))),
            "%_var"
        );
    }

    // ── Display %oN / %iN rewriting ────────────────────────────────

    #[test]
    fn single_output_label() {
        assert_eq!(rw("%o1", &ctx_map(&[(1, "%o6")])), "%o6");
    }

    #[test]
    fn multiple_output_labels() {
        assert_eq!(
            rw("%o1 + %o2", &ctx_map(&[(1, "%o6"), (2, "%o10")])),
            "%o6 + %o10"
        );
    }

    #[test]
    fn input_label_rewrite() {
        assert_eq!(rw("%i1", &ctx_map(&[(1, "%o6")])), "%i6");
    }

    #[test]
    fn mixed_io_labels() {
        assert_eq!(
            rw("%o1 + %i2", &ctx_map(&[(1, "%o6"), (2, "%o10")])),
            "%o6 + %i10"
        );
    }

    #[test]
    fn unknown_output_label_is_an_error() {
        let message = rw_err("%o99", &ctx_map(&[(1, "%o6")]));
        assert!(message.starts_with("Out[99] (%o99) has no value in this session"));
    }

    #[test]
    fn unknown_input_label_is_an_error() {
        assert!(rw_err("%i3 + 1", &ctx_map(&[(1, "%o6")])).starts_with("In[3] (%i3)"));
    }

    #[test]
    fn label_inside_a_longer_name_is_not_a_label() {
        assert_eq!(rw("x%o1 + %o1y", &ctx_map(&[])), "x%o1 + %o1y");
    }

    #[test]
    fn label_in_expression() {
        assert_eq!(
            rw("diff(%o1, x)", &ctx_map(&[(1, "%o6")])),
            "diff(%o6, x)"
        );
    }

    #[test]
    fn multidigit_label() {
        assert_eq!(
            rw("%o12", &ctx_map(&[(12, "%o24")])),
            "%o24"
        );
    }

    // ── Combined & edge cases ───────────────────────────────────────

    #[test]
    fn bare_and_display_combined() {
        assert_eq!(
            rw("% + %o1", &ctx_full(Some("%o10"), &[(1, "%o6")])),
            "%o10 + %o6"
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(rw("", &ctx_bare(Some("%o6"))), "");
    }

    #[test]
    fn no_labels_passthrough() {
        assert_eq!(
            rw("x^2 + 1", &ctx_full(None, &[])),
            "x^2 + 1"
        );
    }

    #[test]
    fn multiline_expression() {
        assert_eq!(
            rw("f(x) :=\n  %o1 + %", &ctx_full(Some("%o6"), &[(1, "%o3")])),
            "f(x) :=\n  %o3 + %o6"
        );
    }

    #[test]
    fn bare_percent_before_semicolon() {
        assert_eq!(rw("%;", &ctx_bare(Some("%o6"))), "%o6;");
    }

    #[test]
    fn bare_percent_before_dollar() {
        assert_eq!(rw("%$", &ctx_bare(Some("%o6"))), "%o6$");
    }

    #[test]
    fn percent_in_string_literal() {
        assert_eq!(
            rw("\"hello %\"", &ctx_bare(Some("%o6"))),
            "\"hello %\""
        );
    }

    #[test]
    fn display_label_in_string_literal() {
        assert_eq!(
            rw(r#"print("see %o1"), %o1"#, &ctx_map(&[(1, "%o6")])),
            r#"print("see %o1"), %o6"#
        );
    }

    #[test]
    fn escaped_quote_does_not_end_string() {
        assert_eq!(
            rw(r#""say \"%\"" + %"#, &ctx_bare(Some("%o6"))),
            r#""say \"%\"" + %o6"#
        );
    }

    #[test]
    fn labels_in_comments_preserved() {
        assert_eq!(
            rw(
                "/* uses % and %o1 /* nested % */ */ % + %o1",
                &ctx_full(Some("%o10"), &[(1, "%o6")])
            ),
            "/* uses % and %o1 /* nested % */ */ %o10 + %o6"
        );
    }

    #[test]
    fn adjacent_percent_and_label() {
        assert_eq!(
            rw("% + %o1 + %", &ctx_full(Some("%o10"), &[(1, "%o6")])),
            "%o10 + %o6 + %o10"
        );
    }

    #[test]
    fn real_label_numbers_higher_than_display() {
        assert_eq!(rw("%o1", &ctx_map(&[(1, "%o50")])), "%o50");
    }

    #[test]
    fn label_map_empty() {
        assert!(rw_err("%o1 + %o2", &ctx_map(&[])).starts_with("Out[1]"));
    }

    // ── Ordering — no double-rewrite ────────────────────────────────

    #[test]
    fn no_double_rewrite() {
        // Bare % → %o6, but execution count 6 maps to %o99.
        // Because display labels are replaced FIRST (on the original input),
        // the bare % expansion to %o6 is NOT re-matched.
        assert_eq!(
            rw("%", &ctx_full(Some("%o6"), &[(6, "%o99")])),
            "%o6"
        );
    }

    // ── Unicode preservation ────────────────────────────────────────

    #[test]
    fn unicode_preserved_in_strings() {
        // Multi-byte UTF-8 chars like → (E2 86 92) must not be garbled
        assert_eq!(
            rw(r#"[title, "error → 0 as T → ∞"]"#, &ctx_bare(None)),
            r#"[title, "error → 0 as T → ∞"]"#
        );
    }

    #[test]
    fn unicode_greek_preserved() {
        assert_eq!(
            rw("sin(theta) + % * alpha", &ctx_bare(Some("%o6"))),
            "sin(theta) + %o6 * alpha"
        );
    }
}
