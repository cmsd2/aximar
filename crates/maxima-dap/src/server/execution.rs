//! Execution control: configurationDone, threads, continue, next, step_in,
//! step_out.

use crate::stepping::{self, AfterStep, StepOutStart};

use super::*;

impl DapServer {
    pub(super) async fn handle_configuration_done(
        &mut self,
        request: &Request,
    ) -> Result<(), TransportError> {
        self.send_response(request, Some(ResponseBody::ConfigurationDone))
            .await?;

        // Load function/macro definitions from the file via strategy.
        // Legacy: extract definitions to temp file and batchload.
        // Enhanced: no-op here (batchload happens in the evaluate phase).
        if let Err(e) = self.load_program_file().await {
            self.send_output_event(
                &format!("Error loading program: {}\n", e),
                OutputEventCategory::Stderr,
            )
            .await?;
            self.state = DebugState::Terminated;
            self.send_event(Event::Terminated(None)).await?;
            return Ok(());
        }

        // Set any pending breakpoints now that functions are defined (Legacy).
        // For Enhanced, breakpoints were already set as deferred during
        // handle_set_breakpoints.
        self.set_pending_breakpoints().await?;

        // Build the expression to evaluate via strategy.
        let program_path = self.program_path.clone().unwrap_or_default();
        let evaluate_expr = self
            .launch_args
            .as_ref()
            .and_then(|a| a.evaluate.clone())
            .filter(|e| !e.trim().is_empty());

        let expr = if let Some(strategy) = self.strategy.as_ref() {
            strategy.build_evaluate_expression(
                &program_path,
                evaluate_expr.as_deref(),
                &self.source_index,
            )
        } else {
            evaluate_expr
        };

        let expr = match expr {
            Some(code) => code,
            None => {
                // No code to execute — file only has definitions (Legacy)
                // or batchload produced nothing (shouldn't happen for Enhanced).
                self.send_output_event(
                    "File contains only function definitions. \
                     Add an \"evaluate\" expression to your launch config \
                     to call a function and trigger breakpoints.\n",
                    OutputEventCategory::Console,
                )
                .await?;
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await?;
                return Ok(());
            }
        };

        tracing::debug!("configurationDone: evaluating {:?}", expr);
        match self.send_maxima_and_wait(&expr).await {
            Ok((
                PromptKind::Debugger {
                    level,
                    error_context,
                },
                canonical,
            )) => {
                tracing::debug!("configurationDone: hit breakpoint at level {}", level);
                self.state = DebugState::Stopped {
                    level,
                    canonical_file: canonical.as_ref().map(|c| c.file.clone()),
                    canonical_line: canonical.as_ref().map(|c| c.line),
                    error_context: error_context.clone(),
                };
                // For Enhanced mode, refresh deferred breakpoint status now
                // that batchload has resolved them.
                self.refresh_breakpoint_status().await?;
                let (reason, desc, text) = if let Some(ref err) = error_context {
                    (
                        StoppedEventReason::Exception,
                        Some(err.clone()),
                        Some(err.clone()),
                    )
                } else {
                    (StoppedEventReason::Breakpoint, None, None)
                };
                self.send_stopped_event(reason, desc, text).await?;
            }
            Ok((PromptKind::Normal, _)) => {
                tracing::debug!("configurationDone: expression completed without breakpoint");
                // Refresh before terminating too — breakpoints may have resolved
                // even though none fired.
                self.refresh_breakpoint_status().await?;
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await?;
            }
            Err(e) => {
                self.flush_output().await?;
                self.send_output_event(
                    &format!("Error evaluating expression: {}\n", e),
                    OutputEventCategory::Stderr,
                )
                .await?;
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await?;
            }
        }
        self.flush_output().await?;

        Ok(())
    }

    pub(super) async fn handle_threads(&mut self, request: &Request) -> Result<(), TransportError> {
        // Maxima is single-threaded
        self.send_response(
            request,
            Some(ResponseBody::Threads(ThreadsResponse {
                threads: vec![Thread {
                    id: 1,
                    name: "Maxima".to_string(),
                }],
            })),
        )
        .await
    }

    pub(super) async fn handle_continue(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::ContinueArguments,
    ) -> Result<(), TransportError> {
        self.send_response(
            request,
            Some(ResponseBody::Continue(ContinueResponse {
                all_threads_continued: Some(true),
            })),
        )
        .await?;

        let outcome = self.send_debugger_command(":resume").await;
        self.report_stop(outcome, StoppedEventReason::Breakpoint)
            .await?;

        Ok(())
    }

    pub(super) async fn handle_next(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::NextArguments,
    ) -> Result<(), TransportError> {
        self.send_response(request, Some(ResponseBody::Next))
            .await?;

        tracing::debug!("handle_next: sending :next (state={:?})", self.state);
        let outcome = self.send_debugger_command(":next").await;
        self.report_stop(outcome, StoppedEventReason::Step).await?;

        Ok(())
    }

    pub(super) async fn handle_step_in(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::StepInArguments,
    ) -> Result<(), TransportError> {
        self.send_response(request, Some(ResponseBody::StepIn))
            .await?;

        let outcome = self.send_debugger_command(":step").await;
        self.report_stop(outcome, StoppedEventReason::Step).await?;

        Ok(())
    }

    /// Maxima has no step-out command: step until the run is back in a
    /// caller (see `crate::stepping`). From top-level code there's no
    /// function to leave, so the run continues.
    pub(super) async fn handle_step_out(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::StepOutArguments,
    ) -> Result<(), TransportError> {
        self.send_response(request, Some(ResponseBody::StepOut))
            .await?;

        let start = match self.send_debugger_command_raw(":bt").await {
            Ok((lines, _)) => StepOutStart::from_backtrace(&lines),
            Err(e) => return self.report_stop(Err(e), StoppedEventReason::Step).await,
        };
        let Some(start) = start else {
            let outcome = self.send_debugger_command(":resume").await;
            return self
                .report_stop(outcome, StoppedEventReason::Breakpoint)
                .await;
        };
        tracing::debug!("handle_step_out: from {:?}", start);

        loop {
            let (prompt, canonical, lines) = match self.send_debugger_command_lines(":step").await {
                Ok(result) => result,
                Err(e) => return self.report_stop(Err(e), StoppedEventReason::Step).await,
            };
            // The run ended or failed: report it as it is
            let PromptKind::Debugger {
                error_context: None,
                ..
            } = prompt
            else {
                return self
                    .report_stop(Ok((prompt, canonical)), StoppedEventReason::Step)
                    .await;
            };
            let on_breakpoint = canonical.as_ref().is_some_and(|stop| {
                stepping::at_breakpoint(stop, &self.build_path_remaps(), &self.breakpoints)
            });
            let decision = if on_breakpoint {
                AfterStep::AtBreakpoint
            } else {
                start.after_step(&lines)
            };
            match decision {
                AfterStep::Inside => continue,
                AfterStep::AtBreakpoint => {
                    return self
                        .report_stop(Ok((prompt, canonical)), StoppedEventReason::Breakpoint)
                        .await
                }
                AfterStep::CheckDepth => match self.send_debugger_command_raw(":bt").await {
                    Ok((bt, _)) if start.is_out(stepping::depth(&bt)) => {
                        return self
                            .report_stop(Ok((prompt, canonical)), StoppedEventReason::Step)
                            .await
                    }
                    Ok(_) => continue,
                    Err(e) => return self.report_stop(Err(e), StoppedEventReason::Step).await,
                },
            }
        }
    }

    /// Report where a debugger command left the run, as continue, next and
    /// step_in do: stopped (with `reason`, or as an exception on an error),
    /// ended, or failed.
    async fn report_stop(
        &mut self,
        outcome: Result<(PromptKind, Option<CanonicalLocation>), AppError>,
        reason: StoppedEventReason,
    ) -> Result<(), TransportError> {
        match outcome {
            Ok((
                PromptKind::Debugger {
                    level,
                    error_context,
                },
                canonical,
            )) => {
                self.state = DebugState::Stopped {
                    level,
                    canonical_file: canonical.as_ref().map(|c| c.file.clone()),
                    canonical_line: canonical.as_ref().map(|c| c.line),
                    error_context: error_context.clone(),
                };
                self.refresh_breakpoint_status().await?;
                self.flush_output().await?;
                let (reason, desc, text) = match error_context {
                    Some(err) => (StoppedEventReason::Exception, Some(err.clone()), Some(err)),
                    None => (reason, None, None),
                };
                self.send_stopped_event(reason, desc, text).await
            }
            Ok((PromptKind::Normal, _)) => {
                self.state = DebugState::Terminated;
                self.flush_output().await?;
                self.send_event(Event::Terminated(None)).await
            }
            Err(e) => {
                self.flush_output().await?;
                self.send_output_event(&format!("Error: {}\n", e), OutputEventCategory::Stderr)
                    .await?;
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await
            }
        }
    }
}
