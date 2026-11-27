//! Maxima's error messages in the user's language.
//!
//! Maxima runs in English (see `process.rs`), so Aximar can read its
//! messages.  `ax_messages.lisp` wraps `errormsg()` so that, after each
//! English error message, Maxima prints the same message from its own
//! catalog for `ax__language`, on one tagged line.  The parser lifts that
//! line into `EvalResult::error_translated`.

use std::sync::OnceLock;

use crate::maxima::plotting::lisp_stdin_line;

const AX_MESSAGES_LISP: &str = include_str!("ax_messages.lisp");
const AX_QUESTIONS_LISP: &str = include_str!("ax_questions.lisp");

const TAG: &str = "__AXIMAR_MSG__";
const TAG_END: &str = "__AXIMAR_MSG_END__";

/// The `errormsg()` wrapper as a single `:lisp` line, for session init.
pub fn messages_lisp_stdin() -> &'static str {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED.get_or_init(|| lisp_stdin_line(AX_MESSAGES_LISP))
}

/// The override that turns Maxima's questions into errors, as a single
/// `:lisp` line for each process's start (see `ax_questions.lisp`)
pub fn questions_lisp_stdin() -> &'static str {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED.get_or_init(|| lisp_stdin_line(AX_QUESTIONS_LISP))
}

/// Maxima's code for the user's language, such as `es` or `pt_BR`: empty
/// for English, or for anything that isn't a plain language code
pub fn maxima_language_code(language: Option<&str>) -> String {
    let Some(language) = language else {
        return String::new();
    };
    // Maxima's catalogs use POSIX names: `pt_BR`, not `pt-BR`
    let code = language.replace('-', "_");
    let valid = (2..=12).contains(&code.len())
        && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid || code == "en" || code.starts_with("en_") {
        return String::new();
    }
    code
}

/// The `:lisp` line that sets the language of Maxima's translated
/// messages.  It returns no values, so Maxima prints nothing, and as Lisp
/// it leaves `%` and the input line numbers alone: it can go to a running
/// session just before a cell.  `code` comes from [`maxima_language_code`].
pub fn language_lisp_stdin(code: &str) -> String {
    format!(":lisp (progn (setq $ax__language \"{code}\") (values))\n")
}

/// The translated message on a tagged line, unescaped.
pub fn parse_translated_line(line: &str) -> Option<String> {
    let inner = line.trim().strip_prefix(TAG)?.strip_suffix(TAG_END)?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tagged_lines_are_unescaped() {
        assert_eq!(
            parse_translated_line(
                "__AXIMAR_MSG__diff: número de argumentos equivocados.__AXIMAR_MSG_END__"
            )
            .as_deref(),
            Some("diff: número de argumentos equivocados.")
        );
        assert_eq!(
            parse_translated_line(r"__AXIMAR_MSG__a\nb \\ c__AXIMAR_MSG_END__").as_deref(),
            Some("a\nb \\ c")
        );
        assert_eq!(
            parse_translated_line("diff: wrong number of arguments."),
            None
        );
    }

    #[test]
    fn only_plain_language_codes_are_set() {
        assert_eq!(maxima_language_code(Some("es")), "es");
        assert_eq!(maxima_language_code(Some("pt-BR")), "pt_BR");
        assert_eq!(maxima_language_code(Some("en")), "");
        assert_eq!(maxima_language_code(Some("en-GB")), "");
        assert_eq!(maxima_language_code(None), "");
        assert_eq!(maxima_language_code(Some("")), "");
        assert_eq!(maxima_language_code(Some("es\") (quit) (\"")), "");
        assert_eq!(
            language_lisp_stdin("es"),
            ":lisp (progn (setq $ax__language \"es\") (values))\n"
        );
    }

    #[test]
    fn the_lisp_is_one_line() {
        let line = messages_lisp_stdin();
        assert!(line.starts_with(":lisp (progn "));
        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.contains("__AXIMAR_MSG__"));
    }
}
