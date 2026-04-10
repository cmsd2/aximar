//! Session lifecycle handlers: initialize, launch, disconnect.

use super::*;

impl DapServer {
    pub(super) async fn handle_initialize(
        &mut self,
        request: &Request,
        args: &InitializeArguments,
    ) -> Result<(), TransportError> {
        self.lines_start_at_1 = args.lines_start_at1.unwrap_or(true);

        let capabilities = Capabilities {
            supports_configuration_done_request: Some(true),
            supports_evaluate_for_hovers: Some(true),
            supports_function_breakpoints: Some(false),
            supports_conditional_breakpoints: Some(false),
            supports_step_back: Some(false),
            ..Default::default()
        };

        self.state = DebugState::Initialized;
        self.send_response(request, Some(ResponseBody::Initialize(capabilities)))
            .await?;

        // Send the initialized event
        self.send_event(Event::Initialized).await?;

        Ok(())
    }

    pub(super) async fn handle_launch(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::LaunchRequestArguments,
    ) -> Result<(), TransportError> {
        // Parse our custom launch arguments from additional_data
        let launch_args: MaximaLaunchArguments = match &args.additional_data {
            Some(data) => match serde_json::from_value(data.clone()) {
                Ok(a) => a,
                Err(e) => {
                    return self
                        .send_error_response(
                            request.seq,
                            &format!("invalid launch arguments: {}", e),
                        )
                        .await;
                }
            },
            None => {
                return self
                    .send_error_response(request.seq, "missing launch arguments")
                    .await;
            }
        };

        let program_path = PathBuf::from(&launch_args.program);
        if !program_path.exists() {
            return self
                .send_error_response(
                    request.seq,
                    &format!("program not found: {}", launch_args.program),
                )
                .await;
        }

        // Parse the backend
        let backend = match launch_args.backend.as_str() {
            "local" => Backend::Local,
            other => {
                return self
                    .send_error_response(
                        request.seq,
                        &format!(
                            "unsupported backend: {} (only 'local' supported for debugging)",
                            other
                        ),
                    )
                    .await;
            }
        };

        // Spawn Maxima process
        let custom_path = launch_args.maxima_path.clone();
        let process =
            match MaximaProcess::spawn(backend, custom_path, self.output_sink.clone()).await {
                Ok(p) => p,
                Err(e) => {
                    return self
                        .send_error_response(
                            request.seq,
                            &format!("failed to start Maxima: {}", e),
                        )
                        .await;
                }
            };

        self.process = Some(process);
        self.program_path = Some(program_path.clone());
        self.launch_args = Some(launch_args.clone());

        // Enable debug mode
        if let Err(e) = self.send_maxima("debugmode(true)$\n").await {
            return self
                .send_error_response(
                    request.seq,
                    &format!("failed to enable debug mode: {}", e),
                )
                .await;
        }
        // Switch to chunk-based reading so read_until_sentinel can detect
        // debugger prompts that appear without a trailing newline.
        if let Some(process) = self.process.as_mut() {
            process.set_debug_mode(true);
        }

        // Suppress output during internal :lisp probes — their raw
        // responses (e.g. "SBCL", "NIL", "T") are not user-relevant.
        self.suppress_output = true;

        // Detect backend (SBCL required for full debugging)
        if let Err(e) = self.check_lisp_backend().await {
            let _ = self.flush_output().await;
            self.suppress_output = false;
            self.send_output_event(&format!("Warning: {}\n", e), OutputEventCategory::Console)
                .await?;
            self.suppress_output = true;
        }

        // Detect Enhanced Maxima debugger support.
        let enhanced = self.detect_enhanced_debugger().await;
        let _ = self.flush_output().await;
        self.suppress_output = false;
        if enhanced {
            tracing::info!("Enhanced Maxima debugger detected — using file:line breakpoints");
            self.strategy = Some(Box::new(EnhancedStrategy));
        } else {
            tracing::info!("Legacy Maxima debugger — using function+offset breakpoints");
            self.strategy = Some(Box::new(LegacyStrategy));
        }

        // Index the source file for breakpoint mapping (but don't load
        // definitions yet — we defer to configurationDone so that
        // breakpoints can be set before top-level code runs).
        if let Err(e) = self.source_index.index_file(&program_path) {
            self.send_output_event(
                &format!(
                    "Warning: could not parse {} for breakpoint mapping: {}\n",
                    program_path.display(),
                    e
                ),
                OutputEventCategory::Console,
            )
            .await?;
        }

        self.state = DebugState::Running;

        // Send the launch response
        self.send_response(request, Some(ResponseBody::Launch))
            .await?;

        // Flush any Maxima output as DAP output events
        self.flush_output().await?;

        Ok(())
    }

    pub(super) async fn handle_disconnect(
        &mut self,
        request: &Request,
    ) -> Result<(), TransportError> {
        // Clean up the Maxima process
        if let Some(ref mut process) = self.process {
            let _ = process.write_stdin(":top\n").await;
            let _ = process.kill().await;
        }
        self.process = None;
        self.state = DebugState::Terminated;

        self.send_response(request, Some(ResponseBody::Disconnect))
            .await?;

        self.send_event(Event::Terminated(None)).await?;

        Ok(())
    }
}
