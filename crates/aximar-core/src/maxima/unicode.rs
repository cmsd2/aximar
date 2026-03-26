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
}
