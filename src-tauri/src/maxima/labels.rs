use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Maps display execution counts to real Maxima labels, and tracks
/// the previous cell's output label for bare `%` resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelContext {
    /// Display execution count → real Maxima output label (e.g. 1 → "%o6")
    pub label_map: HashMap<u32, String>,
    /// The real output label of the most recent previous cell (for bare `%`)
    pub previous_output_label: Option<String>,
}

static DISPLAY_LABEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%([oi])(\d+)").unwrap());

/// Rewrite label references in a Maxima expression.
///
/// 1. Replace display `%oN`/`%iN` labels using `label_map` (first).
/// 2. Replace bare `%` with `previous_output_label` (second).
///
/// Order matters: display labels are replaced first on the original input,
/// then bare `%` is expanded. This prevents double-rewriting where bare `%`
/// expands to e.g. `%o6` and then the display label regex re-matches it.
pub fn rewrite_labels(input: &str, ctx: &LabelContext) -> String {
    // Step 1: replace display %oN / %iN
    let result = DISPLAY_LABEL_RE.replace_all(input, |caps: &regex::Captures| {
        let kind = &caps[1]; // "o" or "i"
        let num: u32 = caps[2].parse().unwrap_or(0);
        if let Some(real_label) = ctx.label_map.get(&num) {
            // Extract the number from the real label (e.g. "%o6" → "6")
            let real_num = real_label.trim_start_matches("%o");
            format!("%{kind}{real_num}")
        } else {
            caps[0].to_string()
        }
    });

    // Step 2: replace bare %
    // Match `%` NOT followed by `%`, a letter, digit, or underscore.
    // The Rust regex crate doesn't support lookahead, so we do this manually.
    if let Some(ref prev) = ctx.previous_output_label {
        replace_bare_percent(&result, prev)
    } else {
        result.into_owned()
    }
}

/// Replace bare `%` (not followed by `%`, letter, digit, or `_`) with `replacement`.
fn replace_bare_percent(input: &str, replacement: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let next = bytes.get(i + 1).copied();
            match next {
                // %% is a Maxima construct — emit both and skip past
                Some(b'%') => {
                    result.push_str("%%");
                    i += 2;
                }
                // Followed by letter, digit, or underscore — not bare
                Some(c) if matches!(c, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') => {
                    result.push('%');
                    i += 1;
                }
                // Bare % (followed by non-identifier char or end of string)
                _ => {
                    result.push_str(replacement);
                    i += 1;
                }
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
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

    // ── Bare % replacement ──────────────────────────────────────────

    #[test]
    fn bare_percent_replaced() {
        assert_eq!(rewrite_labels("% + 1", &ctx_bare(Some("%o6"))), "%o6 + 1");
    }

    #[test]
    fn bare_percent_at_end() {
        assert_eq!(rewrite_labels("1 + %", &ctx_bare(Some("%o6"))), "1 + %o6");
    }

    #[test]
    fn bare_percent_standalone() {
        assert_eq!(rewrite_labels("%", &ctx_bare(Some("%o6"))), "%o6");
    }

    #[test]
    fn bare_percent_in_parens() {
        assert_eq!(rewrite_labels("f(%)", &ctx_bare(Some("%o6"))), "f(%o6)");
    }

    #[test]
    fn bare_percent_after_comma() {
        assert_eq!(
            rewrite_labels("[%, %]", &ctx_bare(Some("%o6"))),
            "[%o6, %o6]"
        );
    }

    #[test]
    fn bare_percent_no_previous() {
        assert_eq!(rewrite_labels("% + 1", &ctx_bare(None)), "% + 1");
    }

    #[test]
    fn multiple_bare_percent() {
        assert_eq!(
            rewrite_labels("% * % + %", &ctx_bare(Some("%o6"))),
            "%o6 * %o6 + %o6"
        );
    }

    // ── Special % forms preserved ───────────────────────────────────

    #[test]
    fn percent_e_preserved() {
        assert_eq!(rewrite_labels("%e + 1", &ctx_bare(Some("%o6"))), "%e + 1");
    }

    #[test]
    fn percent_pi_preserved() {
        assert_eq!(
            rewrite_labels("%pi * 2", &ctx_bare(Some("%o6"))),
            "%pi * 2"
        );
    }

    #[test]
    fn percent_i_preserved() {
        assert_eq!(rewrite_labels("%i", &ctx_bare(Some("%o6"))), "%i");
    }

    #[test]
    fn percent_percent_preserved() {
        assert_eq!(rewrite_labels("%%", &ctx_bare(Some("%o6"))), "%%");
    }

    #[test]
    fn percent_th_preserved() {
        assert_eq!(
            rewrite_labels("%th(1)", &ctx_bare(Some("%o6"))),
            "%th(1)"
        );
    }

    #[test]
    fn percent_gamma_preserved() {
        assert_eq!(
            rewrite_labels("%gamma", &ctx_bare(Some("%o6"))),
            "%gamma"
        );
    }

    #[test]
    fn percent_phi_preserved() {
        assert_eq!(rewrite_labels("%phi", &ctx_bare(Some("%o6"))), "%phi");
    }

    #[test]
    fn percent_with_underscore() {
        assert_eq!(
            rewrite_labels("%_var", &ctx_bare(Some("%o6"))),
            "%_var"
        );
    }

    // ── Display %oN / %iN rewriting ────────────────────────────────

    #[test]
    fn single_output_label() {
        assert_eq!(rewrite_labels("%o1", &ctx_map(&[(1, "%o6")])), "%o6");
    }

    #[test]
    fn multiple_output_labels() {
        assert_eq!(
            rewrite_labels("%o1 + %o2", &ctx_map(&[(1, "%o6"), (2, "%o10")])),
            "%o6 + %o10"
        );
    }

    #[test]
    fn input_label_rewrite() {
        assert_eq!(rewrite_labels("%i1", &ctx_map(&[(1, "%o6")])), "%i6");
    }

    #[test]
    fn mixed_io_labels() {
        assert_eq!(
            rewrite_labels("%o1 + %i2", &ctx_map(&[(1, "%o6"), (2, "%o10")])),
            "%o6 + %i10"
        );
    }

    #[test]
    fn unknown_label_preserved() {
        assert_eq!(rewrite_labels("%o99", &ctx_map(&[(1, "%o6")])), "%o99");
    }

    #[test]
    fn label_in_expression() {
        assert_eq!(
            rewrite_labels("diff(%o1, x)", &ctx_map(&[(1, "%o6")])),
            "diff(%o6, x)"
        );
    }

    #[test]
    fn multidigit_label() {
        assert_eq!(
            rewrite_labels("%o12", &ctx_map(&[(12, "%o24")])),
            "%o24"
        );
    }

    // ── Combined & edge cases ───────────────────────────────────────

    #[test]
    fn bare_and_display_combined() {
        assert_eq!(
            rewrite_labels("% + %o1", &ctx_full(Some("%o10"), &[(1, "%o6")])),
            "%o10 + %o6"
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(rewrite_labels("", &ctx_bare(Some("%o6"))), "");
    }

    #[test]
    fn no_labels_passthrough() {
        assert_eq!(
            rewrite_labels("x^2 + 1", &ctx_full(None, &[])),
            "x^2 + 1"
        );
    }

    #[test]
    fn multiline_expression() {
        assert_eq!(
            rewrite_labels("f(x) :=\n  %o1 + %", &ctx_full(Some("%o6"), &[(1, "%o3")])),
            "f(x) :=\n  %o3 + %o6"
        );
    }

    #[test]
    fn bare_percent_before_semicolon() {
        assert_eq!(rewrite_labels("%;", &ctx_bare(Some("%o6"))), "%o6;");
    }

    #[test]
    fn bare_percent_before_dollar() {
        assert_eq!(rewrite_labels("%$", &ctx_bare(Some("%o6"))), "%o6$");
    }

    #[test]
    fn percent_in_string_literal() {
        // We don't parse string literals — matches existing TS behavior
        assert_eq!(
            rewrite_labels("\"hello %\"", &ctx_bare(Some("%o6"))),
            "\"hello %o6\""
        );
    }

    #[test]
    fn adjacent_percent_and_label() {
        assert_eq!(
            rewrite_labels("% + %o1 + %", &ctx_full(Some("%o10"), &[(1, "%o6")])),
            "%o10 + %o6 + %o10"
        );
    }

    #[test]
    fn real_label_numbers_higher_than_display() {
        assert_eq!(rewrite_labels("%o1", &ctx_map(&[(1, "%o50")])), "%o50");
    }

    #[test]
    fn label_map_empty() {
        assert_eq!(
            rewrite_labels("%o1 + %o2", &ctx_map(&[])),
            "%o1 + %o2"
        );
    }

    // ── Ordering — no double-rewrite ────────────────────────────────

    #[test]
    fn no_double_rewrite() {
        // Bare % → %o6, but execution count 6 maps to %o99.
        // Because display labels are replaced FIRST (on the original input),
        // the bare % expansion to %o6 is NOT re-matched.
        assert_eq!(
            rewrite_labels("%", &ctx_full(Some("%o6"), &[(6, "%o99")])),
            "%o6"
        );
    }
}
