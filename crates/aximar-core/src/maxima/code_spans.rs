//! Split Maxima input into code and non-code pieces.
//!
//! Rewrites such as label substitution must leave string literals and
//! comments alone.  Maxima strings are delimited by `"` and use `\` to
//! escape the next character; comments are `/* ... */` and nest.  An
//! unterminated string or comment runs to the end of the input.

/// One piece of Maxima input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span<'a> {
    /// Code outside strings and comments.
    Code(&'a str),
    /// A string literal or comment, including its delimiters.
    Text(&'a str),
}

/// Split `input` into code and text pieces.  Concatenating the pieces gives
/// back `input` unchanged.
pub fn spans(input: &str) -> Vec<Span<'_>> {
    let bytes = input.as_bytes();
    let mut pieces = Vec::new();
    let mut code_start = 0;
    let mut i = 0;
    while i < bytes.len() {
        let end = if bytes[i] == b'"' {
            Some(string_end(bytes, i))
        } else if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            Some(comment_end(bytes, i))
        } else {
            None
        };
        match end {
            Some(end) => {
                if code_start < i {
                    pieces.push(Span::Code(&input[code_start..i]));
                }
                pieces.push(Span::Text(&input[i..end]));
                code_start = end;
                i = end;
            }
            None => i += 1,
        }
    }
    if code_start < bytes.len() {
        pieces.push(Span::Code(&input[code_start..]));
    }
    pieces
}

/// Byte offset just past the string literal that opens at `start`.
/// Only ASCII bytes end a piece, so the offset is always a char boundary.
fn string_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// Byte offset just past the (possibly nested) comment that opens at `start`.
fn comment_end(bytes: &[u8], start: usize) -> usize {
    let mut depth = 0;
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Span::{Code, Text};

    fn joined(pieces: &[Span]) -> String {
        pieces
            .iter()
            .map(|p| match p {
                Code(s) | Text(s) => *s,
            })
            .collect()
    }

    #[test]
    fn plain_code() {
        assert_eq!(spans("x^2 + 1;"), vec![Code("x^2 + 1;")]);
        assert_eq!(spans(""), vec![]);
    }

    #[test]
    fn string_literal() {
        assert_eq!(
            spans(r#"print("a % b", %)"#),
            vec![Code("print("), Text(r#""a % b""#), Code(", %)")]
        );
    }

    #[test]
    fn escaped_quote_stays_in_string() {
        assert_eq!(
            spans(r#"s : "a\"b%"; %"#),
            vec![Code("s : "), Text(r#""a\"b%""#), Code("; %")]
        );
    }

    #[test]
    fn comments_nest() {
        assert_eq!(
            spans("/* a /* b */ % */ %"),
            vec![Text("/* a /* b */ % */"), Code(" %")]
        );
    }

    #[test]
    fn quote_inside_comment_is_not_a_string() {
        assert_eq!(
            spans(r#"/* don't "quote */ %"#),
            vec![Text(r#"/* don't "quote */"#), Code(" %")]
        );
    }

    #[test]
    fn unterminated_pieces_run_to_the_end() {
        assert_eq!(spans(r#"f("abc"#), vec![Code("f("), Text(r#""abc"#)]);
        assert_eq!(spans("1 /* open"), vec![Code("1 "), Text("/* open")]);
        assert_eq!(spans(r#""ends with \"#), vec![Text(r#""ends with \"#)]);
    }

    #[test]
    fn pieces_join_back_to_the_input() {
        for input in [
            r#"a : "→ ∞ \" %"; /* θ /* nested */ */ b : %;"#,
            "/**/x/* */",
            r#""\\" + %"#,
        ] {
            assert_eq!(joined(&spans(input)), input);
        }
    }
}
