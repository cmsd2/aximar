//! kernel-events envelope types.
//!
//! Phase-A scaffolding: a strongly-typed view of the JSON envelopes
//! emitted by the `kernel-events` Maxima package over the fd-3 channel.
//! The eval pipeline still terminates on the legacy `__AXIMAR_EVAL_END__`
//! sentinel — these types exist so we can read the envelope stream in
//! parallel and start migrating one envelope kind at a time.
//!
//! Schema source of truth: kernel-events/schemas/envelopes/v1/. This
//! module mirrors the subset we currently consume; additive envelope
//! fields are tolerated via `serde(other)` on the kind enum and an
//! explicit `Unknown` arm for `type` strings we don't yet handle.

use serde::Deserialize;

/// One newline-delimited envelope read from the fd-3 channel.
///
/// Every variant carries its `type` discriminator implicitly through
/// serde's internally tagged representation; unknown types are
/// preserved as `Envelope::Unknown` rather than failing to parse, so
/// the reader stays forward-compatible with envelope grammar growth.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Envelope {
    Capabilities(Capabilities),
    Ready(Ready),
    EvalBegin(EvalBegin),
    EvalResult(EvalResult),
    EvalEnd(EvalEnd),
    Output(Output),
    Display(Display),
    Error(ErrorEnvelope),
    DebugEnter(DebugEnter),
    DebugLeave(DebugLeave),
    StdinRequest(StdinRequest),
    Vars(Vars),
    /// Catch-all for envelope types we don't yet model.  Preserves the
    /// raw JSON so a future version can lift it into a dedicated arm
    /// without losing data observed in the wild.
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub kernel_version: Option<String>,
    #[serde(default)]
    pub lisp_implementation: Option<String>,
    #[serde(default)]
    pub features: Option<serde_json::Value>,
    #[serde(default)]
    pub packages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ready {}

#[derive(Debug, Clone, Deserialize)]
pub struct EvalBegin {
    pub eval_id: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvalResult {
    pub eval_id: Option<String>,
    #[serde(default)]
    pub output_label: Option<String>,
    pub suppressed: bool,
    /// Raw mime bundle as a JSON object — the reader keeps it loose so
    /// we can extract text/plain, application/x-maxima-latex, etc. on
    /// demand without re-validating every payload at parse time.
    pub mime_bundle: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvalEnd {
    pub eval_id: Option<String>,
    pub status: EvalStatus,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvalStatus {
    Ok,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Output {
    #[serde(default)]
    pub eval_id: Option<String>,
    pub stream: StreamName,
    /// Always "text/plain" today; kept loose for future MIME types.
    pub mime: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamName {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Display {
    #[serde(default)]
    pub eval_id: Option<String>,
    pub mime_bundle: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorEnvelope {
    #[serde(default)]
    pub eval_id: Option<String>,
    pub kind: ErrorKind,
    pub message: String,
    #[serde(default)]
    pub condition_type: Option<String>,
    #[serde(default)]
    pub form: Option<String>,
    #[serde(default)]
    pub backtrace: Option<Vec<String>>,
    #[serde(default)]
    pub recoverable: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    MaximaError,
    LispError,
    ParserError,
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DebugEnter {
    #[serde(default)]
    pub eval_id: Option<String>,
    pub level: DebugLevel,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DebugLeave {
    #[serde(default)]
    pub eval_id: Option<String>,
    pub level: DebugLevel,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DebugLevel {
    Maxima,
    Lisp,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StdinRequest {
    #[serde(default)]
    pub eval_id: Option<String>,
    pub prompt: String,
    pub kind: StdinKind,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StdinKind {
    String,
    Expression,
    DebuggerCommand,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Vars {
    #[serde(default)]
    pub eval_id: Option<String>,
    /// Variable names (without the leading `$` Maxima prefix).
    #[serde(default)]
    pub vars: Vec<String>,
    /// mgrind-rendered values, parallel to `vars`.
    #[serde(default)]
    pub values_text: Vec<String>,
}

impl Envelope {
    /// Short label for logging — the envelope's `type` field, or
    /// "unknown" for the catch-all arm.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Envelope::Capabilities(_) => "capabilities",
            Envelope::Ready(_) => "ready",
            Envelope::EvalBegin(_) => "eval_begin",
            Envelope::EvalResult(_) => "eval_result",
            Envelope::EvalEnd(_) => "eval_end",
            Envelope::Output(_) => "output",
            Envelope::Display(_) => "display",
            Envelope::Error(_) => "error",
            Envelope::DebugEnter(_) => "debug_enter",
            Envelope::DebugLeave(_) => "debug_leave",
            Envelope::StdinRequest(_) => "stdin_request",
            Envelope::Vars(_) => "vars",
            Envelope::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Envelope {
        serde_json::from_str::<Envelope>(s).expect("envelope parses")
    }

    #[test]
    fn parses_capabilities() {
        let env = parse(
            r#"{"type":"capabilities","kernel_version":"5.47.0","lisp_implementation":"SBCL"}"#,
        );
        assert!(matches!(env, Envelope::Capabilities(_)));
        assert_eq!(env.kind_label(), "capabilities");
    }

    #[test]
    fn parses_ready() {
        let env = parse(r#"{"type":"ready"}"#);
        assert!(matches!(env, Envelope::Ready(_)));
    }

    #[test]
    fn parses_eval_begin_with_id() {
        let env = parse(r#"{"type":"eval_begin","eval_id":"e_42","started_at":"2026-06-07T12:34:56Z"}"#);
        match env {
            Envelope::EvalBegin(b) => assert_eq!(b.eval_id.as_deref(), Some("e_42")),
            _ => panic!("wrong arm"),
        }
    }

    #[test]
    fn parses_eval_end_status_ok() {
        let env = parse(r#"{"type":"eval_end","eval_id":"e_1","status":"ok","duration_ms":42}"#);
        match env {
            Envelope::EvalEnd(e) => {
                assert_eq!(e.status, EvalStatus::Ok);
                assert_eq!(e.duration_ms, Some(42));
            }
            _ => panic!("wrong arm"),
        }
    }

    #[test]
    fn parses_output() {
        let env = parse(r#"{"type":"output","eval_id":"e_1","stream":"stdout","mime":"text/plain","text":"hi\n"}"#);
        match env {
            Envelope::Output(o) => {
                assert_eq!(o.stream, StreamName::Stdout);
                assert_eq!(o.text, "hi\n");
            }
            _ => panic!("wrong arm"),
        }
    }

    #[test]
    fn parses_error_envelope() {
        let env = parse(r#"{"type":"error","eval_id":"e_1","kind":"maxima_error","message":"foo: bar"}"#);
        match env {
            Envelope::Error(e) => {
                assert_eq!(e.kind, ErrorKind::MaximaError);
                assert_eq!(e.message, "foo: bar");
            }
            _ => panic!("wrong arm"),
        }
    }

    #[test]
    fn unknown_type_is_tolerated() {
        let env = parse(r#"{"type":"some_future_envelope","foo":1}"#);
        assert!(matches!(env, Envelope::Unknown));
        assert_eq!(env.kind_label(), "unknown");
    }

    #[test]
    fn parses_display_with_plotly_mime() {
        let env = parse(
            r#"{"type":"display","eval_id":"e_1","mime_bundle":{"application/x-maxima-plotly":"{\"data\":[]}"}}"#,
        );
        match env {
            Envelope::Display(d) => {
                assert!(d.mime_bundle.contains_key("application/x-maxima-plotly"));
            }
            _ => panic!("wrong arm"),
        }
    }
}
