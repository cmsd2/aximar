use tower_lsp::lsp_types;

pub fn parser_pos_to_lsp(pos: maxima_mac_parser::Position) -> lsp_types::Position {
    lsp_types::Position {
        line: pos.line,
        character: pos.character,
    }
}

pub fn parser_span_to_lsp(span: maxima_mac_parser::Span) -> lsp_types::Range {
    lsp_types::Range {
        start: parser_pos_to_lsp(span.start),
        end: parser_pos_to_lsp(span.end),
    }
}

pub fn parser_severity_to_lsp(
    severity: maxima_mac_parser::Severity,
) -> lsp_types::DiagnosticSeverity {
    match severity {
        maxima_mac_parser::Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
        maxima_mac_parser::Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
    }
}

pub fn parse_error_to_diagnostic(
    error: &maxima_mac_parser::ParseError,
) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range: parser_span_to_lsp(error.span),
        severity: Some(parser_severity_to_lsp(error.severity)),
        source: Some("maxima".to_string()),
        message: error.message(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_mac_parser::{ParseError, ParseErrorKind, Position, Severity, Span};

    #[test]
    fn position_conversion() {
        let pos = Position {
            line: 5,
            character: 10,
        };
        let lsp = parser_pos_to_lsp(pos);
        assert_eq!(lsp.line, 5);
        assert_eq!(lsp.character, 10);
    }

    #[test]
    fn span_conversion() {
        let span = Span {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 3,
                character: 5,
            },
        };
        let range = parser_span_to_lsp(span);
        assert_eq!(range.start.line, 1);
        assert_eq!(range.end.line, 3);
        assert_eq!(range.end.character, 5);
    }

    #[test]
    fn error_to_diagnostic() {
        let error = ParseError {
            kind: ParseErrorKind::SkippedStatement,
            span: Span {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 5,
                },
            },
            severity: Severity::Warning,
        };
        let diag = parse_error_to_diagnostic(&error);
        assert_eq!(
            diag.severity,
            Some(lsp_types::DiagnosticSeverity::WARNING)
        );
        assert_eq!(diag.source, Some("maxima".to_string()));
        assert!(!diag.message.is_empty());
    }
}
