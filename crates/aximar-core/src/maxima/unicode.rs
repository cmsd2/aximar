/// Translate Unicode math symbols to Maxima-compatible ASCII names.
///
/// This mirrors the TypeScript `unicodeToMaxima()` in `src/lib/math-symbols.ts`.
/// Both tables must be kept in sync.

/// (unicode_char, maxima_name)
const UNICODE_MAP: &[(&str, &str)] = &[
    // Lowercase Greek
    ("α", "alpha"),
    ("β", "beta"),
    ("γ", "gamma"),
    ("δ", "delta"),
    ("ε", "epsilon"),
    ("ζ", "zeta"),
    ("η", "eta"),
    ("θ", "theta"),
    ("ι", "iota"),
    ("κ", "kappa"),
    ("λ", "lambda"),
    ("μ", "mu"),
    ("ν", "nu"),
    ("ξ", "xi"),
    ("π", "%pi"),
    ("ρ", "rho"),
    ("σ", "sigma"),
    ("τ", "tau"),
    ("υ", "upsilon"),
    ("φ", "phi"),
    ("χ", "chi"),
    ("ψ", "psi"),
    ("ω", "omega"),
    // Uppercase Greek
    ("Γ", "Gamma"),
    ("Δ", "Delta"),
    ("Θ", "Theta"),
    ("Λ", "Lambda"),
    ("Ξ", "Xi"),
    ("Π", "Pi"),
    ("Σ", "Sigma"),
    ("Φ", "Phi"),
    ("Ψ", "Psi"),
    ("Ω", "Omega"),
    // Relations
    ("≤", "<="),
    ("≥", ">="),
    ("≠", "#"),
    // Arithmetic operators
    ("×", "*"),
    ("·", "*"),
    ("÷", "/"),
    // Miscellaneous
    ("∞", "inf"),
];

/// (maxima_name, latex_command) pairs for `texput` initialization.
/// Maxima's default TeX rendering uses `\vartheta` for `theta`, etc.
/// These overrides ensure standard LaTeX names are used.
/// Each LaTeX command needs a double backslash because Maxima's string parser
/// treats `\` as an escape character — `\\` in a Maxima string literal produces
/// a single literal backslash.
const TEXPUT_MAP: &[(&str, &str)] = &[
    // Lowercase Greek
    ("alpha", "\\\\alpha"),
    ("beta", "\\\\beta"),
    ("gamma", "\\\\gamma"),
    ("delta", "\\\\delta"),
    ("epsilon", "\\\\epsilon"),
    ("zeta", "\\\\zeta"),
    ("eta", "\\\\eta"),
    ("theta", "\\\\theta"),
    ("iota", "\\\\iota"),
    ("kappa", "\\\\kappa"),
    ("lambda", "\\\\lambda"),
    ("mu", "\\\\mu"),
    ("nu", "\\\\nu"),
    ("xi", "\\\\xi"),
    ("rho", "\\\\rho"),
    ("sigma", "\\\\sigma"),
    ("tau", "\\\\tau"),
    ("upsilon", "\\\\upsilon"),
    ("phi", "\\\\phi"),
    ("chi", "\\\\chi"),
    ("psi", "\\\\psi"),
    ("omega", "\\\\omega"),
    // Uppercase Greek
    ("Gamma", "\\\\Gamma"),
    ("Delta", "\\\\Delta"),
    ("Theta", "\\\\Theta"),
    ("Lambda", "\\\\Lambda"),
    ("Xi", "\\\\Xi"),
    ("Pi", "\\\\Pi"),
    ("Sigma", "\\\\Sigma"),
    ("Phi", "\\\\Phi"),
    ("Psi", "\\\\Psi"),
    ("Omega", "\\\\Omega"),
];

/// Build a Maxima expression that configures `texput` for all Greek symbols,
/// so e.g. `theta` renders as `\theta` instead of Maxima's default `\vartheta`.
/// Returns a `$`-terminated block that produces no visible output.
pub fn build_texput_init() -> String {
    TEXPUT_MAP
        .iter()
        .map(|(name, tex)| format!("texput({}, \"{}\")", name, tex))
        .collect::<Vec<_>>()
        .join("$ ")
        + "$"
}

/// Map a Unicode subscript digit (₀-₉) to its ASCII digit equivalent.
fn subscript_digit(ch: char) -> Option<char> {
    match ch {
        '₀' => Some('0'),
        '₁' => Some('1'),
        '₂' => Some('2'),
        '₃' => Some('3'),
        '₄' => Some('4'),
        '₅' => Some('5'),
        '₆' => Some('6'),
        '₇' => Some('7'),
        '₈' => Some('8'),
        '₉' => Some('9'),
        _ => None,
    }
}

/// Replace runs of Unicode subscript digits (₀-₉) with Maxima subscript
/// syntax `[digits]`.  E.g. `T₀` → `T[0]`, `x₁₂` → `x[12]`.
fn replace_subscript_digits(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_subscript = false;

    for ch in input.chars() {
        if let Some(digit) = subscript_digit(ch) {
            if !in_subscript {
                result.push('[');
                in_subscript = true;
            }
            result.push(digit);
        } else {
            if in_subscript {
                result.push(']');
                in_subscript = false;
            }
            result.push(ch);
        }
    }

    if in_subscript {
        result.push(']');
    }

    result
}

/// Map a Unicode superscript character to its ASCII equivalent.
fn superscript_char(ch: char) -> Option<char> {
    match ch {
        '⁰' => Some('0'),
        '¹' => Some('1'),
        '²' => Some('2'),
        '³' => Some('3'),
        '⁴' => Some('4'),
        '⁵' => Some('5'),
        '⁶' => Some('6'),
        '⁷' => Some('7'),
        '⁸' => Some('8'),
        '⁹' => Some('9'),
        '⁻' => Some('-'),
        '⁺' => Some('+'),
        'ⁿ' => Some('n'),
        _ => None,
    }
}

/// Replace runs of Unicode superscript characters with Maxima power syntax `^(...)`.
/// E.g. `x²` → `x^(2)`, `x⁻¹` → `x^(-1)`, `a²³` → `a^(23)`.
fn replace_superscripts(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut run = String::new();

    for ch in input.chars() {
        if let Some(ascii) = superscript_char(ch) {
            run.push(ascii);
        } else {
            if !run.is_empty() {
                result.push_str("^(");
                result.push_str(&run);
                result.push(')');
                run.clear();
            }
            result.push(ch);
        }
    }

    if !run.is_empty() {
        result.push_str("^(");
        result.push_str(&run);
        result.push(')');
    }

    result
}

/// Replace Unicode math symbols with their Maxima-compatible ASCII names.
///
/// String literals (delimited by `"`) are left untouched so that Unicode
/// characters in plot labels, print messages, etc. pass through to gnuplot
/// and other consumers verbatim.
pub fn unicode_to_maxima(expr: &str) -> String {
    // Split the expression into alternating segments: code, string, code, string, ...
    // Maxima strings are delimited by double quotes with no escape for the quote itself.
    let mut segments: Vec<String> = Vec::new();
    let mut in_string = false;
    let mut current = String::new();

    for ch in expr.chars() {
        if ch == '"' {
            if in_string {
                // End of string literal — include closing quote in current segment
                current.push(ch);
                segments.push(current);
                current = String::new();
                in_string = false;
            } else {
                // Start of string literal — flush code segment first
                segments.push(current);
                current = String::new();
                current.push(ch);
                in_string = true;
            }
        } else {
            current.push(ch);
        }
    }
    segments.push(current);

    // Apply Unicode→Maxima replacements only to non-string segments.
    // String segments (starting with `"`) are passed through unchanged.
    let mut result = String::with_capacity(expr.len());
    for seg in &segments {
        if seg.starts_with('"') {
            result.push_str(seg);
        } else {
            let mut replaced = seg.clone();
            for &(unicode, maxima) in UNICODE_MAP {
                if replaced.contains(unicode) {
                    replaced = replaced.replace(unicode, maxima);
                }
            }
            // Convert subscript digits (₀-₉) to Maxima subscript syntax [n]
            replaced = replace_subscript_digits(&replaced);
            // Convert superscript chars (²,⁻¹,ⁿ etc.) to Maxima power syntax ^(n)
            replaced = replace_superscripts(&replaced);
            result.push_str(&replaced);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greek_letters() {
        assert_eq!(unicode_to_maxima("sin(θ)"), "sin(theta)");
        assert_eq!(unicode_to_maxima("2*π"), "2*%pi");
        assert_eq!(unicode_to_maxima("α + β"), "alpha + beta");
    }

    #[test]
    fn operators() {
        assert_eq!(unicode_to_maxima("3 × 4"), "3 * 4");
        assert_eq!(unicode_to_maxima("x ≤ 5"), "x <= 5");
        assert_eq!(unicode_to_maxima("a ≠ b"), "a # b");
    }

    #[test]
    fn infinity() {
        assert_eq!(unicode_to_maxima("integrate(f(x), x, 0, ∞)"), "integrate(f(x), x, 0, inf)");
    }

    #[test]
    fn mixed() {
        assert_eq!(
            unicode_to_maxima("integrate(sin(θ), θ, 0, 2*π)"),
            "integrate(sin(theta), theta, 0, 2*%pi)"
        );
    }

    #[test]
    fn no_unicode_passthrough() {
        let plain = "diff(sin(x), x)";
        assert_eq!(unicode_to_maxima(plain), plain);
    }

    #[test]
    fn uppercase_greek() {
        assert_eq!(unicode_to_maxima("Γ(x)"), "Gamma(x)");
        assert_eq!(unicode_to_maxima("Ω"), "Omega");
    }

    #[test]
    fn texput_init_escaping() {
        let init = build_texput_init();
        // Each entry should produce: texput(name, "\\latex")
        // where \\ is a Maxima string escape for a literal backslash
        assert!(init.contains(r#"texput(theta, "\\theta")"#));
        assert!(init.contains(r#"texput(Gamma, "\\Gamma")"#));
        // Should end with $
        assert!(init.ends_with('$'));
    }

    #[test]
    fn string_literals_preserved() {
        // Unicode inside string literals should NOT be translated
        assert_eq!(
            unicode_to_maxima(r#"print("τ/T")"#),
            r#"print("τ/T")"#
        );
        assert_eq!(
            unicode_to_maxima(r#"[xlabel, "α → ∞"]"#),
            r#"[xlabel, "α → ∞"]"#
        );
    }

    #[test]
    fn mixed_code_and_strings() {
        // Code outside strings is translated, strings are preserved
        assert_eq!(
            unicode_to_maxima(r#"plot2d(sin(θ), [xlabel, "θ (radians)"])"#),
            r#"plot2d(sin(theta), [xlabel, "θ (radians)"])"#
        );
    }

    #[test]
    fn multiple_strings() {
        assert_eq!(
            unicode_to_maxima(r#"f(τ) + "τ" + g(τ) + "τ""#),
            r#"f(tau) + "τ" + g(tau) + "τ""#
        );
    }

    #[test]
    fn subscript_digit_single() {
        assert_eq!(unicode_to_maxima("T₀"), "T[0]");
        assert_eq!(unicode_to_maxima("x₁ + x₂"), "x[1] + x[2]");
    }

    #[test]
    fn subscript_digit_multi() {
        assert_eq!(unicode_to_maxima("a₁₂"), "a[12]");
    }

    #[test]
    fn subscript_digit_with_greek() {
        assert_eq!(unicode_to_maxima("ω₀"), "omega[0]");
        assert_eq!(unicode_to_maxima("τ₁ + τ₂"), "tau[1] + tau[2]");
    }

    #[test]
    fn subscript_digit_in_string_preserved() {
        assert_eq!(
            unicode_to_maxima(r#"T₀ + "T₀""#),
            r#"T[0] + "T₀""#
        );
    }

    #[test]
    fn superscript_single_digit() {
        assert_eq!(unicode_to_maxima("x²"), "x^(2)");
        assert_eq!(unicode_to_maxima("x³ + y²"), "x^(3) + y^(2)");
    }

    #[test]
    fn superscript_multi_digit() {
        assert_eq!(unicode_to_maxima("x²³"), "x^(23)");
    }

    #[test]
    fn superscript_negative() {
        assert_eq!(unicode_to_maxima("x⁻¹"), "x^(-1)");
        assert_eq!(unicode_to_maxima("r⁻²"), "r^(-2)");
    }

    #[test]
    fn superscript_n() {
        assert_eq!(unicode_to_maxima("xⁿ"), "x^(n)");
    }

    #[test]
    fn superscript_with_greek() {
        assert_eq!(unicode_to_maxima("θ²"), "theta^(2)");
        assert_eq!(unicode_to_maxima("ω⁻¹"), "omega^(-1)");
    }

    #[test]
    fn superscript_in_string_preserved() {
        assert_eq!(
            unicode_to_maxima(r#"x² + "x²""#),
            r#"x^(2) + "x²""#
        );
    }

    #[test]
    fn mixed_sub_and_superscript() {
        assert_eq!(unicode_to_maxima("x₁² + x₂²"), "x[1]^(2) + x[2]^(2)");
    }
}
