export interface MathSymbol {
  latex: string;   // without backslash, e.g. "alpha"
  unicode: string; // e.g. "α"
  maxima: string;  // Maxima-compatible name, e.g. "alpha"
}

/** Symbols offered by `\name` completion and the command palette.  Each
 *  is one the backend translates (see docs/unicode-input.md); `maxima` is
 *  what it becomes.  A Rust test in aximar-core checks this list against
 *  the translator, so don't add a character the table rejects. */
export const MATH_SYMBOLS: MathSymbol[] = [
  // Lowercase Greek
  { latex: "alpha", unicode: "α", maxima: "alpha" },
  { latex: "beta", unicode: "β", maxima: "beta" },
  { latex: "gamma", unicode: "γ", maxima: "gamma" },
  { latex: "delta", unicode: "δ", maxima: "delta" },
  { latex: "epsilon", unicode: "ε", maxima: "epsilon" },
  { latex: "zeta", unicode: "ζ", maxima: "zeta" },
  { latex: "eta", unicode: "η", maxima: "eta" },
  { latex: "theta", unicode: "θ", maxima: "theta" },
  { latex: "iota", unicode: "ι", maxima: "iota" },
  { latex: "kappa", unicode: "κ", maxima: "kappa" },
  { latex: "lambda", unicode: "λ", maxima: "lambda" },
  { latex: "mu", unicode: "μ", maxima: "mu" },
  { latex: "nu", unicode: "ν", maxima: "nu" },
  { latex: "xi", unicode: "ξ", maxima: "xi" },
  { latex: "pi", unicode: "π", maxima: "%pi" },
  { latex: "rho", unicode: "ρ", maxima: "rho" },
  { latex: "sigma", unicode: "σ", maxima: "sigma" },
  { latex: "tau", unicode: "τ", maxima: "tau" },
  { latex: "upsilon", unicode: "υ", maxima: "upsilon" },
  { latex: "phi", unicode: "φ", maxima: "phi" },
  { latex: "chi", unicode: "χ", maxima: "chi" },
  { latex: "psi", unicode: "ψ", maxima: "psi" },
  { latex: "omega", unicode: "ω", maxima: "omega" },

  // Uppercase Greek (commonly used)
  { latex: "Gamma", unicode: "Γ", maxima: "Gamma" },
  { latex: "Delta", unicode: "Δ", maxima: "Delta" },
  { latex: "Theta", unicode: "Θ", maxima: "Theta" },
  { latex: "Lambda", unicode: "Λ", maxima: "Lambda" },
  { latex: "Xi", unicode: "Ξ", maxima: "Xi" },
  { latex: "Pi", unicode: "Π", maxima: "Pi" },
  { latex: "Sigma", unicode: "Σ", maxima: "Sigma" },
  { latex: "Phi", unicode: "Φ", maxima: "Phi" },
  { latex: "Psi", unicode: "Ψ", maxima: "Psi" },
  { latex: "Omega", unicode: "Ω", maxima: "Omega" },

  // Relations
  { latex: "leq", unicode: "≤", maxima: "<=" },
  { latex: "geq", unicode: "≥", maxima: ">=" },
  { latex: "neq", unicode: "≠", maxima: "#" },

  // Logic
  { latex: "land", unicode: "∧", maxima: "and" },
  { latex: "lor", unicode: "∨", maxima: "or" },
  { latex: "neg", unicode: "¬", maxima: "not" },

  // Roots and physics
  { latex: "sqrt", unicode: "√", maxima: "sqrt" },
  { latex: "hbar", unicode: "ℏ", maxima: "hbar" },
  { latex: "ell", unicode: "ℓ", maxima: "ell" },

  // Arithmetic operators
  { latex: "times", unicode: "×", maxima: "*" },
  { latex: "cdot", unicode: "·", maxima: "*" },
  { latex: "div", unicode: "÷", maxima: "/" },

  // Miscellaneous
  { latex: "inf", unicode: "∞", maxima: "inf" },
  { latex: "infty", unicode: "∞", maxima: "inf" },

  // Subscript digits (for variable subscripts like T₀, x₁)
  { latex: "_0", unicode: "₀", maxima: "[0]" },
  { latex: "_1", unicode: "₁", maxima: "[1]" },
  { latex: "_2", unicode: "₂", maxima: "[2]" },
  { latex: "_3", unicode: "₃", maxima: "[3]" },
  { latex: "_4", unicode: "₄", maxima: "[4]" },
  { latex: "_5", unicode: "₅", maxima: "[5]" },
  { latex: "_6", unicode: "₆", maxima: "[6]" },
  { latex: "_7", unicode: "₇", maxima: "[7]" },
  { latex: "_8", unicode: "₈", maxima: "[8]" },
  { latex: "_9", unicode: "₉", maxima: "[9]" },

  // Superscript characters (translated to ^(n) power syntax)
  { latex: "^0", unicode: "⁰", maxima: "^(0)" },
  { latex: "^1", unicode: "¹", maxima: "^(1)" },
  { latex: "^2", unicode: "²", maxima: "^(2)" },
  { latex: "^3", unicode: "³", maxima: "^(3)" },
  { latex: "^4", unicode: "⁴", maxima: "^(4)" },
  { latex: "^5", unicode: "⁵", maxima: "^(5)" },
  { latex: "^6", unicode: "⁶", maxima: "^(6)" },
  { latex: "^7", unicode: "⁷", maxima: "^(7)" },
  { latex: "^8", unicode: "⁸", maxima: "^(8)" },
  { latex: "^9", unicode: "⁹", maxima: "^(9)" },
  { latex: "^-", unicode: "⁻", maxima: "^(-" },
  { latex: "^+", unicode: "⁺", maxima: "^(+" },
  { latex: "^n", unicode: "ⁿ", maxima: "^(n)" },
];
