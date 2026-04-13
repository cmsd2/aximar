//! Execution control: configurationDone, threads, continue, next, step_in.

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
            Ok((PromptKind::Debugger { level }, canonical)) => {
                tracing::debug!("configurationDone: hit breakpoint at level {}", level);
                self.state = DebugState::Stopped {
                    level,
                    canonical_file: canonical.as_ref().map(|c| c.file.clone()),
                    canonical_line: canonical.as_ref().map(|c| c.line),
                };
                // For Enhanced mode, refresh deferred breakpoint status now
                // that batchload has resolved them.
                self.refresh_breakpoint_status().await?;
                self.send_stopped_event(StoppedEventReason::Breakpoint)
                    .await?;
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

    pub(super) async fn handle_threads(
        &mut self,
        request: &Request,
    ) -> Result<(), TransportError> {
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

        match self.send_debugger_command(":resume").await {
            Ok((PromptKind::Debugger { level }, canonical)) => {
                self.state = DebugState::Stopped {
                    level,
                    canonical_file: canonical.as_ref().map(|c| c.file.clone()),
                    canonical_line: canonical.as_ref().map(|c| c.line),
                };
                self.refresh_breakpoint_status().await?;
                self.flush_output().await?;
                self.send_stopped_event(StoppedEventReason::Breakpoint)
                    .await?;
            }
            Ok((PromptKind::Normal, _)) => {
                self.state = DebugState::Terminated;
                self.flush_output().await?;
                self.send_event(Event::Terminated(None)).await?;
            }
            Err(e) => {
                self.flush_output().await?;
                self.send_output_event(
                    &format!("Error: {}\n", e),
                    OutputEventCategory::Stderr,
                )
                .await?;
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await?;
            }
        }

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
        match self.send_debugger_command(":next").await {
            Ok((PromptKind::Debugger { level }, canonical)) => {
                tracing::debug!("handle_next: stopped at debugger level {}", level);
                self.state = DebugState::Stopped {
                    level,
                    canonical_file: canonical.as_ref().map(|c| c.file.clone()),
                    canonical_line: canonical.as_ref().map(|c| c.line),
                };
                self.refresh_breakpoint_status().await?;
                self.flush_output().await?;
                self.send_stopped_event(StoppedEventReason::Step).await?;
            }
            Ok((PromptKind::Normal, _)) => {
                tracing::debug!("handle_next: expression completed (sentinel reached)");
                self.state = DebugState::Terminated;
                self.flush_output().await?;
                self.send_event(Event::Terminated(None)).await?;
            }
            Err(e) => {
                tracing::error!("handle_next: error: {}", e);
                self.flush_output().await?;
                self.send_output_event(
                    &format!("Error: {}\n", e),
                    OutputEventCategory::Stderr,
                )
                .await?;
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await?;
            }
        }

        Ok(())
    }

    pub(super) async fn handle_step_in(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::StepInArguments,
    ) -> Result<(), TransportError> {
        self.send_response(request, Some(ResponseBody::StepIn))
            .await?;

        match self.send_debugger_command(":step").await {
            Ok((PromptKind::Debugger { level }, canonical)) => {
                self.state = DebugState::Stopped {
                    level,
                    canonical_file: canonical.as_ref().map(|c| c.file.clone()),
                    canonical_line: canonical.as_ref().map(|c| c.line),
                };
                self.refresh_breakpoint_status().await?;
                self.flush_output().await?;
                self.send_stopped_event(StoppedEventReason::Step).await?;
            }
            Ok((PromptKind::Normal, _)) => {
                self.state = DebugState::Terminated;
                self.flush_output().await?;
                self.send_event(Event::Terminated(None)).await?;
            }
            Err(e) => {
                self.flush_output().await?;
                self.send_output_event(
                    &format!("Error: {}\n", e),
                    OutputEventCategory::Stderr,
                )
                .await?;
                self.state = DebugState::Terminated;
                self.send_event(Event::Terminated(None)).await?;
            }
        }

        Ok(())
    }
}
