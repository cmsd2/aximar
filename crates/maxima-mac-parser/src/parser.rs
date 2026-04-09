use crate::lexer::{self, Token, Spanned};
use crate::types::{ParseErrorKind, Severity};

/// Intermediate types with byte-offset spans (converted to Position/Span in lib.rs)

#[derive(Debug, Clone, Default)]
pub struct RawMacFile<'src> {
    pub items: Vec<RawItem<'src>>,
    pub load_calls: Vec<RawLoadCall<'src>>,
    pub errors: Vec<RawParseError>,
}

#[derive(Debug, Clone)]
pub enum RawItem<'src> {
    FuncDef(RawFunctionDef<'src>),
    MacroDef(RawFunctionDef<'src>),
    VarAssign(RawVariableAssign<'src>),
}

#[derive(Debug, Clone)]
pub struct RawFunctionDef<'src> {
    pub name: &'src str,
    pub params: Vec<String>,
    pub span: lexer::Span,
    pub name_span: lexer::Span,
    pub body_start_offset: usize,
    pub doc_comment: Option<String>,
    pub block_locals: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RawVariableAssign<'src> {
    pub name: &'src str,
    pub span: lexer::Span,
    pub name_span: lexer::Span,
}

#[derive(Debug, Clone)]
pub struct RawLoadCall<'src> {
    pub path: &'src str,
    pub span: lexer::Span,
}

#[derive(Debug, Clone)]
pub struct RawParseError {
    pub kind: ParseErrorKind,
    pub span: lexer::Span,
    pub severity: Severity,
}


impl<'src> RawItem<'src> {
    pub fn span(&self) -> lexer::Span {
        match self {
            RawItem::FuncDef(f) | RawItem::MacroDef(f) => f.span,
            RawItem::VarAssign(v) => v.span,
        }
    }

    pub fn set_doc_comment(&mut self, comment: String) {
        match self {
            RawItem::FuncDef(f) | RawItem::MacroDef(f) => f.doc_comment = Some(comment),
            RawItem::VarAssign(_) => {}
        }
    }
}

/// Structural parser for the filtered (no-comment) token stream.
/// Extracts function defs, macro defs, variable assignments, and load calls.
struct Parser<'src, 'tok> {
    tokens: &'tok [Spanned<Token<'src>>],
    pos: usize,
    items: Vec<RawItem<'src>>,
    load_calls: Vec<RawLoadCall<'src>>,
    errors: Vec<RawParseError>,
}

impl<'src, 'tok> Parser<'src, 'tok> {
    fn new(tokens: &'tok [Spanned<Token<'src>>]) -> Self {
        Self {
            tokens,
            pos: 0,
            items: Vec::new(),
            load_calls: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn peek(&self) -> Option<&Token<'src>> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn span(&self) -> lexer::Span {
        self.tokens
            .get(self.pos)
            .map(|(_, s)| *s)
            .unwrap_or_else(|| {
                // EOF: use span at end
                self.tokens
                    .last()
                    .map(|(_, s)| (s.end..s.end).into())
                    .unwrap_or((0..0).into())
            })
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn is_terminator(&self) -> bool {
        matches!(self.peek(), Some(Token::Semicolon | Token::Dollar))
    }

    /// Skip to the next statement terminator, respecting paren/bracket nesting.
    fn skip_to_terminator(&mut self) {
        let mut paren_depth: i32 = 0;
        let mut bracket_depth: i32 = 0;

        loop {
            match self.peek() {
                None => break,
                Some(Token::LParen) => {
                    paren_depth += 1;
                    self.advance();
                }
                Some(Token::RParen) => {
                    paren_depth -= 1;
                    self.advance();
                }
                Some(Token::LBracket) => {
                    bracket_depth += 1;
                    self.advance();
                }
                Some(Token::RBracket) => {
                    bracket_depth -= 1;
                    self.advance();
                }
                Some(Token::Semicolon | Token::Dollar)
                    if paren_depth <= 0 && bracket_depth <= 0 =>
                {
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Consume a terminator if present.
    fn skip_terminator(&mut self) {
        if self.is_terminator() {
            self.advance();
        }
    }

    /// Record a skipped-statement warning covering the span from `start` to current position.
    fn record_skipped(&mut self, start: usize) {
        let end = self.pos;
        if start < end && start < self.tokens.len() {
            let span_start = self.tokens[start].1.start;
            let span_end = if end > 0 && end <= self.tokens.len() {
                self.tokens[end - 1].1.end
            } else {
                span_start
            };
            self.errors.push(RawParseError {
                kind: ParseErrorKind::SkippedStatement,
                span: (span_start..span_end).into(),
                severity: Severity::Warning,
            });
        }
    }

    /// Parse the entire file.
    fn parse_file(mut self) -> RawMacFile<'src> {
        while !self.at_end() {
            match self.peek() {
                Some(Token::Semicolon | Token::Dollar) => {
                    self.advance();
                }
                Some(Token::Ident("load")) => {
                    if let Some(lc) = self.try_parse_load_call() {
                        self.load_calls.push(lc);
                        self.skip_to_terminator();
                        self.skip_terminator();
                    } else {
                        // Still a valid function call, just not one we extract info from
                        self.skip_to_terminator();
                        self.skip_terminator();
                    }
                }
                Some(Token::Ident("define")) => {
                    if let Some(fd) = self.try_parse_define_call() {
                        self.items.push(RawItem::FuncDef(fd));
                        self.skip_to_terminator();
                        self.skip_terminator();
                    } else {
                        // Still a valid function call, just not a form we recognize
                        self.skip_to_terminator();
                        self.skip_terminator();
                    }
                }
                Some(Token::Ident(_)) => {
                    if let Some(item) = self.try_parse_func_macro_or_var() {
                        self.items.push(item);
                        self.skip_terminator();
                    } else {
                        // Valid expression statement (e.g. print(...), foo(x), bar)
                        self.skip_to_terminator();
                        self.skip_terminator();
                    }
                }
                _ => {
                    let start = self.pos;
                    self.skip_to_terminator();
                    self.record_skipped(start);
                    self.skip_terminator();
                }
            }
        }

        RawMacFile {
            items: self.items,
            load_calls: self.load_calls,
            errors: self.errors,
        }
    }

    /// Try to parse `load("path")`.
    fn try_parse_load_call(&mut self) -> Option<RawLoadCall<'src>> {
        let save = self.pos;
        let start_span = self.span();

        self.advance(); // consume "load"

        if !matches!(self.peek(), Some(Token::LParen)) {
            self.pos = save;
            return None;
        }
        self.advance();

        let path = match self.peek() {
            Some(Token::Str(s)) => strip_quotes(s),
            _ => {
                self.pos = save;
                return None;
            }
        };
        self.advance();

        if !matches!(self.peek(), Some(Token::RParen)) {
            self.pos = save;
            return None;
        }
        let end = self.span().end;
        self.advance();

        Some(RawLoadCall {
            path,
            span: (start_span.start..end).into(),
        })
    }

    /// Try to parse `define(f(x, y), body)`.
    fn try_parse_define_call(&mut self) -> Option<RawFunctionDef<'src>> {
        let save = self.pos;
        let start_span = self.span();

        self.advance(); // consume "define"

        if !matches!(self.peek(), Some(Token::LParen)) {
            self.pos = save;
            return None;
        }
        self.advance(); // consume outer (

        let (name, name_span) = match self.peek().cloned() {
            Some(Token::Ident(s)) => {
                let span = self.span();
                self.advance();
                (s, span)
            }
            _ => {
                self.pos = save;
                return None;
            }
        };

        if !matches!(self.peek(), Some(Token::LParen)) {
            self.pos = save;
            return None;
        }
        self.advance(); // consume params (

        let params = self.parse_param_list();

        if !matches!(self.peek(), Some(Token::RParen)) {
            self.pos = save;
            return None;
        }
        self.advance(); // consume params )

        if !matches!(self.peek(), Some(Token::Comma)) {
            self.pos = save;
            return None;
        }
        let body_start_offset = self.span().end;
        self.advance(); // consume ,

        let block_locals = self.try_extract_block_locals();

        // Skip body until closing ) (balanced)
        let mut depth: i32 = 1;
        while depth > 0 && !self.at_end() {
            match self.peek() {
                Some(Token::LParen) => {
                    depth += 1;
                    self.advance();
                }
                Some(Token::RParen) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    self.advance();
                }
                _ => self.advance(),
            }
        }

        let end = self.span().end;
        if matches!(self.peek(), Some(Token::RParen)) {
            self.advance();
        }

        Some(RawFunctionDef {
            name,
            params,
            span: (start_span.start..end).into(),
            name_span,
            body_start_offset,
            doc_comment: None,
            block_locals,
        })
    }

    /// Try to parse `IDENT(params) := body`, `IDENT(params) ::= body`, or `IDENT : body`.
    fn try_parse_func_macro_or_var(&mut self) -> Option<RawItem<'src>> {
        let save = self.pos;
        let start_span = self.span();

        let (name, name_span) = match self.peek().cloned() {
            Some(Token::Ident(s)) => {
                let span = self.span();
                self.advance();
                (s, span)
            }
            _ => return None,
        };

        match self.peek() {
            // IDENT ( params ) := body  or  IDENT ( params ) ::= body
            Some(Token::LParen) => {
                self.advance(); // consume (
                let params = self.parse_param_list();

                if !matches!(self.peek(), Some(Token::RParen)) {
                    self.pos = save;
                    return None;
                }
                self.advance(); // consume )

                let is_macro = match self.peek() {
                    Some(Token::ColonEqual) => false,
                    Some(Token::DoubleColonEqual) => true,
                    _ => {
                        self.pos = save;
                        return None;
                    }
                };
                self.advance(); // consume := or ::=

                // body_start_offset is the byte offset of the first body token
                let body_start_offset = self.span().start;

                let block_locals = self.try_extract_block_locals();
                self.skip_to_terminator();

                let span = self.span_including_terminator(start_span);
                let func_def = RawFunctionDef {
                    name,
                    params,
                    span,
                    name_span,
                    body_start_offset,
                    doc_comment: None,
                    block_locals,
                };

                if is_macro {
                    Some(RawItem::MacroDef(func_def))
                } else {
                    Some(RawItem::FuncDef(func_def))
                }
            }

            // IDENT : body (variable assignment)
            Some(Token::Colon) => {
                self.advance(); // consume :
                self.skip_to_terminator();

                let span = self.span_including_terminator(start_span);
                Some(RawItem::VarAssign(RawVariableAssign {
                    name,
                    span,
                    name_span,
                }))
            }

            _ => {
                self.pos = save;
                None
            }
        }
    }

    /// Compute span from start to current position, including the terminator if present.
    fn span_including_terminator(&self, start_span: lexer::Span) -> lexer::Span {
        let end = if self.is_terminator() {
            self.span().end
        } else if self.pos > 0 {
            self.tokens[self.pos - 1].1.end
        } else {
            start_span.end
        };
        (start_span.start..end).into()
    }

    /// Try to extract block local variable names from the current position.
    ///
    /// Looks for `block([var1, var2, ...], ...)` and returns the variable
    /// names. Handles initialized locals like `block([a: 0, b], ...)` by
    /// extracting just the names. This is a lookahead-only operation — the
    /// parser position is restored afterwards.
    fn try_extract_block_locals(&mut self) -> Vec<String> {
        let save = self.pos;

        if !matches!(self.peek(), Some(Token::Ident("block"))) {
            return Vec::new();
        }
        self.advance();

        if !matches!(self.peek(), Some(Token::LParen)) {
            self.pos = save;
            return Vec::new();
        }
        self.advance();

        if !matches!(self.peek(), Some(Token::LBracket)) {
            self.pos = save;
            return Vec::new();
        }
        self.advance();

        let mut locals = Vec::new();
        loop {
            match self.peek() {
                Some(Token::RBracket) => break,
                None => {
                    self.pos = save;
                    return Vec::new();
                }
                Some(Token::Ident(name)) => {
                    locals.push(name.to_string());
                    self.advance();
                    // Skip initializer if present (e.g. `a: 0`)
                    if matches!(self.peek(), Some(Token::Colon)) {
                        self.advance(); // skip :
                        let mut depth = 0i32;
                        loop {
                            match self.peek() {
                                None => break,
                                Some(Token::LParen | Token::LBracket) => {
                                    depth += 1;
                                    self.advance();
                                }
                                Some(Token::RParen | Token::RBracket) if depth > 0 => {
                                    depth -= 1;
                                    self.advance();
                                }
                                Some(Token::RBracket) if depth == 0 => break,
                                Some(Token::Comma) if depth == 0 => break,
                                _ => self.advance(),
                            }
                        }
                    }
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    }
                }
                _ => {
                    // Not a simple locals list
                    self.pos = save;
                    return Vec::new();
                }
            }
        }

        self.pos = save;
        locals
    }

    /// Parse comma-separated parameter list. Handles `[args]` variadic form.
    fn parse_param_list(&mut self) -> Vec<String> {
        let mut params = Vec::new();

        if matches!(self.peek(), Some(Token::RParen) | None) {
            return params;
        }

        loop {
            match self.peek() {
                Some(Token::LBracket) => {
                    self.advance();
                    if let Some(Token::Ident(s)) = self.peek() {
                        params.push(format!("[{}]", s));
                        self.advance();
                    }
                    if matches!(self.peek(), Some(Token::RBracket)) {
                        self.advance();
                    }
                }
                Some(Token::Ident(s)) => {
                    params.push(s.to_string());
                    self.advance();
                }
                _ => break,
            }

            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            } else {
                break;
            }
        }

        params
    }
}

pub fn parse<'src>(tokens: &[Spanned<Token<'src>>]) -> RawMacFile<'src> {
    Parser::new(tokens).parse_file()
}

fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;
    use crate::types::*;

    #[test]
    fn simple_function_def() {
        let file = parse("f(x) := x^2$");
        assert_eq!(file.items.len(), 1);
        assert_eq!(file.items[0].name(), "f");
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.params, vec!["x"]);
        }
    }

    #[test]
    fn function_with_block() {
        let source = "f(x) := block(\n  [a],\n  a: x + 1,\n  a^2\n)$";
        let file = parse(source);
        assert_eq!(file.items.len(), 1);
        assert_eq!(file.items[0].name(), "f");
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.params, vec!["x"]);
            assert_eq!(f.block_locals, vec!["a"]);
        }
    }

    #[test]
    fn block_locals_multiple() {
        let source = "f(x) := block([a, b, c], a: x+1, b: a*2, c: a+b, c)$";
        let file = parse(source);
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.block_locals, vec!["a", "b", "c"]);
        }
    }

    #[test]
    fn block_locals_with_initializers() {
        let source = "f(x) := block([a: 0, b: [1,2,3], c], c: a+x, c)$";
        let file = parse(source);
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.block_locals, vec!["a", "b", "c"]);
        }
    }

    #[test]
    fn block_locals_empty_for_no_block() {
        let file = parse("f(x) := x^2$");
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert!(f.block_locals.is_empty());
        }
    }

    #[test]
    fn block_locals_define_form() {
        let source = "define(f(x), block([a], a: x+1, a^2))$";
        let file = parse(source);
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.block_locals, vec!["a"]);
        }
    }

    #[test]
    fn macro_def() {
        let file = parse("m(x) ::= buildq([x], x^2)$");
        assert_eq!(file.items.len(), 1);
        assert!(matches!(&file.items[0], MacItem::MacroDef(_)));
        if let MacItem::MacroDef(m) = &file.items[0] {
            assert_eq!(m.name, "m");
            assert_eq!(m.params, vec!["x"]);
        }
    }

    #[test]
    fn variable_assignment() {
        let file = parse("my_var : 42$");
        assert_eq!(file.items.len(), 1);
        assert!(matches!(&file.items[0], MacItem::VariableAssign(_)));
        assert_eq!(file.items[0].name(), "my_var");
    }

    #[test]
    fn load_call() {
        let file = parse(r#"load("stringproc")$"#);
        assert_eq!(file.load_calls.len(), 1);
        assert_eq!(file.load_calls[0].path, "stringproc");
    }

    #[test]
    fn multiple_items() {
        let source = r#"
load("foo")$
x : 10$
f(a, b) := a + b;
g(x) := x^2$
"#;
        let file = parse(source);
        assert_eq!(file.load_calls.len(), 1);
        assert_eq!(file.load_calls[0].path, "foo");
        assert_eq!(file.items.len(), 3);
        assert_eq!(file.items[0].name(), "x");
        assert_eq!(file.items[1].name(), "f");
        assert_eq!(file.items[2].name(), "g");
    }

    #[test]
    fn doc_comment() {
        let source = "/* Compute the square */\nf(x) := x^2$";
        let file = parse(source);
        assert_eq!(file.items.len(), 1);
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.doc_comment.as_deref(), Some("Compute the square"));
        }
    }

    #[test]
    fn variadic_params() {
        let file = parse("f(x, [args]) := x$");
        assert_eq!(file.items.len(), 1);
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.params, vec!["x", "[args]"]);
        }
    }

    #[test]
    fn define_form() {
        let file = parse("define(f(x), x^2)$");
        assert_eq!(file.items.len(), 1);
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.name, "f");
            assert_eq!(f.params, vec!["x"]);
        }
    }

    #[test]
    fn nested_block_not_top_level_assign() {
        let source = "f(x) := block([a], a: x+1, a)$";
        let file = parse(source);
        assert_eq!(file.items.len(), 1);
        assert!(matches!(&file.items[0], MacItem::FunctionDef(_)));
    }

    #[test]
    fn multiple_functions_semicolons() {
        let source = "f(x) := x; g(y) := y;";
        let file = parse(source);
        assert_eq!(file.items.len(), 2);
        assert_eq!(file.items[0].name(), "f");
        assert_eq!(file.items[1].name(), "g");
    }

    #[test]
    fn body_start_line() {
        let source = "f(x) :=\n  x^2$";
        let file = parse(source);
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.body_start_line, 1);
        }
    }

    #[test]
    fn empty_input() {
        let file = parse("");
        assert!(file.items.is_empty());
        assert!(file.load_calls.is_empty());
    }

    #[test]
    fn only_comments() {
        let file = parse("/* just a comment */");
        assert!(file.items.is_empty());
    }

    #[test]
    fn function_no_params() {
        let file = parse("f() := 42$");
        if let MacItem::FunctionDef(f) = &file.items[0] {
            assert_eq!(f.name, "f");
            assert!(f.params.is_empty());
        }
    }

    #[test]
    fn lisp_escape() {
        let source = ":lisp (format t \"test\")\nf(x) := x$";
        let file = parse(source);
        assert_eq!(file.items.len(), 1);
        assert_eq!(file.items[0].name(), "f");
    }

    #[test]
    fn top_level_function_call_no_warning() {
        let source = r#"
f(x) := x^2$
print("result =", f(3))$
"#;
        let file = parse(source);
        assert_eq!(file.items.len(), 1);
        assert_eq!(file.items[0].name(), "f");
        assert!(
            file.errors.is_empty(),
            "print() call should not produce a warning, got: {:?}",
            file.errors
        );
    }

    #[test]
    fn multiple_top_level_calls_no_warning() {
        let source = r#"
f(x) := x + 1$
g(x) := x * 2$
print("f(3) =", f(3))$
print("g(4) =", g(4))$
display(f(5))$
"#;
        let file = parse(source);
        assert_eq!(file.items.len(), 2);
        assert!(
            file.errors.is_empty(),
            "top-level calls should not produce warnings, got: {:?}",
            file.errors
        );
    }

    #[test]
    fn fault_tolerant_recovers_after_bad_statement() {
        let source = "??? bad stuff $\nf(x) := x^2$";
        let file = parse(source);
        let names: Vec<&str> = file.items.iter().map(|i| i.name()).collect();
        assert!(names.contains(&"f"));
    }

    #[test]
    fn mixed_definitions() {
        let source = r#"
load("draw")$

/* style config */
default_color : "blue"$

/* Main draw function */
my_draw(expr) := block(
  [opts],
  opts: [color=default_color],
  apply(draw2d, append(opts, [expr]))
)$

/* Macro version */
my_draw_macro(expr) ::= buildq([expr], draw2d(expr))$
"#;
        let file = parse(source);
        assert_eq!(file.load_calls.len(), 1);
        assert_eq!(file.load_calls[0].path, "draw");

        assert_eq!(file.items.len(), 3);
        assert!(matches!(&file.items[0], MacItem::VariableAssign(_)));
        assert!(matches!(&file.items[1], MacItem::FunctionDef(_)));
        assert!(matches!(&file.items[2], MacItem::MacroDef(_)));
    }
}
