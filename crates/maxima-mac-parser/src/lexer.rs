use chumsky::prelude::*;

/// Token type for the Maxima `.mac` lexer.
///
/// Borrows string content from the source for zero-copy lexing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token<'src> {
    Ident(&'src str),
    Number(&'src str),
    Str(&'src str),     // includes surrounding quotes
    Comment(&'src str), // includes /* */
    LParen,
    RParen,
    LBracket,
    RBracket,
    ColonEqual,       // :=
    DoubleColonEqual, // ::=
    Colon,            // bare :
    Semicolon,
    Dollar,
    Comma,
    Other(&'src str),
}

impl<'src> std::fmt::Display for Token<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "{}", s),
            Token::Number(s) => write!(f, "{}", s),
            Token::Str(s) => write!(f, "{}", s),
            Token::Comment(s) => write!(f, "{}", s),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::ColonEqual => write!(f, ":="),
            Token::DoubleColonEqual => write!(f, "::="),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::Dollar => write!(f, "$"),
            Token::Comma => write!(f, ","),
            Token::Other(s) => write!(f, "{}", s),
        }
    }
}

pub type Span = SimpleSpan;
pub type Spanned<T> = (T, Span);

/// Build the chumsky lexer for Maxima `.mac` files.
///
/// Returns a parser that produces a flat list of spanned tokens from a source string.
/// The spans are byte offsets into the source.
pub fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char>>> {
    // Nested block comment: /* ... /* ... */ ... */
    // Use just("/*") as guard, then custom() for depth-tracking body
    let comment_body =
        custom::<_, &'src str, (), extra::Err<Rich<'src, char>>>(|inp| {
            let mut depth: u32 = 1;
            while depth > 0 {
                match inp.peek() {
                    None => break, // unterminated comment
                    Some('/') => {
                        inp.skip();
                        if inp.peek() == Some('*') {
                            inp.skip();
                            depth += 1;
                        }
                    }
                    Some('*') => {
                        inp.skip();
                        if inp.peek() == Some('/') {
                            inp.skip();
                            depth -= 1;
                        }
                    }
                    _ => {
                        inp.skip();
                    }
                }
            }
            Ok(())
        });

    let nested_comment = just("/*")
        .then(comment_body)
        .to_slice()
        .map(Token::Comment);

    // String literal: "..." with \" escapes
    let string = just('"')
        .then(
            none_of("\\\"")
                .ignored()
                .or(just('\\').then(any()).ignored())
                .repeated(),
        )
        .then(just('"').or_not())
        .to_slice()
        .map(Token::Str);

    // Maxima identifier: starts with letter/_%/?, continues with letter/digit/_%
    let ident = any()
        .filter(|c: &char| c.is_alphabetic() || *c == '_' || *c == '%' || *c == '?')
        .then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || *c == '_' || *c == '%')
                .repeated(),
        )
        .to_slice()
        .map(Token::Ident);

    // Number: digits with optional decimal and exponent
    let number = text::digits(10)
        .then(just('.').then(text::digits(10)).or_not())
        .then(
            one_of("eE")
                .then(one_of("+-").or_not())
                .then(text::digits(10))
                .or_not(),
        )
        .to_slice()
        .map(Token::Number);

    // Colon operators: ::= , := , :
    // Also handle :lisp escape before trying colons.
    // :lisp escape — skip the entire line, produce no token
    let lisp_escape = just(":lisp")
        .then(any().filter(|c: &char| *c != '\n').repeated())
        .then(just('\n').or_not())
        .to_slice()
        .ignored();

    // Order matters: try longest match first
    let colon_ops = choice((
        just("::=").to(Token::DoubleColonEqual),
        just(":=").to(Token::ColonEqual),
        just(':').to(Token::Colon),
    ));

    // Single-character tokens
    let single = choice((
        just('(').to(Token::LParen),
        just(')').to(Token::RParen),
        just('[').to(Token::LBracket),
        just(']').to(Token::RBracket),
        just(';').to(Token::Semicolon),
        just('$').to(Token::Dollar),
        just(',').to(Token::Comma),
    ));

    // Any other single non-whitespace character (operators, etc.)
    let other = any()
        .filter(|c: &char| !c.is_ascii_whitespace())
        .to_slice()
        .map(Token::Other);

    // A single real token
    // Order: lisp_escape before colon_ops (since :lisp starts with :)
    // Order: comment before other (since /* starts with /)
    // Order: string before other (since " is a single char)
    // Order: colon_ops before single (since : isn't in single)
    // Order: number before ident (no overlap since idents don't start with digits)
    let element = choice((
        lisp_escape.to(None),
        nested_comment.map(Some),
        string.map(Some),
        colon_ops.map(Some),
        single.map(Some),
        number.map(Some),
        ident.map(Some),
        other.map(Some),
    ))
    .map_with(|tok, e| tok.map(|t| (t, e.span())));

    // Whitespace padding between tokens
    let ws = any()
        .filter(|c: &char| c.is_ascii_whitespace())
        .repeated();

    ws.ignore_then(
        element
            .then_ignore(ws)
            .repeated()
            .collect::<Vec<_>>(),
    )
    .map(|v| v.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(source: &str) -> Vec<Spanned<Token<'_>>> {
        lexer().parse(source).into_output().unwrap_or_default()
    }

    fn tok_kinds(source: &str) -> Vec<&str> {
        tokenize(source)
            .into_iter()
            .map(|(tok, _)| match tok {
                Token::Ident(_) => "Ident",
                Token::Number(_) => "Number",
                Token::Str(_) => "Str",
                Token::Comment(_) => "Comment",
                Token::LParen => "LParen",
                Token::RParen => "RParen",
                Token::LBracket => "LBracket",
                Token::RBracket => "RBracket",
                Token::ColonEqual => "ColonEqual",
                Token::DoubleColonEqual => "DoubleColonEqual",
                Token::Colon => "Colon",
                Token::Semicolon => "Semicolon",
                Token::Dollar => "Dollar",
                Token::Comma => "Comma",
                Token::Other(_) => "Other",
            })
            .collect()
    }

    #[test]
    fn simple_function_def() {
        let kinds = tok_kinds("f(x) := x^2$");
        assert_eq!(
            kinds,
            vec![
                "Ident",      // f
                "LParen",     // (
                "Ident",      // x
                "RParen",     // )
                "ColonEqual", // :=
                "Ident",      // x
                "Other",      // ^
                "Number",     // 2
                "Dollar",     // $
            ]
        );
    }

    #[test]
    fn colon_variants() {
        let kinds = tok_kinds(": := ::=");
        assert_eq!(kinds, vec!["Colon", "ColonEqual", "DoubleColonEqual"]);
    }

    #[test]
    fn nested_comment() {
        let tokens = tokenize("/* outer /* inner */ still outer */ x");
        assert_eq!(tokens.len(), 2); // Comment, Ident
        assert!(matches!(tokens[0].0, Token::Comment(s) if s == "/* outer /* inner */ still outer */"));
        assert!(matches!(tokens[1].0, Token::Ident("x")));
    }

    #[test]
    fn string_with_escapes() {
        let tokens = tokenize(r#""hello \"world\"""#);
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].0, Token::Str(_)));
        if let Token::Str(s) = tokens[0].0 {
            assert_eq!(s, r#""hello \"world\"""#);
        }
    }

    #[test]
    fn load_call() {
        let kinds = tok_kinds(r#"load("stringproc")$"#);
        assert_eq!(
            kinds,
            vec!["Ident", "LParen", "Str", "RParen", "Dollar"]
        );
    }

    #[test]
    fn position_tracking() {
        let tokens = tokenize("ab\ncd");
        // 'ab' at bytes 0..2
        assert_eq!(tokens[0].1.start, 0);
        assert_eq!(tokens[0].1.end, 2);
        // 'cd' at bytes 3..5
        assert_eq!(tokens[1].1.start, 3);
        assert_eq!(tokens[1].1.end, 5);
    }

    #[test]
    fn lisp_escape_skipped() {
        let tokens = tokenize(":lisp (format t \"hello\")\nx: 5$");
        // :lisp line should be skipped entirely
        assert!(matches!(tokens[0].0, Token::Ident("x")));
    }

    #[test]
    fn multiline_comment_spans() {
        let tokens = tokenize("/* line1\nline2 */\nx");
        assert!(matches!(tokens[0].0, Token::Comment(_)));
        assert_eq!(tokens[0].1.start, 0);
        assert_eq!(tokens[0].1.end, 17); // "/* line1\nline2 */" is 17 bytes
        assert!(matches!(tokens[1].0, Token::Ident("x")));
        assert_eq!(tokens[1].1.start, 18);
    }

    #[test]
    fn number_with_exponent() {
        let tokens = tokenize("1.5e-3");
        assert!(matches!(tokens[0].0, Token::Number("1.5e-3")));
    }

    #[test]
    fn percent_ident() {
        let tokens = tokenize("%pi %e");
        assert!(matches!(tokens[0].0, Token::Ident("%pi")));
        assert!(matches!(tokens[1].0, Token::Ident("%e")));
    }

    #[test]
    fn unterminated_comment() {
        let tokens = tokenize("/* not closed");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].0, Token::Comment(_)));
    }

    #[test]
    fn unterminated_string() {
        let tokens = tokenize("\"not closed");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].0, Token::Str(_)));
    }

    #[test]
    fn macro_def_tokens() {
        let kinds = tok_kinds("m(x) ::= buildq([x], x^2)$");
        assert_eq!(
            kinds,
            vec![
                "Ident",            // m
                "LParen",           // (
                "Ident",            // x
                "RParen",           // )
                "DoubleColonEqual", // ::=
                "Ident",            // buildq
                "LParen",           // (
                "LBracket",         // [
                "Ident",            // x
                "RBracket",         // ]
                "Comma",            // ,
                "Ident",            // x
                "Other",            // ^
                "Number",           // 2
                "RParen",           // )
                "Dollar",           // $
            ]
        );
    }

    #[test]
    fn variable_assignment_tokens() {
        let kinds = tok_kinds("my_var : 42$");
        assert_eq!(kinds, vec!["Ident", "Colon", "Number", "Dollar"]);
    }
}
