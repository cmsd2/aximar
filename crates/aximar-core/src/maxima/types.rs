use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::message::Message;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvalResult {
    pub cell_id: String,
    pub text_output: String,
    pub latex: Option<String>,
    /// Inline SVG content, read from the file Maxima wrote.  Populated by
    /// both the legacy stdout-scrape parser and the envelope overlay.
    pub plot_svg: Option<String>,
    pub plot_data: Option<String>,
    /// Base64-encoded PNG image data (from np_imshow)
    pub image_png: Option<String>,
    pub error: Option<String>,
    pub error_info: Option<ErrorInfo>,
    pub is_error: bool,
    pub duration_ms: u64,
    /// Maxima output label (e.g. "%o6") for stable back-references
    pub output_label: Option<String>,
    /// Maxima's one-line form of the final value, as `string(%)` gives it
    #[serde(default)]
    pub text_plain: Option<String>,
    /// Maxima's error message in the user's language, from its own
    /// catalog, when it has one (see `ax_messages.lisp`)
    #[serde(default)]
    pub error_translated: Option<String>,
}

/// Aximar's explanation of an error.  The text fields are [`Message`]s, in
/// English; see `maxima::errors`.
///
/// In JSON, `title`, `explanation` and `suggestion` are their English text,
/// as before IDs.  `id` names the explanation, `params` holds every
/// message's parameters, and `messages` gives each field's ID and English
/// template, which is what the window looks up in the user's language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "ErrorInfoJson", into = "ErrorInfoJson")]
pub struct ErrorInfo {
    /// Such as `error.division_by_zero`
    pub id: String,
    pub title: Message,
    pub explanation: Message,
    pub suggestion: Option<Message>,
    /// Maxima code to try, which stays the same in every language
    pub example: Option<String>,
    pub did_you_mean: Vec<String>,
    pub correct_signatures: Vec<String>,
}

impl ErrorInfo {
    /// An explanation with a title and an explanation, and nothing else
    pub fn new(id: &str, title: Message, explanation: Message) -> Self {
        ErrorInfo {
            id: id.to_string(),
            title,
            explanation,
            suggestion: None,
            example: None,
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: Message) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }

    /// Each message, by the name of its field
    pub fn messages(&self) -> impl Iterator<Item = (&'static str, &Message)> {
        [
            ("title", Some(&self.title)),
            ("explanation", Some(&self.explanation)),
            ("suggestion", self.suggestion.as_ref()),
        ]
        .into_iter()
        .filter_map(|(field, m)| Some((field, m?)))
    }
}

/// Where a message's translation is found: its context and English template
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MessageSource {
    id: String,
    template: String,
}

#[derive(Serialize, Deserialize)]
struct ErrorInfoJson {
    #[serde(default)]
    id: String,
    #[serde(default)]
    params: BTreeMap<String, String>,
    title: String,
    explanation: String,
    suggestion: Option<String>,
    example: Option<String>,
    did_you_mean: Vec<String>,
    correct_signatures: Vec<String>,
    #[serde(default)]
    messages: BTreeMap<String, MessageSource>,
}

impl From<ErrorInfo> for ErrorInfoJson {
    fn from(info: ErrorInfo) -> Self {
        let mut params = BTreeMap::new();
        let mut messages = BTreeMap::new();
        for (field, m) in info.messages() {
            params.extend(m.params.clone());
            messages.insert(
                field.to_string(),
                MessageSource {
                    id: m.id.clone(),
                    template: m.template.to_string(),
                },
            );
        }
        ErrorInfoJson {
            id: info.id,
            params,
            title: info.title.text,
            explanation: info.explanation.text,
            suggestion: info.suggestion.map(|m| m.text),
            example: info.example,
            did_you_mean: info.did_you_mean,
            correct_signatures: info.correct_signatures,
            messages,
        }
    }
}

impl From<ErrorInfoJson> for ErrorInfo {
    fn from(json: ErrorInfoJson) -> Self {
        // JSON from before IDs has text alone; a message is then its text
        let message = |field: &str, text: String| {
            let (id, template) = match json.messages.get(field) {
                Some(source) => (source.id.clone(), source.template.clone()),
                None => (String::new(), text.clone()),
            };
            let params = json
                .params
                .iter()
                .filter(|(name, _)| template.contains(&format!("{{{name}}}")))
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect();
            Message {
                id,
                template: Cow::Owned(template),
                params,
                text,
            }
        };
        ErrorInfo {
            title: message("title", json.title.clone()),
            explanation: message("explanation", json.explanation.clone()),
            suggestion: json.suggestion.clone().map(|t| message("suggestion", t)),
            id: json.id,
            example: json.example,
            did_you_mean: json.did_you_mean,
            correct_signatures: json.correct_signatures,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Starting,
    Ready,
    Busy,
    Stopped,
    Error(String),
}

impl SessionStatus {
    pub fn as_code(&self) -> u8 {
        match self {
            SessionStatus::Stopped => 0,
            SessionStatus::Starting => 1,
            SessionStatus::Ready => 2,
            SessionStatus::Busy => 3,
            SessionStatus::Error(_) => 4,
        }
    }

    pub fn from_code(code: u8, error_msg: impl FnOnce() -> String) -> Self {
        match code {
            1 => SessionStatus::Starting,
            2 => SessionStatus::Ready,
            3 => SessionStatus::Busy,
            4 => SessionStatus::Error(error_msg()),
            _ => SessionStatus::Stopped,
        }
    }
}
