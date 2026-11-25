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

const TAG: &str = "__AXIMAR_MSG__";
const TAG_END: &str = "__AXIMAR_MSG_END__";

/// The `errormsg()` wrapper as a single `:lisp` line, for session init.
pub fn messages_lisp_stdin() -> &'static str {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED.get_or_init(|| lisp_stdin_line(AX_MESSAGES_LISP))
}

/// The Maxima statement that sets the language of translated messages,
/// or `None` if `language` isn't a plain language code such as `es` or
/// `pt_BR`.
pub fn set_language_code(language: &str) -> Option<String> {
    let valid = (2..=12).contains(&language.len())
        && language
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    valid.then(|| format!("ax__language: \"{language}\"$"))
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
        assert_eq!(
            set_language_code("es").as_deref(),
            Some("ax__language: \"es\"$")
        );
        assert!(set_language_code("pt_BR").is_some());
        assert_eq!(set_language_code(""), None);
        assert_eq!(set_language_code("es\"$ kill(all)$ \""), None);
    }

    #[test]
    fn the_lisp_is_one_line() {
        let line = messages_lisp_stdin();
        assert!(line.starts_with(":lisp (progn "));
        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.contains("__AXIMAR_MSG__"));
    }
}
