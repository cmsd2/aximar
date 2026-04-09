pub mod lexer;
mod parser;
pub mod types;

pub use types::*;

use chumsky::prelude::*;

/// Parse a `.mac` file source string, extracting function definitions,
/// variable assignments, load() calls, and structure.
///
/// This is a fault-tolerant parser: it always returns partial results
/// even when the input contains syntax errors.
pub fn parse(source: &str) -> MacFile {
    // Phase 1: Lex
    let (tokens, lex_errors) = lexer::lexer().parse(source).into_output_errors();
    let all_tokens = tokens.unwrap_or_default();

    // Separate comments out for the parser (keep full list for doc comment attachment)
    let filtered: Vec<_> = all_tokens
        .iter()
        .filter(|(t, _)| !matches!(t, lexer::Token::Comment(_)))
        .cloned()
        .collect();

    // Phase 2: Parse structure
    let mut raw = parser::parse(&filtered);

    // Phase 2b: Convert lexer errors into parser-level errors
    for lex_err in lex_errors {
        raw.errors.push(classify_lex_error(&lex_err));
    }

    // Phase 3: Attach doc comments
    attach_doc_comments(&mut raw, &all_tokens);

    // Phase 4: Convert byte-offset spans to UTF-16 line:character spans
    let converter = SpanConverter::new(source);
    converter.convert(raw)
}

/// Replace top-level statement terminators (`$` and `;`) with commas.
///
/// Uses the lexer to correctly handle comments, strings, and nesting.
/// Only terminators at paren/bracket depth 0 are replaced. This is
/// useful for embedding multiple Maxima statements inside a `block()`
/// wrapper where commas separate statements.
pub fn replace_terminators(source: &str) -> String {
    let (tokens, _errors) = lexer::lexer().parse(source).into_output_errors();
    let tokens = tokens.unwrap_or_default();

    let mut result = source.to_string();
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;

    // Collect replacement positions (process in reverse to preserve offsets).
    let mut replacements = Vec::new();

    for (token, span) in &tokens {
        match token {
            lexer::Token::LParen => paren_depth += 1,
            lexer::Token::RParen => paren_depth -= 1,
            lexer::Token::LBracket => bracket_depth += 1,
            lexer::Token::RBracket => bracket_depth -= 1,
            lexer::Token::Dollar | lexer::Token::Semicolon
                if paren_depth <= 0 && bracket_depth <= 0 =>
            {
                replacements.push(span.start..span.end);
            }
            _ => {}
        }
    }

    for range in replacements.into_iter().rev() {
        result.replace_range(range, ",");
    }

    result
}

/// Classify a chumsky lexer error into a domain-specific `RawParseError`.
fn classify_lex_error(err: &Rich<'_, char>) -> parser::RawParseError {
    let span = *err.span();
    let msg = format!("{}", err);

    // Detect unterminated string/comment from chumsky's error message
    let kind = if msg.contains("unterminated") && msg.contains("string") {
        ParseErrorKind::UnterminatedString
    } else if msg.contains("unterminated") && msg.contains("comment") {
        ParseErrorKind::UnterminatedComment
    } else if let Some(found) = err.found() {
        ParseErrorKind::UnexpectedChar(*found)
    } else {
        ParseErrorKind::Other(msg)
    };

    parser::RawParseError {
        kind,
        span,
        severity: Severity::Error,
    }
}

/// Attach `/* ... */` doc comments to the immediately following item.
fn attach_doc_comments<'src>(
    raw: &mut parser::RawMacFile<'src>,
    all_tokens: &[lexer::Spanned<lexer::Token<'src>>],
) {
    for item in &mut raw.items {
        let item_start = item.span().start;
        if let Some(comment_text) = find_preceding_comment(all_tokens, item_start) {
            item.set_doc_comment(comment_text);
        }
    }
}

/// Find the `/* ... */` comment token immediately before a given byte offset.
fn find_preceding_comment<'src>(
    all_tokens: &[lexer::Spanned<lexer::Token<'src>>],
    item_start: usize,
) -> Option<String> {
    // Find the last token whose span starts before the item
    let idx = all_tokens.partition_point(|(_, s)| s.start < item_start);
    if idx == 0 {
        return None;
    }
    match &all_tokens[idx - 1].0 {
        lexer::Token::Comment(text) => {
            // Strip /* and */ delimiters
            if text.len() >= 4 {
                let inner = &text[2..text.len() - 2];
                // Strip leading * on each line (for boxed comments like /* * text * */)
                let trimmed: String = inner
                    .lines()
                    .map(|line| line.trim().trim_start_matches('*').trim())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
            None
        }
        _ => None,
    }
}

/// Converts byte-offset `SimpleSpan` to UTF-16 `Position`/`Span` per LSP convention.
struct SpanConverter {
    /// Byte offset of each line start.
    line_starts: Vec<usize>,
    source: String,
}

impl SpanConverter {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (i, ch) in source.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            line_starts,
            source: source.to_string(),
        }
    }

    fn byte_offset_to_position(&self, offset: usize) -> Position {
        let offset = offset.min(self.source.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(next) => next.saturating_sub(1),
        };
        let line_start = self.line_starts[line];
        let line_slice = &self.source[line_start..offset];
        let character: u32 = line_slice
            .chars()
            .map(|ch| if (ch as u32) > 0xFFFF { 2u32 } else { 1u32 })
            .sum();
        Position {
            line: line as u32,
            character,
        }
    }

    fn convert_span(&self, span: SimpleSpan) -> Span {
        Span {
            start: self.byte_offset_to_position(span.start),
            end: self.byte_offset_to_position(span.end),
        }
    }

    fn convert(&self, raw: parser::RawMacFile) -> MacFile {
        let items = raw
            .items
            .into_iter()
            .map(|item| self.convert_item(item))
            .collect();
        let load_calls = raw
            .load_calls
            .into_iter()
            .map(|lc| self.convert_load_call(lc))
            .collect();
        let errors = raw
            .errors
            .into_iter()
            .map(|e| self.convert_error(e))
            .collect();
        MacFile {
            items,
            load_calls,
            errors,
        }
    }

    fn convert_item(&self, item: parser::RawItem) -> MacItem {
        match item {
            parser::RawItem::FuncDef(f) => MacItem::FunctionDef(self.convert_func_def(f)),
            parser::RawItem::MacroDef(f) => MacItem::MacroDef(self.convert_func_def(f)),
            parser::RawItem::VarAssign(v) => MacItem::VariableAssign(self.convert_var_assign(v)),
        }
    }

    fn convert_func_def(&self, f: parser::RawFunctionDef) -> FunctionDef {
        FunctionDef {
            name: f.name.to_string(),
            params: f.params,
            span: self.convert_span(f.span),
            name_span: self.convert_span(f.name_span),
            body_start_line: self.byte_offset_to_position(f.body_start_offset).line,
            doc_comment: f.doc_comment,
            block_locals: f.block_locals,
        }
    }

    fn convert_var_assign(&self, v: parser::RawVariableAssign) -> VariableAssign {
        VariableAssign {
            name: v.name.to_string(),
            span: self.convert_span(v.span),
            name_span: self.convert_span(v.name_span),
        }
    }

    fn convert_load_call(&self, lc: parser::RawLoadCall) -> LoadCall {
        LoadCall {
            path: lc.path.to_string(),
            span: self.convert_span(lc.span),
        }
    }

    fn convert_error(&self, e: parser::RawParseError) -> ParseError {
        ParseError {
            kind: e.kind,
            span: self.convert_span(e.span),
            severity: e.severity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_converter_basic() {
        let conv = SpanConverter::new("ab\ncd\nef");
        assert_eq!(conv.byte_offset_to_position(0), Position { line: 0, character: 0 });
        assert_eq!(conv.byte_offset_to_position(1), Position { line: 0, character: 1 });
        assert_eq!(conv.byte_offset_to_position(2), Position { line: 0, character: 2 });
        assert_eq!(conv.byte_offset_to_position(3), Position { line: 1, character: 0 });
        assert_eq!(conv.byte_offset_to_position(5), Position { line: 1, character: 2 });
        assert_eq!(conv.byte_offset_to_position(6), Position { line: 2, character: 0 });
    }

    #[test]
    fn empty_parse() {
        let file = parse("");
        assert!(file.items.is_empty());
        assert!(file.load_calls.is_empty());
        assert!(file.errors.is_empty());
    }

    #[test]
    fn parse_ax_plotting_mac() {
        let source = include_str!("../../aximar-core/src/maxima/ax_plotting.mac");
        let file = parse(source);

        let func_names: Vec<&str> = file
            .items
            .iter()
            .filter_map(|item| match item {
                MacItem::FunctionDef(f) => Some(f.name.as_str()),
                MacItem::MacroDef(f) => Some(f.name.as_str()),
                _ => None,
            })
            .collect();
        let var_names: Vec<&str> = file
            .items
            .iter()
            .filter_map(|item| match item {
                MacItem::VariableAssign(v) => Some(v.name.as_str()),
                _ => None,
            })
            .collect();

        // Verify key function definitions are found
        assert!(
            func_names.contains(&"ax__float_to_json"),
            "should find ax__float_to_json, got: {:?}",
            func_names
        );
        assert!(
            func_names.contains(&"ax_draw2d"),
            "should find ax_draw2d, got: {:?}",
            func_names
        );
        assert!(
            func_names.contains(&"ax_polar"),
            "should find ax_polar, got: {:?}",
            func_names
        );

        // Verify variable assignments are found
        assert!(
            var_names.contains(&"ax__layout_option_names"),
            "should find ax__layout_option_names, got: {:?}",
            var_names
        );
        assert!(
            var_names.contains(&"ax__trace_option_names"),
            "should find ax__trace_option_names, got: {:?}",
            var_names
        );

        // Verify load calls
        let load_paths: Vec<&str> = file.load_calls.iter().map(|lc| lc.path.as_str()).collect();
        assert!(
            load_paths.contains(&"stringproc"),
            "should find load(\"stringproc\"), got: {:?}",
            load_paths
        );

        // Verify variadic params: ax_draw2d([ax__args])
        let draw2d = file
            .items
            .iter()
            .find_map(|item| match item {
                MacItem::FunctionDef(f) if f.name == "ax_draw2d" => Some(f),
                _ => None,
            })
            .expect("ax_draw2d should exist");
        assert!(
            draw2d.params.iter().any(|p| p.contains("ax__args")),
            "ax_draw2d should have variadic param [ax__args], got: {:?}",
            draw2d.params
        );

        // Verify doc comments are attached
        let float_fn = file
            .items
            .iter()
            .find_map(|item| match item {
                MacItem::FunctionDef(f) if f.name == "ax__float_to_json" => Some(f),
                _ => None,
            })
            .expect("ax__float_to_json should exist");
        assert!(
            float_fn.doc_comment.is_some(),
            "ax__float_to_json should have a doc comment"
        );

        // Reasonable total count — should find many definitions
        assert!(
            file.items.len() >= 20,
            "should find at least 20 items, found {}",
            file.items.len()
        );

        // Well-formed file should have no errors
        assert!(
            file.errors.is_empty(),
            "well-formed file should have no errors, got: {:?}",
            file.errors
        );
    }

    #[test]
    fn error_skipped_statement_is_warning() {
        // "!@#" is not a valid statement — should be skipped with a warning
        let file = parse("f(x) := x$ !@# $ g(y) := y$");
        // f and g should be found
        assert_eq!(file.items.len(), 2);
        // The "!@#" region should produce a SkippedStatement warning
        assert!(
            file.errors.iter().any(|e| e.kind == ParseErrorKind::SkippedStatement),
            "expected SkippedStatement warning, got: {:?}",
            file.errors
        );
        for err in &file.errors {
            if err.kind == ParseErrorKind::SkippedStatement {
                assert_eq!(err.severity, Severity::Warning);
            }
        }
    }

    #[test]
    fn error_has_lsp_compatible_span() {
        // Put the malformed bit on line 1 so we can verify line/character
        let file = parse("f(x) := x$\n!@# $\ng(y) := y$");
        let skipped = file
            .errors
            .iter()
            .find(|e| e.kind == ParseErrorKind::SkippedStatement)
            .expect("should have a SkippedStatement error");
        // The malformed "!@#" is on line 1 (0-indexed)
        assert_eq!(skipped.span.start.line, 1);
        assert_eq!(skipped.span.start.character, 0);
    }

    #[test]
    fn error_display_uses_kind() {
        let file = parse("!@# $");
        assert!(!file.errors.is_empty());
        let msg = file.errors[0].message();
        assert!(!msg.is_empty(), "error message should not be empty");
    }

    #[test]
    fn replace_terminators_simple() {
        assert_eq!(replace_terminators("a$ b$ c$"), "a, b, c,");
    }

    #[test]
    fn replace_terminators_semicolons() {
        assert_eq!(replace_terminators("a; b; c;"), "a, b, c,");
    }

    #[test]
    fn replace_terminators_nested_parens() {
        assert_eq!(replace_terminators("f(a; b)$ g()$"), "f(a; b), g(),");
    }

    #[test]
    fn replace_terminators_strings() {
        assert_eq!(
            replace_terminators(r#"print("hello$world")$ x$"#),
            r#"print("hello$world"), x,"#,
        );
    }

    #[test]
    fn replace_terminators_comments() {
        assert_eq!(replace_terminators("/* a $ b */ x$"), "/* a $ b */ x,");
    }

    #[test]
    fn replace_terminators_multiline() {
        let code = "classify(-3)$\nclassify(0)$\nclassify(7)$";
        assert_eq!(
            replace_terminators(code),
            "classify(-3),\nclassify(0),\nclassify(7),",
        );
    }
}
