//! Text Aximar shows to people, ready for translation.
//!
//! [`msg!`](crate::msg) and [`msg_ctx!`](crate::msg_ctx) build a
//! [`Message`]: the English template, its parameters, and the English text
//! rendered from them.  aximar-core only ever renders English; the window
//! looks the template up in the user's language and fills in the same
//! parameters, falling back to `text`.
//!
//! `scripts/i18n-extract` finds every call with `xgettext`, so the template
//! (and the context of `msg_ctx!`) must be a string literal.  A note for
//! translators goes in a `// i18n:` comment directly above the call.
//! Placeholders are named, in braces (`{expr}`), and take no format spec:
//! a translation can only move them.  The text can't hold a literal brace,
//! which Rust and the window's ICU messages escape differently; the
//! catalog test rejects one.  See docs/i18n.md.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A piece of text in English, with what the window needs to translate it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// The message's stable ID, its `msgctxt` in the catalog, such as
    /// `error.division_by_zero.title`.  Empty for a message without context.
    pub id: String,
    /// The English template, its `msgid` in the catalog.
    pub template: Cow<'static, str>,
    /// The value of each placeholder in the template.
    pub params: BTreeMap<String, String>,
    /// The template rendered in English with `params`.
    pub text: String,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// A [`Message`] without context.
///
/// ```
/// use aximar_core::msg;
///
/// // i18n: a function's name, and the number of arguments it takes
/// let m = msg!("{func} expects {n} arguments", func = "diff", n = 2);
/// assert_eq!(m.text, "diff expects 2 arguments");
/// assert_eq!(m.template, "{func} expects {n} arguments");
/// assert_eq!(m.params["n"], "2");
/// ```
///
/// Each placeholder needs a parameter, and each parameter a placeholder:
///
/// ```compile_fail
/// let m = aximar_core::msg!("{func} expects {n} arguments", func = "diff");
/// ```
///
/// ```compile_fail
/// let m = aximar_core::msg!("{func} expects arguments", func = "diff", n = 2);
/// ```
///
/// A local variable can't stand in for a missing parameter:
///
/// ```compile_fail
/// let n = 2;
/// let m = aximar_core::msg!("{func} expects {n} arguments", func = "diff");
/// ```
#[macro_export]
macro_rules! msg {
    ($template:literal $(, $name:ident = $value:expr)* $(,)?) => {
        $crate::__message!("", $template $(, $name = $value)*)
    };
}

/// A [`Message`] with its ID as context.
///
/// ```
/// use aximar_core::msg_ctx;
///
/// let m = msg_ctx!("question.sign", "Maxima needs to know the sign of {expr}", expr = "a");
/// assert_eq!(m.id, "question.sign");
/// assert_eq!(m.text, "Maxima needs to know the sign of a");
/// ```
#[macro_export]
macro_rules! msg_ctx {
    ($id:literal, $template:literal $(, $name:ident = $value:expr)* $(,)?) => {
        $crate::__message!($id, $template $(, $name = $value)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __message {
    ($id:literal, $template:literal $(, $name:ident = $value:expr)*) => {{
        // Every value first, so one parameter's value can name another
        let ($($name,)*) = ($(::std::string::ToString::to_string(&$value),)*);
        // A function item can't see local variables, so a placeholder
        // without a parameter fails to compile instead of capturing one
        fn render($($name: &str),*) -> ::std::string::String {
            ::std::fmt::format(::std::format_args!($template $(, $name = $name)*))
        }
        let text = render($(&$name),*);
        $crate::message::Message {
            id: ::std::string::String::from($id),
            template: ::std::borrow::Cow::Borrowed($template),
            params: ::std::collections::BTreeMap::<::std::string::String, ::std::string::String>::from([
                $((::std::string::String::from(::std::stringify!($name)), $name)),*
            ]),
            text,
        }
    }};
}
