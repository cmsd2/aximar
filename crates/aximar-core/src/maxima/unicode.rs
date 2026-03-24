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

/// Replace Unicode math symbols with their Maxima-compatible ASCII names.
pub fn unicode_to_maxima(expr: &str) -> String {
    let mut result = expr.to_string();
    for &(unicode, maxima) in UNICODE_MAP {
        if result.contains(unicode) {
            result = result.replace(unicode, maxima);
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
}
