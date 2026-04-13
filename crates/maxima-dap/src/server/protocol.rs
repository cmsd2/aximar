//! DAP transport helpers and output filtering.

use super::*;

/// Matches Maxima input/output labels like `(%i1)`, `(%o42)`, `(%t3)`.
pub(super) static MAXIMA_LABEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(%[iot]\d+\)\s*$").unwrap());

/// Matches debugger source location lines: `/path/to/file.mac:12::`
pub(super) static DEBUGGER_SOURCE_LOC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^.+:\d+::$").unwrap());

/// Matches backtrace frames: `#0: func(args) (file.mac line 7)`
pub(super) static BACKTRACE_FRAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#\d+:\s+\w+\(").unwrap());

/// Matches breakpoint/step location: `(file.mac line 14, in function $ADD)`
pub(super) static BREAKPOINT_LOC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(.*\bline\s+\d+.*in function\b").unwrap());

impl DapServer {
    pub(super) fn next_seq(&mut self) -> i64 {
        self.seq_counter += 1;
        self.seq_counter
    }

    pub(super) async fn send_response(
        &mut self,
        request: &Request,
        body: Option<ResponseBody>,
    ) -> Result<(), TransportError> {
        let seq = self.next_seq();
        let msg = BaseMessage {
            seq,
            message: Sendable::Response(Response {
                request_seq: request.seq,
                success: true,
                message: None,
                body,
                error: None,
            }),
        };
        self.transport.write_message(&msg).await
    }

    pub(super) async fn send_error_response(
        &mut self,
        request_seq: i64,
        message: &str,
    ) -> Result<(), TransportError> {
        let seq = self.next_seq();
        let msg = BaseMessage {
            seq,
            message: Sendable::Response(Response {
                request_seq,
                success: false,
                message: Some(emmy_dap_types::responses::ResponseMessage::Error(
                    message.to_string(),
                )),
                body: None,
                error: None,
            }),
        };
        self.transport.write_message(&msg).await
    }

    pub(super) async fn send_event(&mut self, event: Event) -> Result<(), TransportError> {
        let seq = self.next_seq();
        let msg = BaseMessage {
            seq,
            message: Sendable::Event(event),
        };
        self.transport.write_message(&msg).await
    }

    /// Send a custom DAP event not defined in the standard spec.
    ///
    /// VS Code silently ignores unknown event types, but companion
    /// extensions can intercept them via `onDidReceiveDebugSessionCustomEvent`.
    pub(super) async fn send_custom_event<T: serde::Serialize>(
        &mut self,
        event_name: &'static str,
        body: T,
    ) -> Result<(), TransportError> {
        let msg = types::CustomEvent {
            seq: self.next_seq(),
            message_type: "event",
            event: event_name,
            body,
        };
        self.transport.write_message(&msg).await
    }

    pub(super) async fn send_stopped_event(
        &mut self,
        reason: StoppedEventReason,
        description: Option<String>,
        text: Option<String>,
    ) -> Result<(), TransportError> {
        self.send_event(Event::Stopped(StoppedEventBody {
            reason,
            description,
            thread_id: Some(1),
            preserve_focus_hint: None,
            text,
            all_threads_stopped: Some(true),
            hit_breakpoint_ids: None,
        }))
        .await
    }

    pub(super) async fn send_output_event(
        &mut self,
        text: &str,
        category: OutputEventCategory,
    ) -> Result<(), TransportError> {
        self.send_event(Event::Output(OutputEventBody {
            category: Some(category),
            output: text.to_string(),
            group: None,
            variables_reference: None,
            source: None,
            line: None,
            column: None,
            data: None,
        }))
        .await
    }

    /// Flush buffered Maxima output as DAP output events, filtering out
    /// internal protocol noise (sentinels, prompts, labels, debugger
    /// commands) so the Debug Console shows only user-visible output.
    ///
    /// Filtered lines are still sent as custom `maxima-output` events
    /// so the companion VS Code extension can display them in a
    /// dedicated "Maxima Protocol" output channel.
    pub(super) async fn flush_output(&mut self) -> Result<(), TransportError> {
        let events = self.output_sink.drain();
        for event in events {
            if self.suppress_output || Self::is_noise(&event) {
                // Send as custom event for the extension's protocol output channel.
                self.send_custom_event(
                    "maxima-output",
                    types::MaximaOutputEventBody {
                        category: event.stream.clone(),
                        output: event.line.clone(),
                    },
                )
                .await?;
                continue;
            }
            let category = match event.stream.as_str() {
                "stderr" => OutputEventCategory::Stderr,
                _ => OutputEventCategory::Stdout,
            };
            self.send_output_event(&format!("{}\n", event.line), category)
                .await?;
        }
        Ok(())
    }

    /// Returns true if the output line is internal protocol noise that
    /// should not be shown to the user in the Debug Console.
    fn is_noise(event: &OutputEvent) -> bool {
        let line = event.line.trim();

        // Never show stdin echoes (commands sent to Maxima).
        if event.stream == "stdin" {
            return true;
        }

        // Empty lines.
        if line.is_empty() {
            return true;
        }

        // Any sentinel strings (DAP and aximar-core).
        if line.contains("__MAXIMA_DAP_DONE__")
            || line.contains("__dap_r__")
            || line.contains("__AXIMAR_")
        {
            return true;
        }

        // Maxima startup messages: "Loading /path/to/maxima-init.mac"
        if line.starts_with("Loading ") {
            return true;
        }

        // Maxima input/output labels: (%i1), (%o1), (%i42), etc.
        if MAXIMA_LABEL_RE.is_match(line) {
            return true;
        }

        // Debugger prompts: (dbm:1)
        if debugger::detect_debugger_prompt(line).is_some() {
            return true;
        }

        // Debugger source location lines: /path/to/file.mac:12::
        if DEBUGGER_SOURCE_LOC_RE.is_match(line) {
            return true;
        }

        // Backtrace frames: #0: add(a=3,b=4) (file.mac line 12)
        // These come from automatic :bt calls; VS Code shows the
        // stack trace in its own panel.
        if BACKTRACE_FRAME_RE.is_match(line) {
            return true;
        }

        // Breakpoint/step location: (file.mac line 14, in function $ADD)
        if BREAKPOINT_LOC_RE.is_match(line) {
            return true;
        }

        // Breakpoint messages: set confirmations ("Bkpt 1 for add at line 0")
        // and hit notifications ("Bkpt 0: (file.mac line 3, in function $add)").
        // VS Code shows breakpoint status via its own UI.
        if line.starts_with("Bkpt ") {
            return true;
        }

        // Enhanced Maxima debugger messages.
        if line.starts_with("Deferred breakpoint")
            || line.starts_with("Breakpoint at") // deferred: "Breakpoint at file line N deferred ..."
            || line.starts_with("Breakpoint resolved")
            || line.starts_with("Breakpoint re-applied")
            || line.starts_with("Resolving deferred breakpoint")
            || line.contains("deferred (file not yet loaded)")
            || line.contains("no executable code; adjusted to line")
        {
            return true;
        }

        // No line info messages from Legacy breakpoint attempts on
        // functions from other files.
        if line.starts_with("No line info for") {
            return true;
        }

        // Bare words from batchload/debugmode/backend detection/lisp probes.
        if matches!(line, "done" | "true" | "false" | "NIL" | "T"
            | "SBCL" | "GCL" | "CLISP" | "ECL") {
            return true;
        }

        false
    }
}
