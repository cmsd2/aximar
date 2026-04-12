//! Maxima I/O helpers: sending commands, reading responses, variable expansion.

use super::*;

impl DapServer {
    /// Send a command to Maxima's stdin and read until sentinel.
    pub(super) async fn send_maxima(
        &mut self,
        cmd: &str,
    ) -> Result<(Vec<String>, PromptKind), AppError> {
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;
        let sentinel = "__MAXIMA_DAP_DONE__";
        process.write_stdin(cmd).await?;
        process
            .write_stdin(&format!("print(\"{}\")$\n", sentinel))
            .await?;
        process.read_until_sentinel(sentinel).await
    }

    /// Evaluate an expression and wait for either completion or a debugger
    /// breakpoint.
    ///
    /// The sentinel is embedded *inside* a wrapping `block()` so it only
    /// fires when the expression completes — it is never left as a separate
    /// line in stdin that could be consumed at a `dbm:>` prompt.
    ///
    /// A configurable timeout (`evalTimeout` in launch config, default 60s,
    /// 0 to disable) guards against hangs caused by parse errors that bypass
    /// the debugger, runaway computations, or blocking I/O (e.g. gnuplot).
    /// On timeout the Maxima process is interrupted and the sentinel is
    /// drained so the session can be reused.
    pub(super) async fn send_maxima_and_wait(
        &mut self,
        expr: &str,
    ) -> Result<(PromptKind, Option<CanonicalLocation>), AppError> {
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;
        let sentinel = "__MAXIMA_DAP_DONE__";
        let trimmed = expr
            .trim()
            .trim_end_matches(|c: char| c == ';' || c == '$');
        let wrapped = format!(
            "block([__dap_r__], __dap_r__: ({}), print(\"{}\"), __dap_r__)$\n",
            trimmed, sentinel
        );
        tracing::debug!("send_maxima_and_wait: sending {:?}", wrapped.trim());
        process.write_stdin(&wrapped).await?;

        let timeout_secs = self
            .launch_args
            .as_ref()
            .map(|a| a.eval_timeout)
            .unwrap_or(60);

        if timeout_secs == 0 {
            // No timeout — wait indefinitely (original behaviour).
            let (lines, prompt_kind) = process.read_dap_response(Some(sentinel)).await?;
            tracing::debug!(
                "send_maxima_and_wait: got {:?}, lines: {:?}",
                prompt_kind,
                lines
            );
            let canonical = debugger::find_canonical_location(&lines);
            return Ok((prompt_kind, canonical));
        }

        let timeout = std::time::Duration::from_secs(timeout_secs);
        match tokio::time::timeout(timeout, process.read_dap_response(Some(sentinel))).await {
            Ok(result) => {
                let (lines, prompt_kind) = result?;
                tracing::debug!(
                    "send_maxima_and_wait: got {:?}, lines: {:?}",
                    prompt_kind,
                    lines
                );
                let canonical = debugger::find_canonical_location(&lines);
                Ok((prompt_kind, canonical))
            }
            Err(_) => {
                tracing::warn!(
                    "send_maxima_and_wait: timed out after {}s, interrupting",
                    timeout_secs
                );
                // Re-borrow process after the timeout future is dropped.
                let process = self
                    .process
                    .as_mut()
                    .ok_or(AppError::ProcessNotRunning)?;
                process.interrupt_and_resync(sentinel).await;
                Err(AppError::Timeout(timeout_secs))
            }
        }
    }

    /// Send a debugger command (like `:step`, `:resume`) and wait for the
    /// next debugger prompt or normal prompt (sentinel from outer eval).
    ///
    /// Uses chunk-based reading to detect debugger prompts that lack a
    /// trailing newline.  The sentinel is NOT sent here — it was already
    /// queued by `send_maxima_and_wait` as part of the original expression
    /// evaluation.  When the expression eventually completes (after one or
    /// more `:resume` / `:step` commands), the sentinel fires and this
    /// returns `PromptKind::Normal`.
    pub(super) async fn send_debugger_command(
        &mut self,
        cmd: &str,
    ) -> Result<(PromptKind, Option<CanonicalLocation>), AppError> {
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;
        let sentinel = "__MAXIMA_DAP_DONE__";
        tracing::debug!("send_debugger_command: sending {:?}", cmd);
        process.write_stdin(&format!("{}\n", cmd)).await?;
        let (lines, prompt_kind) = process.read_dap_response(Some(sentinel)).await?;
        tracing::debug!(
            "send_debugger_command: got {:?}, lines: {:?}",
            prompt_kind,
            lines
        );
        let canonical = debugger::find_canonical_location(&lines);
        Ok((prompt_kind, canonical))
    }

    /// Send a raw debugger command and read until the debugger prompt.
    /// Used for commands where we know we'll stay in the debugger (like `:bt`).
    ///
    /// Uses chunk-based reading to detect the debugger prompt without a
    /// trailing newline.
    pub(super) async fn send_debugger_command_raw(
        &mut self,
        cmd: &str,
    ) -> Result<(Vec<String>, u32), AppError> {
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;
        process.write_stdin(&format!("{}\n", cmd)).await?;
        let (lines, prompt_kind) = process.read_dap_response(None).await?;
        match prompt_kind {
            PromptKind::Debugger { level } => Ok((lines, level)),
            PromptKind::Normal => Err(AppError::CommunicationError(
                "unexpected normal prompt in debugger context".into(),
            )),
        }
    }

    /// Get the backtrace from Maxima.
    /// Returns (raw lines, args text per frame, canonical paths per frame index).
    ///
    /// Delegates to the strategy's `resolve_frame_paths` method — Enhanced
    /// mode issues `:frame N` per frame to get canonical absolute paths;
    /// Legacy mode returns an empty map (heuristic resolution).
    pub(super) async fn get_backtrace(
        &mut self,
    ) -> Result<(Vec<String>, Vec<String>, HashMap<u32, CanonicalLocation>), AppError> {
        let (lines, _level) = self.send_debugger_command_raw(":bt").await?;

        let mut args_per_frame = Vec::new();
        let mut frame_indices = Vec::new();
        for line in &lines {
            if let Some(frame) = debugger::parse_backtrace_frame(line) {
                frame_indices.push(frame.index);
                args_per_frame.push(frame.args);
            }
        }

        // Delegate to strategy to resolve frame paths.
        let canonical_paths = if let Some(strategy) = self.strategy.as_ref() {
            let process = self
                .process
                .as_mut()
                .ok_or(AppError::ProcessNotRunning)?;
            let mut ctx = StrategyContext {
                process,
                state: &self.state,
                source_index: &self.source_index,
            };
            strategy.resolve_frame_paths(&mut ctx, &frame_indices).await
        } else {
            HashMap::new()
        };

        Ok((lines, args_per_frame, canonical_paths))
    }

    /// Evaluate an expression at the debugger prompt.
    pub(super) async fn evaluate_at_debugger(
        &mut self,
        expression: &str,
    ) -> Result<String, AppError> {
        let cmd = format!("{};\n", expression.trim_end_matches(|c| c == ';' || c == '$'));
        let (lines, _level) = self.send_debugger_command_raw(&cmd).await?;

        let result: Vec<&str> = lines
            .iter()
            .map(|l| l.as_str())
            .filter(|l| debugger::detect_debugger_prompt(l).is_none())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(result.join("\n"))
    }

    /// Expand a compound variable by evaluating sub-parts.
    pub(super) async fn expand_variable(&mut self, expression: &str) -> Vec<Variable> {
        let length_expr = format!("length({})", expression);
        let length = match self.evaluate_at_debugger(&length_expr).await {
            Ok(result) => result.trim().parse::<usize>().unwrap_or(0),
            Err(_) => return Vec::new(),
        };

        let mut variables = Vec::new();
        for i in 1..=length.min(100) {
            let part_expr = format!("part({}, {})", expression, i);
            let value = match self.evaluate_at_debugger(&part_expr).await {
                Ok(v) => v.trim().to_string(),
                Err(_) => "?".to_string(),
            };
            variables.push(Variable {
                name: format!("[{}]", i),
                value,
                type_field: None,
                presentation_hint: None,
                evaluate_name: Some(part_expr),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                memory_reference: None,
            });
        }
        variables
    }

    /// Build a path remapping table from temp file -> original source.
    ///
    /// Used by `frames::parse_backtrace` to translate Maxima's temp file
    /// references back to the user's original `.mac` file.
    pub(super) fn build_path_remaps(&self) -> HashMap<PathBuf, PathBuf> {
        let mut remaps = HashMap::new();
        if let (Some(temp), Some(original)) = (&self.defs_temp_path, &self.program_path) {
            remaps.insert(temp.clone(), original.clone());
            // Also map the canonical form in case Maxima resolves symlinks.
            if let Ok(canonical) = temp.canonicalize() {
                remaps.insert(canonical, original.clone());
            }
        }
        remaps
    }

    /// Detect whether the running Maxima supports Enhanced debugger features
    /// (file:line breakpoints, deferred breakpoints, line-snapping).
    ///
    /// Probes for the `set_breakpoint` function which is only present in
    /// patched Maxima.
    pub(super) async fn detect_enhanced_debugger(&mut self) -> bool {
        match self.send_maxima(":lisp (fboundp 'maxima::$set_breakpoint)\n").await {
            Ok((lines, _)) => {
                let output = lines.join(" ");
                output.contains("T") && !output.contains("NIL")
            }
            Err(_) => false,
        }
    }

    /// Check the Lisp backend and warn if it's not SBCL.
    pub(super) async fn check_lisp_backend(&mut self) -> Result<(), String> {
        match self.send_maxima(":lisp (lisp-implementation-type)\n").await {
            Ok((lines, _)) => {
                let output = lines.join(" ");
                if !output.contains("SBCL") {
                    return Err(format!(
                        "Maxima is running on a non-SBCL Lisp backend. \
                         Debugging features like :bt and :frame may not work correctly. \
                         Backend detected: {}",
                        output.trim()
                    ));
                }
                Ok(())
            }
            Err(e) => Err(format!("Could not detect Lisp backend: {}", e)),
        }
    }
}
