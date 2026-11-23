//! Unicode in Maxima code.
//!
//! Aximar translates non-ASCII characters in code before Maxima sees them,
//! and rejects the ones with no clear meaning.  `docs/unicode-input.md` is
//! the reference: what each character does, how it displays and why.
//! Notebooks depend on this, so a change to a mapping is a breaking change.
//!
//! Strings and comments are left as typed.  Each piece of code goes through
//! three passes:
//!
//! 1. Normalise: NFC, spaces, invisible characters, full-width forms,
//!    ligatures, variant letters and mathematical italic letters.
//! 2. Names: a Greek letter that is a whole name (optionally with a number)
//!    becomes its ASCII name; inside a longer name it stays as typed.
//! 3. Symbols: operator forms, superscripts, subscripts and `√`; anything
//!    else non-ASCII is rejected with a hint.

use unicode_normalization::UnicodeNormalization;

use super::code_spans::{spans, Span};

/// A character, or a use of one, that Maxima code can't contain.  The
/// message names it and says what to type instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedCharacter(pub String);

/// (maxima_name, latex) pairs for `texput`, for names whose default TeX
/// differs from the character typed.  Maxima renders `theta` as `\vartheta`
/// by default; it already gives the other Greek names their letters, and its
/// `\varepsilon` and `\varphi` match the glyphs of `ε` and `φ`.  Each
/// backslash is doubled for Maxima's string syntax.
const TEXPUT_MAP: &[(&str, &str)] = &[
    ("theta", "\\\\theta"),
    ("hbar", "\\\\hbar"),
    ("ell", "\\\\ell"),
];

/// A Maxima statement that sets up `texput` for the names above.
/// Returns a `$`-terminated block that produces no visible output.
pub fn build_texput_init() -> String {
    TEXPUT_MAP
        .iter()
        .map(|(name, tex)| format!("texput({}, \"{}\")", name, tex))
        .collect::<Vec<_>>()
        .join("$ ")
        + "$"
}

/// Translate the Unicode in Maxima code, or say why it can't be sent.
pub fn unicode_to_maxima(input: &str) -> Result<String, UnsupportedCharacter> {
    let multi_line = input.contains('\n');
    let mut out = String::with_capacity(input.len());
    for span in spans(input) {
        match span {
            Span::Text(text) => out.push_str(text),
            Span::Code(code) if code.is_ascii() => out.push_str(code),
            Span::Code(code) => {
                let first_line = out.matches('\n').count() + 1;
                let line = |chars: &[char], at: usize| {
                    multi_line
                        .then(|| first_line + chars[..at].iter().filter(|&&c| c == '\n').count())
                };
                translate_code(code, &line, &mut out)?;
            }
        }
    }
    Ok(out)
}

// ── Pass 1: normalise ────────────────────────────────────────────────

fn normalise(code: &str) -> Vec<char> {
    let mut chars = Vec::with_capacity(code.len());
    for c in code.nfc() {
        match c {
            // Spaces of other widths, as pasted from documents
            '\u{00A0}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' => {
                chars.push(' ')
            }
            // Invisible: zero-width characters, word joiner, BOM, soft hyphen
            '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}' | '\u{00AD}' => {}
            '\u{2028}' | '\u{2029}' => chars.push('\n'),
            // Full-width ASCII, from CJK input methods
            '\u{FF01}'..='\u{FF5E}' => chars.push(shift(c, -0xFEE0)),
            '\u{FB00}' => chars.extend("ff".chars()),
            '\u{FB01}' => chars.extend("fi".chars()),
            '\u{FB02}' => chars.extend("fl".chars()),
            '\u{FB03}' => chars.extend("ffi".chars()),
            '\u{FB04}' => chars.extend("ffl".chars()),
            '\u{FB05}' | '\u{FB06}' => chars.extend("st".chars()),
            _ => chars.push(variant_of(c).unwrap_or(c)),
        }
    }
    chars
}

fn shift(c: char, by: i32) -> char {
    char::from_u32((c as i32 + by) as u32).unwrap_or(c)
}

/// The letter a variant form or mathematical italic letter stands for.
fn variant_of(c: char) -> Option<char> {
    let letter = match c {
        'ϐ' => 'β',
        'ϑ' => 'θ',
        'ϕ' => 'φ',
        'ϰ' => 'κ',
        'ϱ' => 'ρ',
        'ϴ' => 'Θ',
        'ϵ' => 'ε',
        'ς' => 'σ',
        'µ' => 'μ',
        '\u{2126}' => 'Ω', // ohm sign
        '∆' => 'Δ',        // increment, typed by macOS Option-J
        'ℎ' => 'h',        // Planck constant: mathematical italic h
        // Mathematical italic Latin; U+1D455 is unassigned (ℎ above)
        '\u{1D434}'..='\u{1D44D}' => shift(c, 'A' as i32 - 0x1D434),
        '\u{1D44E}'..='\u{1D467}' => shift(c, 'a' as i32 - 0x1D44E),
        // Mathematical italic Greek capitals: Α..Ρ, ϴ, Σ..Ω, ∇
        '\u{1D6E2}'..='\u{1D6F2}' => shift(c, 0x391 - 0x1D6E2),
        '\u{1D6F3}' => 'Θ',
        '\u{1D6F4}'..='\u{1D6FA}' => shift(c, 0x3A3 - 0x1D6F4),
        '\u{1D6FB}' => '∇',
        // Mathematical italic Greek small letters α..ω, then symbols
        '\u{1D6FC}'..='\u{1D714}' => {
            let letter = shift(c, 0x3B1 - 0x1D6FC);
            if letter == 'ς' {
                'σ'
            } else {
                letter
            }
        }
        '\u{1D715}' => '∂',
        '\u{1D716}' => 'ε',
        '\u{1D717}' => 'θ',
        '\u{1D718}' => 'κ',
        '\u{1D719}' => 'φ',
        '\u{1D71A}' => 'ρ',
        '\u{1D71B}' => 'ϖ',
        _ => return None,
    };
    Some(letter)
}

// ── Pass 2: names ────────────────────────────────────────────────────

/// The Maxima name for a Greek letter or letterlike symbol that is a whole
/// name.
fn name_for(c: char) -> Option<&'static str> {
    Some(match c {
        'α' => "alpha",
        'β' => "beta",
        'γ' => "gamma",
        'δ' => "delta",
        'ε' => "epsilon",
        'ζ' => "zeta",
        'η' => "eta",
        'θ' => "theta",
        'ι' => "iota",
        'κ' => "kappa",
        'λ' => "lambda",
        'μ' => "mu",
        'ν' => "nu",
        'ξ' => "xi",
        'ρ' => "rho",
        'σ' => "sigma",
        'τ' => "tau",
        'υ' => "upsilon",
        'φ' => "phi",
        'χ' => "chi",
        'ψ' => "psi",
        'ω' => "omega",
        'Γ' => "Gamma",
        'Δ' => "Delta",
        'Θ' => "Theta",
        'Λ' => "Lambda",
        'Ξ' => "Xi",
        'Π' => "Pi",
        'Σ' => "Sigma",
        'Φ' => "Phi",
        'Ψ' => "Psi",
        'Ω' => "Omega",
        'ϒ' => "Upsilon",
        'ℏ' => "hbar",
        'ℓ' => "ell",
        _ => return None,
    })
}

/// The Maxima constant a character stands for, as a whole name.
fn constant_for(c: char) -> Option<&'static str> {
    Some(match c {
        'π' => "%pi",
        'ⅈ' => "%i",
        'ⅇ' | 'ℯ' => "%e",
        _ => return None,
    })
}

/// Characters that can continue a Maxima name.  Superscripts and subscripts
/// are translated separately, so they end a name.
fn is_name_char(c: char) -> bool {
    (c.is_alphabetic() || c.is_ascii_digit() || c == '_' || c == '%')
        && superscript(c).is_none()
        && subscript(c).is_none()
}

/// Letters Maxima would accept in a name, but which Aximar rejects.
fn rejected_letter(c: char) -> Option<String> {
    let lookalike = match c {
        'Α' => Some('A'),
        'Β' => Some('B'),
        'Ε' => Some('E'),
        'Ζ' => Some('Z'),
        'Η' => Some('H'),
        'Ι' => Some('I'),
        'Κ' => Some('K'),
        'Μ' => Some('M'),
        'Ν' => Some('N'),
        'Ο' => Some('O'),
        'Ρ' => Some('P'),
        'Τ' => Some('T'),
        'Υ' => Some('Y'),
        'Χ' => Some('X'),
        'ο' => Some('o'),
        _ => None,
    };
    if let Some(latin) = lookalike {
        let mut hint = format!(
            "it looks like the Latin letter `{latin}` but would be a different name. Type `{latin}`."
        );
        if c == 'Υ' {
            hint.push_str(" For capital upsilon in maths, use `ϒ`.");
        }
        return Some(hint);
    }
    let hint = match c {
        'ϖ' => "this is a variant of π, not the constant. Use `π` for the constant, or a name for a variable.",
        'ⅉ' => "Maxima's imaginary unit is `%i`, which you can also type as `ⅈ`.",
        'ⅅ' | 'ⅆ' => "for a derivative write `diff(f, x)`; for an integral, `integrate(f, x)`.",
        'ℂ' | 'ℍ' | 'ℕ' | 'ℙ' | 'ℚ' | 'ℝ' | 'ℤ' | '\u{213C}'..='\u{2140}' | '\u{1D538}'..='\u{1D56B}' => {
            "Maxima has no number-set values. To say x is real, write `declare(x, real)` or `assume(x > 0)`; for an integer, `declare(n, integer)`."
        }
        // Other mathematical alphanumerics (italic was normalised already),
        // and script and fraktur letters in the letterlike block
        '\u{1D400}'..='\u{1D7FF}' | 'ℊ' | 'ℋ' | 'ℌ' | 'ℐ' | 'ℑ' | 'ℒ' | 'ℛ' | 'ℜ' | 'ℨ' | 'ℬ'
        | 'ℭ' | 'ℰ' | 'ℱ' | 'ℳ' | 'ℴ' => {
            "Maxima names have no style, so this would be a different name from the plain letter. Type the plain letter, or a name such as `xvec`."
        }
        // Superscript and modifier letters
        '\u{02B0}'..='\u{02FF}' | '\u{1D2C}'..='\u{1D61}' | '\u{1D9B}'..='\u{1DBF}' | 'ⁱ' => {
            "superscript letters have no meaning in Maxima. Write a power with `^`; for a transpose, `transpose(M)`."
        }
        // Subscript Greek letters
        '\u{1D66}'..='\u{1D6A}' => "write the index in brackets, for example `x[β]`.",
        'º' => "this looks like a degree sign. Maxima works in radians: write `30*%pi/180`.",
        _ => return None,
    };
    Some(hint.to_string())
}

/// Translate one name token, or reject it.  `next` is the next character
/// after the token that isn't a space.
fn translate_name(token: &[char], next: Option<char>) -> Result<String, (String, String)> {
    let text: String = token.iter().collect();
    let first = token[0];

    if first.is_ascii_digit() {
        // A number running into a translated letter: Maxima has no implicit
        // multiplication, and its own error would name `%pi`, not `π`
        if let Some(at) = token
            .iter()
            .position(|&c| name_for(c).is_some() || constant_for(c).is_some())
        {
            let (number, rest): (String, String) =
                (token[..at].iter().collect(), token[at..].iter().collect());
            return Err((
                text.clone(),
                format!("Maxima has no implicit multiplication: write `{number}*{rest}`."),
            ));
        }
    } else {
        let rest = &token[1..];
        if rest.is_empty() {
            if let Some(constant) = constant_for(first) {
                return Ok(constant.to_string());
            }
            if first == 'Γ' && next == Some('(') {
                return Ok("gamma".to_string());
            }
        }
        if let Some(name) = name_for(first) {
            let number = rest.iter().all(char::is_ascii_digit);
            let indexed = rest.first() == Some(&'_')
                && rest.len() > 1
                && rest[1..].iter().all(char::is_ascii_digit);
            if number || indexed {
                let rest: String = rest.iter().collect();
                return Ok(format!("{name}{rest}"));
            }
        }
    }

    for &c in token {
        if let Some(hint) = rejected_letter(c) {
            return Err((char_subject(c), hint));
        }
    }
    Ok(text)
}

// ── Pass 3: symbols ──────────────────────────────────────────────────

fn superscript(c: char) -> Option<char> {
    Some(match c {
        '⁰' => '0',
        '¹' => '1',
        '²' => '2',
        '³' => '3',
        '⁴' => '4',
        '⁵' => '5',
        '⁶' => '6',
        '⁷' => '7',
        '⁸' => '8',
        '⁹' => '9',
        '⁺' => '+',
        '⁻' => '-',
        'ⁿ' => 'n',
        _ => return None,
    })
}

fn subscript(c: char) -> Option<char> {
    Some(match c {
        '₀'..='₉' => shift(c, '0' as i32 - '₀' as i32),
        '₊' => '+',
        '₋' => '-',
        'ₐ' => 'a',
        'ₑ' => 'e',
        'ₒ' => 'o',
        'ₓ' => 'x',
        'ₕ' => 'h',
        'ₖ' => 'k',
        'ₗ' => 'l',
        'ₘ' => 'm',
        'ₙ' => 'n',
        'ₚ' => 'p',
        'ₛ' => 's',
        'ₜ' => 't',
        'ᵢ' => 'i',
        'ⱼ' => 'j',
        'ᵣ' => 'r',
        'ᵤ' => 'u',
        'ᵥ' => 'v',
        _ => return None,
    })
}

/// The Maxima operator for a symbol with one clear meaning.
fn operator_for(c: char) -> Option<&'static str> {
    Some(match c {
        '−' => "-",
        '×' | '·' | '⋅' | '∗' => "*",
        '÷' | '∕' | '⁄' => "/",
        '≤' | '≦' | '⩽' => "<=",
        '≥' | '≧' | '⩾' => ">=",
        '≠' => "#",
        '∞' => "inf",
        _ => return None,
    })
}

/// What to type instead of a symbol Aximar rejects.
fn symbol_hint(c: char) -> Option<&'static str> {
    Some(match c {
        '∏' => "for a product write `product(f(k), k, 1, n)`. For a variable named Π, use `Π` (U+03A0).",
        '∑' => "for a sum write `sum(f(k), k, 1, n)`. For a variable named Σ, use `Σ` (U+03A3).",
        '∫' | '∬' | '∭' | '∮' => "write `integrate(f, x)`, or `integrate(f, x, a, b)` for a definite integral.",
        '∂' => "write `diff(f, x)` for a partial derivative.",
        '∇' => "write `diff` for each variable, or use `grad` from the `vect` package.",
        '∛' | '∜' => "write a root as a power, for example `x^(1/3)`.",
        '±' | '∓' => "Maxima has no ± operator; write both cases, for example `[a + b, a - b]`.",
        '≈' | '≃' | '≅' => "Maxima has no approximately-equal operator. To compare with a tolerance, write `abs(a - b) < 1e-6`.",
        '≡' => "write `=` for an equation, or `is(equal(a, b))` to test equality.",
        '≔' => "`:` assigns a value and `:=` defines a function.",
        '→' | '⟶' => "Maxima has no arrow operator. For a limit write `limit(f, x, a)`.",
        '⇒' | '⟹' | '⇔' | '⟺' => "Maxima has no implication operator; use `if ... then ...` or `and`, `or` and `not`.",
        '↦' => "for an anonymous function write `lambda([x], ...)`.",
        '∈' | '∉' => "write `elementp(x, S)` for a set, or `member(x, L)` for a list.",
        '⊂' | '⊆' | '⊃' | '⊇' => "write `subsetp(A, B)`.",
        '∪' => "write `union(A, B)`.",
        '∩' => "write `intersection(A, B)`.",
        '∖' => "write `setdifference(A, B)`.",
        '∅' => "`{}` is the empty set.",
        '∀' | '∃' => "Maxima has no quantifiers in expressions.",
        '′' | '″' | '‴' => "for a derivative write `diff(f(x), x)`.",
        '°' => "Maxima works in radians: write `30*%pi/180` for 30°.",
        '¼' | '½' | '¾' | '\u{2150}'..='\u{215F}' => "write the fraction with `/`, for example `1/2`.",
        '‰' => "write `/1000`.",
        '–' | '—' | '‐' | '‑' | '‒' | '―' => "use `-` for minus.",
        '“' | '”' | '„' | '‟' | '«' | '»' => "use straight quotes `\"` for strings.",
        '‘' | '’' | '‚' | '‛' => "Maxima's quote is `'`.",
        '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' => {
            "bidirectional control characters can make code read differently from how it runs."
        }
        _ => return None,
    })
}

const GENERAL_HINT: &str =
    "Maxima doesn't use it in code. If it's meant as text, put it in a string.";

/// How an error names a single character.
fn char_subject(c: char) -> String {
    format!("`{c}` (U+{:04X})", c as u32)
}

fn translate_code(
    code: &str,
    line: &dyn Fn(&[char], usize) -> Option<usize>,
    out: &mut String,
) -> Result<(), UnsupportedCharacter> {
    let chars = normalise(code);
    let reject = |at: usize, subject: String, hint: &str| {
        let place = line(&chars, at)
            .map(|n| format!(" on line {n}"))
            .unwrap_or_default();
        UnsupportedCharacter(format!(
            "{subject}{place} can't be used in Maxima code: {hint}"
        ))
    };
    let next_non_space = |from: usize| chars[from..].iter().copied().find(|c| !c.is_whitespace());
    let token_end = |from: usize| {
        let mut end = from;
        while end < chars.len() && is_name_char(chars[end]) {
            end += 1;
        }
        end
    };

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if is_name_char(c) {
            let end = token_end(i);
            let name = translate_name(&chars[i..end], next_non_space(end))
                .map_err(|(subject, hint)| reject(i, subject, &hint))?;
            out.push_str(&name);
            i = end;
        } else if c.is_ascii() {
            out.push(c);
            i += 1;
        } else if superscript(c).is_some() {
            out.push_str("^(");
            while let Some(ascii) = chars.get(i).copied().and_then(superscript) {
                out.push(ascii);
                i += 1;
            }
            out.push(')');
        } else if subscript(c).is_some() {
            let mut index = String::new();
            while let Some(ascii) = chars.get(i).copied().and_then(subscript) {
                if ascii.is_ascii_alphabetic() && index.ends_with(|p: char| p.is_ascii_alphabetic())
                {
                    return Err(reject(
                        i,
                        char_subject(chars[i]),
                        "two subscript letters together could mean `x[i,j]` or `x[ij]`; write the index in brackets.",
                    ));
                }
                index.push(ascii);
                i += 1;
            }
            out.push('[');
            out.push_str(&index);
            out.push(']');
        } else if c == '√' {
            let (root, next) = square_root(&chars, i + 1, &next_non_space).ok_or_else(|| {
                reject(
                    i,
                    char_subject(c),
                    "write `sqrt(...)` with brackets, for example `sqrt(x^2)`.",
                )
            })?;
            out.push_str(&root);
            i = next;
        } else if let Some(op) = operator_for(c) {
            out.push_str(op);
            i += 1;
        } else if matches!(c, '∧' | '∨' | '¬') {
            let word = match c {
                '∧' => "and",
                '∨' => "or",
                _ => "not",
            };
            // Keep the word apart from the names around it
            if out.ends_with(|p: char| !p.is_whitespace() && p != '(') && !out.is_empty() {
                out.push(' ');
            }
            out.push_str(word);
            if chars.get(i + 1).is_some_and(|n| !n.is_whitespace()) {
                out.push(' ');
            }
            i += 1;
        } else if let Some(hint) = rejected_letter(c) {
            return Err(reject(i, char_subject(c), &hint));
        } else {
            let hint = symbol_hint(c).unwrap_or(GENERAL_HINT);
            return Err(reject(i, char_subject(c), hint));
        }
    }
    Ok(())
}

/// `√` followed by a bracket, a number or a name, starting at `from` (just
/// after the `√`).  Returns the Maxima text and where to carry on, or `None`
/// when the reading isn't clear.
fn square_root(
    chars: &[char],
    from: usize,
    next_non_space: &dyn Fn(usize) -> Option<char>,
) -> Option<(String, usize)> {
    let first = *chars.get(from)?;
    if first == '(' {
        // The bracket is copied as it is
        return Some(("sqrt".to_string(), from));
    }
    let mut end = from;
    let operand = if first.is_ascii_digit() {
        while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.') {
            end += 1;
        }
        chars[from..end].iter().collect::<String>()
    } else if is_name_char(first) {
        while end < chars.len() && is_name_char(chars[end]) {
            end += 1;
        }
        translate_name(&chars[from..end], next_non_space(end)).ok()?
    } else {
        return None;
    };
    // `√x²`, `√x[1]`, `√2x` and `√f(x)` read differently to different people
    let after = chars.get(end).copied();
    let unclear = after.is_some_and(|a| {
        matches!(a, '^' | '(' | '[' | '.')
            || is_name_char(a)
            || superscript(a).is_some()
            || subscript(a).is_some()
    });
    if unclear {
        return None;
    }
    Some((format!("sqrt({operand})"), end))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Translate, expecting success.
    fn tr(input: &str) -> String {
        unicode_to_maxima(input).unwrap_or_else(|e| panic!("{input:?} was rejected: {}", e.0))
    }

    /// Translate, expecting a rejection whose message contains `hint`.
    fn rejects(input: &str, hint: &str) {
        match unicode_to_maxima(input) {
            Ok(out) => panic!("{input:?} was accepted as {out:?}"),
            Err(e) => assert!(e.0.contains(hint), "{input:?}: {:?} lacks {hint:?}", e.0),
        }
    }

    // ── Kept from the first translator ──

    #[test]
    fn greek_letters() {
        assert_eq!(tr("sin(θ)"), "sin(theta)");
        assert_eq!(tr("2*π"), "2*%pi");
        assert_eq!(tr("α + β"), "alpha + beta");
    }

    #[test]
    fn operators() {
        assert_eq!(tr("3 × 4"), "3 * 4");
        assert_eq!(tr("x ≤ 5"), "x <= 5");
        assert_eq!(tr("a ≠ b"), "a # b");
    }

    #[test]
    fn infinity() {
        assert_eq!(tr("integrate(f(x), x, 0, ∞)"), "integrate(f(x), x, 0, inf)");
        assert_eq!(tr("limit(f, x, -∞)"), "limit(f, x, -inf)");
    }

    #[test]
    fn mixed() {
        assert_eq!(
            tr("integrate(sin(θ), θ, 0, 2*π)"),
            "integrate(sin(theta), theta, 0, 2*%pi)"
        );
    }

    #[test]
    fn no_unicode_passthrough() {
        let plain = "diff(sin(x), x)";
        assert_eq!(tr(plain), plain);
    }

    #[test]
    fn uppercase_greek() {
        assert_eq!(tr("Ω"), "Omega");
        assert_eq!(
            tr("Δ + Θ + Λ + Ξ + Π + Σ + Φ + Ψ"),
            "Delta + Theta + Lambda + Xi + Pi + Sigma + Phi + Psi"
        );
    }

    #[test]
    fn texput_init_escaping() {
        let init = build_texput_init();
        // Each entry should produce: texput(name, "\\latex")
        // where \\ is a Maxima string escape for a literal backslash
        assert!(init.contains(r#"texput(theta, "\\theta")"#));
        assert!(init.contains(r#"texput(hbar, "\\hbar")"#));
        // Maxima's defaults for these match the glyphs typed
        assert!(!init.contains("epsilon") && !init.contains("phi"));
        assert!(init.ends_with('$'));
    }

    #[test]
    fn string_literals_preserved() {
        assert_eq!(tr(r#"print("τ/T")"#), r#"print("τ/T")"#);
        assert_eq!(tr(r#"[xlabel, "α → ∞"]"#), r#"[xlabel, "α → ∞"]"#);
    }

    #[test]
    fn mixed_code_and_strings() {
        assert_eq!(
            tr(r#"plot2d(sin(θ), [xlabel, "θ (radians)"])"#),
            r#"plot2d(sin(theta), [xlabel, "θ (radians)"])"#
        );
    }

    #[test]
    fn multiple_strings() {
        assert_eq!(
            tr(r#"f(τ) + "τ" + g(τ) + "τ""#),
            r#"f(tau) + "τ" + g(tau) + "τ""#
        );
    }

    #[test]
    fn subscript_digits() {
        assert_eq!(tr("T₀"), "T[0]");
        assert_eq!(tr("x₁ + x₂"), "x[1] + x[2]");
        assert_eq!(tr("a₁₂"), "a[12]");
        assert_eq!(tr("ω₀"), "omega[0]");
        assert_eq!(tr("τ₁ + τ₂"), "tau[1] + tau[2]");
        assert_eq!(tr(r#"T₀ + "T₀""#), r#"T[0] + "T₀""#);
    }

    #[test]
    fn superscripts() {
        assert_eq!(tr("x²"), "x^(2)");
        assert_eq!(tr("x³ + y²"), "x^(3) + y^(2)");
        assert_eq!(tr("x²³"), "x^(23)");
        assert_eq!(tr("x⁻¹"), "x^(-1)");
        assert_eq!(tr("r⁻²"), "r^(-2)");
        assert_eq!(tr("xⁿ"), "x^(n)");
        assert_eq!(tr("2ⁿ⁺¹"), "2^(n+1)");
        assert_eq!(tr("θ²"), "theta^(2)");
        assert_eq!(tr("ω⁻¹"), "omega^(-1)");
        assert_eq!(tr(r#"x² + "x²""#), r#"x^(2) + "x²""#);
        assert_eq!(tr("x₁² + x₂²"), "x[1]^(2) + x[2]^(2)");
    }

    // ── Only code is translated ──

    #[test]
    fn strings_with_escaped_quotes_stay_as_typed() {
        let input = r#"print("say \"α\" and β");"#;
        assert_eq!(tr(input), input);
    }

    #[test]
    fn comments_stay_as_typed() {
        assert_eq!(
            tr("/* α is the angle */ α: 1;"),
            "/* α is the angle */ alpha: 1;"
        );
        assert_eq!(tr("/* ≈ /* nested ∑ */ */ x;"), "/* ≈ /* nested ∑ */ */ x;");
    }

    #[test]
    fn rejected_characters_in_strings_are_fine() {
        assert_eq!(
            tr(r#"print("≈ “quoted” 30°");"#),
            r#"print("≈ “quoted” 30°");"#
        );
    }

    // ── Greek letters as names ──

    #[test]
    fn every_lowercase_letter() {
        let pairs = [
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
        ];
        for (letter, name) in pairs {
            assert_eq!(tr(letter), name, "{letter}");
        }
    }

    #[test]
    fn letter_with_a_number() {
        assert_eq!(tr("α1 + α_2;"), "alpha1 + alpha_2;");
        assert_eq!(tr("Δ12"), "Delta12");
    }

    #[test]
    fn letter_inside_a_longer_name_stays_as_typed() {
        assert_eq!(tr("Δx: 1$ xπ + αβ;"), "Δx: 1$ xπ + αβ;");
        assert_eq!(tr("α_max"), "α_max");
        assert_eq!(tr("π2"), "π2");
    }

    #[test]
    fn gamma_function() {
        assert_eq!(tr("Γ(5);"), "gamma(5);");
        assert_eq!(tr("Γ (5);"), "gamma (5);");
        assert_eq!(tr("Γ + 1;"), "Gamma + 1;");
        assert_eq!(tr("γ(5);"), "gamma(5);");
    }

    #[test]
    fn mathematical_names_and_constants() {
        assert_eq!(tr("ℏ*ω"), "hbar*omega");
        assert_eq!(tr("ℓ"), "ell");
        assert_eq!(tr("ϒ"), "Upsilon");
        assert_eq!(tr("ⅈ^2 + ⅇ + ℯ"), "%i^2 + %e + %e");
    }

    // ── Variants ──

    #[test]
    fn variant_forms_are_their_letter() {
        let pairs = [
            ("ϐ", "beta"),
            ("ϑ", "theta"),
            ("ϕ", "phi"),
            ("ϰ", "kappa"),
            ("ϱ", "rho"),
            ("ϴ", "Theta"),
            ("ϵ", "epsilon"),
            ("ς", "sigma"),
            ("µ", "mu"),
            ("\u{2126}", "Omega"),
            ("∆", "Delta"),
        ];
        for (variant, name) in pairs {
            assert_eq!(tr(variant), name, "{variant}");
        }
        // Inside a name too
        assert_eq!(tr("xϕ"), "xφ");
    }

    #[test]
    fn mathematical_italic_is_the_plain_letter() {
        assert_eq!(tr("𝑥 + 𝛼;"), "x + alpha;");
        assert_eq!(tr("𝐴𝑏"), "Ab");
        assert_eq!(tr("ℎ"), "h");
        assert_eq!(tr("𝛤(5)"), "gamma(5)");
        assert_eq!(tr("𝜔 + 𝜑 + 𝜙"), "omega + phi + phi");
    }

    // ── Operators and symbols ──

    #[test]
    fn operator_forms() {
        assert_eq!(tr("2 × 3 − 1;"), "2 * 3 - 1;");
        assert_eq!(tr("a·b⋅c∗d"), "a*b*c*d");
        assert_eq!(tr("a÷b∕c⁄d"), "a/b/c/d");
        assert_eq!(tr("a≦b; a⩽b; a≧b; a⩾b"), "a<=b; a<=b; a>=b; a>=b");
    }

    #[test]
    fn logic() {
        assert_eq!(tr("a∧¬b;"), "a and not b;");
        assert_eq!(tr("a ∨ b"), "a or b");
        assert_eq!(tr("¬(a)"), "not (a)");
    }

    #[test]
    fn square_roots() {
        assert_eq!(
            tr("√2 + √(x+1) + √α;"),
            "sqrt(2) + sqrt(x+1) + sqrt(alpha);"
        );
        assert_eq!(tr("√2.5"), "sqrt(2.5)");
        rejects("√x²", "sqrt(");
        rejects("√2x", "sqrt(");
        rejects("√x[1]", "sqrt(");
        rejects("√ x", "sqrt(");
        rejects("√", "sqrt(");
    }

    #[test]
    fn subscript_letters() {
        assert_eq!(tr("aₙ₊₁ = x²;"), "a[n+1] = x^(2);");
        assert_eq!(tr("xᵢ + yⱼ"), "x[i] + y[j]");
        rejects("xᵢⱼ", "x[i,j]");
    }

    // ── Rejections ──

    #[test]
    fn rejection_names_the_character_and_code_point() {
        rejects("a ≈ b;", "`≈` (U+2248)");
        rejects("a ≈ b;", "abs(a - b)");
    }

    #[test]
    fn rejection_names_the_line_when_there_are_several() {
        rejects("a: 1;\nb ≈ c;", "on line 2");
        rejects("/* one\ntwo */\nx: \"a\nb\";\n≈", "on line 5");
        let single = unicode_to_maxima("a ≈ b").unwrap_err();
        assert!(!single.0.contains("line"), "{}", single.0);
    }

    #[test]
    fn lookalike_capitals_and_omicron() {
        for (greek, latin) in [
            ("Α", "A"),
            ("Β", "B"),
            ("Ε", "E"),
            ("Ζ", "Z"),
            ("Η", "H"),
            ("Ι", "I"),
            ("Κ", "K"),
            ("Μ", "M"),
            ("Ν", "N"),
            ("Ο", "O"),
            ("Ρ", "P"),
            ("Τ", "T"),
            ("Υ", "Y"),
            ("Χ", "X"),
            ("ο", "o"),
        ] {
            rejects(&format!("{greek} + 1;"), &format!("Type `{latin}`"));
        }
        rejects("Υ", "ϒ");
        rejects("xΑ", "Latin letter `A`");
    }

    #[test]
    fn rejected_letters() {
        rejects("ϖ", "variant of π");
        rejects("ⅉ", "%i");
        rejects("ℝ", "declare(x, real)");
        rejects("ℕ", "declare(n, integer)");
        rejects("𝕩", "number-set");
        rejects("𝐱", "no style");
        rejects("𝔵", "no style");
        rejects("ℒ", "no style");
        rejects("Mᵀ", "transpose(M)");
        rejects("ⅆx", "diff(f, x)");
        rejects("30º", "radians");
    }

    #[test]
    fn rejected_symbols_have_hints() {
        let cases = [
            ("∑ k", "sum("),
            ("∏ k", "product("),
            ("∫ f", "integrate("),
            ("∂f", "diff("),
            ("∇f", "grad"),
            ("∛x", "x^(1/3)"),
            ("a ± b", "both cases"),
            ("a ≡ b", "is(equal"),
            ("a ≔ 1", ":="),
            ("x → 0", "limit("),
            ("x ↦ x", "lambda("),
            ("x ∈ S", "elementp("),
            ("A ⊂ B", "subsetp("),
            ("A ∪ B", "union("),
            ("A ∩ B", "intersection("),
            ("A ∖ B", "setdifference("),
            ("∅", "{}"),
            ("∀x", "quantifiers"),
            ("f′(x)", "diff("),
            ("sin(30°)", "30*%pi/180"),
            ("½", "1/2"),
            ("5‰", "/1000"),
            ("a – b", "`-`"),
            ("a — b", "`-`"),
            ("“text”", "straight quotes"),
            ("’a", "quote is"),
            ("a\u{202E}b", "bidirectional"),
        ];
        for (input, hint) in cases {
            rejects(input, hint);
        }
    }

    #[test]
    fn other_symbols_get_the_general_hint() {
        rejects("x😀", "put it in a string");
        rejects("a … b", "put it in a string");
        rejects("x。", "put it in a string");
    }

    #[test]
    fn implicit_multiplication() {
        rejects("2π;", "write `2*π`");
        rejects("3α", "write `3*α`");
        // A digit before an ASCII name is Maxima's to report
        assert_eq!(tr("2x"), "2x");
        assert_eq!(tr("1e5 + α"), "1e5 + alpha");
    }

    // ── Pasted text ──

    #[test]
    fn spaces_and_invisible_characters() {
        assert_eq!(tr("a\u{00A0}+ b;"), "a + b;");
        assert_eq!(tr("a\u{2009}+\u{3000}b"), "a + b");
        assert_eq!(tr("a\u{200B}b\u{FEFF}\u{00AD}"), "ab");
        assert_eq!(tr("a;\u{2028}b;"), "a;\nb;");
    }

    #[test]
    fn full_width_forms_and_ligatures() {
        assert_eq!(tr("ｘ＋（１）"), "x+(1)");
        assert_eq!(tr("deﬁne(ﬂ)"), "define(fl)");
    }

    #[test]
    fn combining_accents_are_composed() {
        assert_eq!(tr("e\u{0301}"), "é");
    }

    // ── Other letters ──

    #[test]
    fn other_letters_pass_through() {
        assert_eq!(tr("größe: 2;"), "größe: 2;");
        assert_eq!(tr("жар + 中"), "жар + 中");
        assert_eq!(tr("ά"), "ά");
    }

    // ── The frontend's symbol list ──

    /// `\name` completion and the command palette offer the symbols in
    /// `src/lib/math-symbols.ts`.  Each must be one this table translates,
    /// to what the list says.
    #[test]
    fn frontend_symbol_list_matches_the_table() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/lib/math-symbols.ts");
        let source = std::fs::read_to_string(path).expect("read math-symbols.ts");
        let entry = regex::Regex::new(r#"unicode: "([^"]+)", maxima: "([^"]*)""#).unwrap();
        let mut checked = 0;
        for caps in entry.captures_iter(&source) {
            let (unicode, maxima) = (&caps[1], &caps[2]);
            checked += 1;
            // Subscripts and superscripts follow a name; their `maxima` is
            // only a guide (`^(-`)
            if maxima.starts_with('[') || maxima.starts_with('^') {
                assert!(
                    unicode_to_maxima(&format!("x{unicode}")).is_ok(),
                    "{unicode}"
                );
                continue;
            }
            // √ needs something to take the root of
            if unicode == "√" {
                assert_eq!(tr("√(x)"), format!("{maxima}(x)"));
                continue;
            }
            let got = tr(unicode);
            assert_eq!(
                got.trim(),
                maxima,
                "{unicode} becomes {got:?}, the list says {maxima:?}"
            );
        }
        assert!(checked > 50, "only {checked} entries found");
    }
}
