export interface MathSymbol {
  latex: string;   // without backslash, e.g. "alpha"
  unicode: string; // e.g. "α"
  maxima: string;  // Maxima-compatible name, e.g. "alpha"
}

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
  { latex: "equiv", unicode: "≡", maxima: "equiv" },
  { latex: "approx", unicode: "≈", maxima: "approx" },
  { latex: "sim", unicode: "∼", maxima: "sim" },
  { latex: "propto", unicode: "∝", maxima: "propto" },

  // Set theory
  { latex: "in", unicode: "∈", maxima: "in" },
  { latex: "notin", unicode: "∉", maxima: "notin" },
  { latex: "subset", unicode: "⊂", maxima: "subset" },
  { latex: "subseteq", unicode: "⊆", maxima: "subseteq" },
  { latex: "supset", unicode: "⊃", maxima: "supset" },
  { latex: "supseteq", unicode: "⊇", maxima: "supseteq" },
  { latex: "cup", unicode: "∪", maxima: "union" },
  { latex: "cap", unicode: "∩", maxima: "intersection" },
  { latex: "emptyset", unicode: "∅", maxima: "emptyset" },
  { latex: "setminus", unicode: "∖", maxima: "setdifference" },

  // Logic
  { latex: "land", unicode: "∧", maxima: "and" },
  { latex: "lor", unicode: "∨", maxima: "or" },
  { latex: "neg", unicode: "¬", maxima: "not" },
  { latex: "implies", unicode: "⟹", maxima: "implies" },
  { latex: "iff", unicode: "⟺", maxima: "iff" },
  { latex: "forall", unicode: "∀", maxima: "forall" },
  { latex: "exists", unicode: "∃", maxima: "exists" },
  { latex: "bot", unicode: "⊥", maxima: "bot" },
  { latex: "top", unicode: "⊤", maxima: "top" },

  // Calculus / analysis
  { latex: "nabla", unicode: "∇", maxima: "nabla" },
  { latex: "partial", unicode: "∂", maxima: "del" },
  { latex: "pm", unicode: "±", maxima: "pm" },

  // Arithmetic operators
  { latex: "times", unicode: "×", maxima: "*" },
  { latex: "cdot", unicode: "·", maxima: "*" },
  { latex: "div", unicode: "÷", maxima: "/" },

  // Miscellaneous
  { latex: "inf", unicode: "∞", maxima: "inf" },
  { latex: "infty", unicode: "∞", maxima: "inf" },
  { latex: "parallel", unicode: "∥", maxima: "parallel" },
  { latex: "perp", unicode: "⊥", maxima: "bot" },

  // Arrows
  { latex: "to", unicode: "→", maxima: "to" },
  { latex: "rightarrow", unicode: "→", maxima: "to" },
  { latex: "Rightarrow", unicode: "⇒", maxima: "implies" },
  { latex: "Leftrightarrow", unicode: "⇔", maxima: "iff" },

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

/** Symbols handled by the simple per-character regex replacement.
 *  Subscript/superscript chars are excluded — they need run-grouping
 *  (e.g. ₁₂ → [12] not [1][2]) and are handled by dedicated functions. */
const REGEX_SYMBOLS = MATH_SYMBOLS.filter(
  (s) => !s.latex.startsWith("_") && !s.latex.startsWith("^"),
);

/** Map from Unicode character to Maxima-compatible name */
export const UNICODE_TO_MAXIMA: Map<string, string> = new Map(
  REGEX_SYMBOLS.map((s) => [s.unicode, s.maxima]),
);

/** Regex matching any mapped Unicode symbol character */
export const UNICODE_SYMBOL_RE: RegExp = new RegExp(
  "[" + [...new Set(REGEX_SYMBOLS.map((s) => s.unicode))].map(escapeRegex).join("") + "]",
  "g",
);

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Map from Unicode subscript digit to ASCII digit */
const SUBSCRIPT_DIGITS: Record<string, string> = {
  "₀": "0", "₁": "1", "₂": "2", "₃": "3", "₄": "4",
  "₅": "5", "₆": "6", "₇": "7", "₈": "8", "₉": "9",
};

/** Replace runs of Unicode subscript digits (₀-₉) with Maxima subscript syntax [digits]. */
function replaceSubscriptDigits(input: string): string {
  let result = "";
  let inSubscript = false;
  for (const ch of input) {
    const digit = SUBSCRIPT_DIGITS[ch];
    if (digit !== undefined) {
      if (!inSubscript) { result += "["; inSubscript = true; }
      result += digit;
    } else {
      if (inSubscript) { result += "]"; inSubscript = false; }
      result += ch;
    }
  }
  if (inSubscript) result += "]";
  return result;
}

/** Map from Unicode superscript character to ASCII equivalent */
const SUPERSCRIPT_CHARS: Record<string, string> = {
  "⁰": "0", "¹": "1", "²": "2", "³": "3", "⁴": "4",
  "⁵": "5", "⁶": "6", "⁷": "7", "⁸": "8", "⁹": "9",
  "⁻": "-", "⁺": "+", "ⁿ": "n",
};

/** Replace runs of Unicode superscript chars with Maxima power syntax ^(...). */
function replaceSuperscripts(input: string): string {
  let result = "";
  let run = "";
  for (const ch of input) {
    const ascii = SUPERSCRIPT_CHARS[ch];
    if (ascii !== undefined) {
      run += ascii;
    } else {
      if (run) { result += "^(" + run + ")"; run = ""; }
      result += ch;
    }
  }
  if (run) result += "^(" + run + ")";
  return result;
}

/**
 * Replace Unicode math symbols with their Maxima-compatible ASCII names.
 * String literals (delimited by `"`) are left untouched so that Unicode
 * characters in plot labels, print messages, etc. pass through verbatim.
 */
export function unicodeToMaxima(expr: string): string {
  // Split on string literals preserving delimiters: alternating code/string segments
  const parts = expr.split(/("(?:[^"]*)")/);
  return parts
    .map((part) =>
      part.startsWith('"')
        ? part // string literal — preserve unchanged
        : replaceSuperscripts(
            replaceSubscriptDigits(
              part.replace(UNICODE_SYMBOL_RE, (ch) => UNICODE_TO_MAXIMA.get(ch) ?? ch),
            ),
          ),
    )
    .join("");
}

/**
 * Build a Maxima expression that configures texput for all symbols,
 * so e.g. `theta` renders as `\theta` instead of Maxima's default `\vartheta`.
 * Returns a `$`-terminated block that produces no visible output.
 */
/** Names that are Maxima keywords/operators — skip texput for these */
const SKIP_TEXPUT = new Set([
  "and", "or", "not", "in", "true", "false", "inf",
]);

export function buildTexputInit(): string {
  // Deduplicate by maxima name; skip constants (%pi), operators (<=, *, etc.),
  // and Maxima keywords (and, or, not, etc.) that already have TeX representations.
  const seen = new Set<string>();
  const calls: string[] = [];
  for (const s of MATH_SYMBOLS) {
    if (s.maxima.startsWith("%") || seen.has(s.maxima)) continue;
    if (!/^[a-zA-Z_]\w*$/.test(s.maxima)) continue; // skip operators like <=, #, *, /
    if (SKIP_TEXPUT.has(s.maxima)) continue;
    seen.add(s.maxima);
    // In Maxima strings, \\ produces a literal backslash
    calls.push(`texput(${s.maxima}, "\\\\${s.latex}")`);
  }
  return calls.join("$ ") + "$";
}
