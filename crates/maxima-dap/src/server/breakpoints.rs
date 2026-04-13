//! Breakpoint management: set, load, pending, refresh.

use super::*;

impl DapServer {
    pub(super) async fn handle_set_breakpoints(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::SetBreakpointsArguments,
    ) -> Result<(), TransportError> {
        let source_path = args
            .source
            .path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_default();

        // Clear any existing breakpoints for this file in Maxima
        if let Some(existing) = self.breakpoints.get(&source_path) {
            let ids_to_delete: Vec<u32> = existing
                .iter()
                .filter_map(|bp| bp.maxima_id)
                .collect();
            if !ids_to_delete.is_empty() {
                if let (Some(strategy), Some(process)) =
                    (self.strategy.as_ref(), self.process.as_mut())
                {
                    let mut ctx = StrategyContext {
                        process,
                        state: &self.state,
                        source_index: &self.source_index,
                    };
                    for maxima_id in ids_to_delete {
                        strategy.delete_breakpoint(&mut ctx, maxima_id).await;
                    }
                }
            }
        }

        let mut dap_breakpoints = Vec::new();
        let mut mapped_breakpoints = Vec::new();

        let source_breakpoints = args.breakpoints.as_deref().unwrap_or(&[]);

        // Index the file if not already done
        if self.source_index.get(&source_path).is_none() {
            let _ = self.source_index.index_file(&source_path);
        }

        let is_deferred = self
            .strategy
            .as_ref()
            .map(|s| s.supports_deferred_breakpoints())
            .unwrap_or(false);


        for src_bp in source_breakpoints {
            let line = src_bp.line;
            let bp_id = self.next_breakpoint_id;
            self.next_breakpoint_id += 1;

            if is_deferred {
                // Enhanced mode: set file:line breakpoint directly (or deferred).
                // We can always try to set the breakpoint — Enhanced supports
                // deferred breakpoints even before the file is loaded.
                if let (Some(strategy), Some(process)) =
                    (self.strategy.as_ref(), self.process.as_mut())
                {
                    let mut ctx = StrategyContext {
                        process,
                        state: &self.state,
                        source_index: &self.source_index,
                    };
                    let result =
                        strategy.set_breakpoint(&mut ctx, &source_path, line).await;

                    let display_line = result.actual_line.unwrap_or(line);
                    dap_breakpoints.push(Breakpoint {
                        id: Some(bp_id),
                        verified: result.verified,
                        source: Some(Source {
                            path: Some(source_path.to_string_lossy().to_string()),
                            ..Default::default()
                        }),
                        line: Some(display_line),
                        message: result.message.clone(),
                        ..Default::default()
                    });

                    mapped_breakpoints.push(MappedBreakpoint {
                        dap_id: bp_id,
                        source_path: source_path.clone(),
                        line,
                        function: None,
                        offset: None,
                        verified: result.verified,
                        actual_line: result.actual_line,
                        maxima_id: result.maxima_id,
                        message: result.message,
                    });
                } else {
                    // No strategy yet — shouldn't happen, but handle gracefully
                    dap_breakpoints.push(Breakpoint {
                        id: Some(bp_id),
                        verified: false,
                        source: Some(Source {
                            path: Some(source_path.to_string_lossy().to_string()),
                            ..Default::default()
                        }),
                        line: Some(line),
                        message: Some("Strategy not initialized".to_string()),
                        ..Default::default()
                    });
                    mapped_breakpoints.push(MappedBreakpoint {
                        dap_id: bp_id,
                        source_path: source_path.clone(),
                        line,
                        function: None,
                        offset: None,
                        verified: false,
                        actual_line: None,
                        maxima_id: None,
                        message: Some("Strategy not initialized".to_string()),
                    });
                }
            } else {
                // Legacy mode: map line to function+offset, set if file loaded
                let mapping = LegacyStrategy::map_line(
                    &self.source_index,
                    &source_path,
                    line,
                );
                tracing::debug!(
                    "setBreakpoints: line {} -> func={:?} offset={:?}",
                    line,
                    mapping.function_name,
                    mapping.offset,
                );

                if mapping.function_name.is_some() {
                    let (verified, maxima_id) = if self.file_loaded {
                        if let (Some(strategy), Some(process)) =
                            (self.strategy.as_ref(), self.process.as_mut())
                        {
                            let mut ctx = StrategyContext {
                                process,
                                state: &self.state,
                                source_index: &self.source_index,
                            };
                            let result = strategy
                                .set_breakpoint(&mut ctx, &source_path, line)
                                .await;
                            (result.verified, result.maxima_id)
                        } else {
                            (false, None)
                        }
                    } else {
                        (false, None)
                    };

                    let pending = !self.file_loaded;
                    let function_name = mapping.function_name.clone().unwrap();
                    let message = if pending {
                        Some("Pending — will be set when file is loaded".to_string())
                    } else if !verified {
                        Some(format!("Could not set breakpoint in {}", function_name))
                    } else {
                        None
                    };

                    dap_breakpoints.push(Breakpoint {
                        id: Some(bp_id),
                        verified,
                        source: Some(Source {
                            path: Some(source_path.to_string_lossy().to_string()),
                            ..Default::default()
                        }),
                        line: Some(line),
                        message: message.clone(),
                        ..Default::default()
                    });

                    mapped_breakpoints.push(MappedBreakpoint {
                        dap_id: bp_id,
                        source_path: source_path.clone(),
                        line,
                        function: mapping.function_name,
                        offset: mapping.offset,
                        verified,
                        actual_line: None,
                        maxima_id,
                        message,
                    });
                } else {
                    let message = mapping
                        .message
                        .unwrap_or_else(|| "Could not parse source file".to_string());

                    dap_breakpoints.push(Breakpoint {
                        id: Some(bp_id),
                        verified: false,
                        source: Some(Source {
                            path: Some(source_path.to_string_lossy().to_string()),
                            ..Default::default()
                        }),
                        line: Some(line),
                        message: Some(message.clone()),
                        ..Default::default()
                    });

                    mapped_breakpoints.push(MappedBreakpoint {
                        dap_id: bp_id,
                        source_path: source_path.clone(),
                        line,
                        function: None,
                        offset: None,
                        verified: false,
                        actual_line: None,
                        maxima_id: None,
                        message: Some(message),
                    });
                }
            }
        }

        self.breakpoints.insert(source_path, mapped_breakpoints);

        self.send_response(
            request,
            Some(ResponseBody::SetBreakpoints(SetBreakpointsResponse {
                breakpoints: dap_breakpoints,
            })),
        )
        .await
    }

    /// Load the program file into Maxima via the active strategy.
    ///
    /// Legacy: Extracts definitions to temp file, batchloads it.
    /// Enhanced: No-op (batchload happens in the evaluate phase).
    pub(super) async fn load_program_file(&mut self) -> Result<(), AppError> {
        let program_path = self
            .program_path
            .clone()
            .ok_or(AppError::ProcessNotRunning)?;

        let strategy = self
            .strategy
            .as_ref()
            .ok_or(AppError::CommunicationError("no strategy set".to_string()))?;
        let process = self
            .process
            .as_mut()
            .ok_or(AppError::ProcessNotRunning)?;

        let mut ctx = StrategyContext {
            process,
            state: &self.state,
            source_index: &self.source_index,
        };

        let result = strategy.load_program(&mut ctx, &program_path).await?;

        // Store temp file resources if the strategy created them (Legacy).
        if result.temp_file.is_some() {
            self.defs_temp_path = Some(result.loaded_path);
            self.defs_temp_file = result.temp_file;
        }

        self.file_loaded = true;
        Ok(())
    }

    /// Set all pending (unverified) breakpoints in Maxima and notify the
    /// client of their updated status via breakpoint-changed events.
    ///
    /// For Legacy mode: sets `:break func offset` now that functions are defined.
    /// For Enhanced mode: breakpoints were already set as deferred, so this
    /// is a no-op (they'll resolve during batchload).
    pub(super) async fn set_pending_breakpoints(&mut self) -> Result<(), TransportError> {
        let is_deferred = self
            .strategy
            .as_ref()
            .map(|s| s.supports_deferred_breakpoints())
            .unwrap_or(false);

        // Deferred breakpoints resolve during batchload — nothing to do here.
        if is_deferred {
            return Ok(());
        }

        // Suppress output while setting breakpoints — Maxima may print
        // "No line info" errors for functions from other files.
        self.suppress_output = true;

        // Collect all pending breakpoints across all files.
        let all_paths: Vec<PathBuf> = self.breakpoints.keys().cloned().collect();
        for path in all_paths {
            let bps = match self.breakpoints.get(&path) {
                Some(bps) => bps.clone(),
                None => continue,
            };

            let mut updated = Vec::new();
            for mut bp in bps {
                if bp.verified || bp.function.is_none() {
                    // Already set or unmappable — keep as-is.
                    updated.push(bp);
                    continue;
                }

                // Use strategy to set the breakpoint
                if let (Some(strategy), Some(process)) =
                    (self.strategy.as_ref(), self.process.as_mut())
                {
                    let mut ctx = StrategyContext {
                        process,
                        state: &self.state,
                        source_index: &self.source_index,
                    };
                    let result = strategy
                        .set_breakpoint(&mut ctx, &bp.source_path, bp.line)
                        .await;

                    bp.verified = result.verified;
                    bp.maxima_id = result.maxima_id;
                    bp.message = result.message;
                } else {
                    let function = bp.function.as_ref().unwrap().clone();
                    bp.message = Some(format!("Could not set breakpoint in {}", function));
                }

                // Notify client of the breakpoint status.
                self.send_event(Event::Breakpoint(BreakpointEventBody {
                    reason: BreakpointEventReason::Changed,
                    breakpoint: Breakpoint {
                        id: Some(bp.dap_id),
                        verified: bp.verified,
                        line: Some(bp.line),
                        message: bp.message.clone(),
                        source: Some(Source {
                            path: Some(path.to_string_lossy().to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                }))
                .await?;

                updated.push(bp);
            }

            self.breakpoints.insert(path, updated);
        }

        let _ = self.flush_output().await;
        self.suppress_output = false;
        Ok(())
    }

    /// Refresh breakpoint status after deferred breakpoints have resolved.
    ///
    /// Enhanced mode only: uses breakpoint resolution messages collected
    /// from execution output (populated by `send_maxima_and_wait` and
    /// `send_debugger_command`), then sends breakpoint-changed events to
    /// VS Code with the updated line numbers.
    ///
    /// No-op for Legacy mode (no deferred breakpoints).
    pub(super) async fn refresh_breakpoint_status(&mut self) -> Result<(), TransportError> {
        let is_deferred = self
            .strategy
            .as_ref()
            .map(|s| s.supports_deferred_breakpoints())
            .unwrap_or(false);

        if !is_deferred {
            return Ok(());
        }

        // Consume resolutions collected from execution output.
        let resolved = std::mem::take(&mut self.pending_resolutions);

        if resolved.is_empty() {
            return Ok(());
        }

        // Match resolved Maxima breakpoints back to our stored DAP breakpoints.
        //
        // Two cases:
        // 1. Breakpoint already has a maxima_id (set while file was loaded) — match by ID
        // 2. Breakpoint has no maxima_id (deferred) — match by line proximity,
        //    scoped to the same source file to avoid cross-file mismatches.

        // Collect resolved breakpoints into a mutable vec so we can mark them as consumed
        let mut unmatched_resolved = resolved;

        tracing::debug!(
            "refresh_breakpoint_status: {} resolved breakpoints from execution output",
            unmatched_resolved.len(),
        );
        for r in &unmatched_resolved {
            tracing::debug!(
                "  resolved: maxima_id={} line={} file={:?}",
                r.maxima_id,
                r.line,
                r.file,
            );
        }

        let all_paths: Vec<PathBuf> = self.breakpoints.keys().cloned().collect();
        for path in all_paths {
            let bps = match self.breakpoints.get(&path) {
                Some(bps) => bps.clone(),
                None => continue,
            };

            let path_str = path.to_string_lossy();

            let mut updated = Vec::new();
            for mut bp in bps {
                let was_unverified = !bp.verified;

                // Try to match this breakpoint to a resolved Maxima breakpoint
                let match_idx = if let Some(mid) = bp.maxima_id {
                    // Case 1: already has a maxima_id — find by ID
                    unmatched_resolved.iter().position(|r| r.maxima_id == mid)
                } else if was_unverified {
                    // Case 2: deferred breakpoint with no ID — match by file path
                    // and line proximity. Execution output provides full paths.
                    unmatched_resolved
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| match &r.file {
                            Some(f) => f.as_str() == path_str.as_ref(),
                            None => false,
                        })
                        .min_by_key(|(_, r)| (bp.line - r.line).abs())
                        .map(|(idx, _)| idx)
                } else {
                    None
                };

                tracing::debug!(
                    "refresh_breakpoint_status: dap_id={} path={} line={} verified={} maxima_id={:?} → match_idx={:?}",
                    bp.dap_id,
                    path_str,
                    bp.line,
                    bp.verified,
                    bp.maxima_id,
                    match_idx,
                );

                if let Some(idx) = match_idx {
                    let r = unmatched_resolved.remove(idx);
                    bp.maxima_id = Some(r.maxima_id);
                    bp.verified = true;
                    bp.actual_line = if r.line != bp.line {
                        Some(r.line)
                    } else {
                        None
                    };
                    bp.message = None;

                    if was_unverified {
                        let display_line = bp.actual_line.unwrap_or(bp.line);
                        self.send_event(Event::Breakpoint(BreakpointEventBody {
                            reason: BreakpointEventReason::Changed,
                            breakpoint: Breakpoint {
                                id: Some(bp.dap_id),
                                verified: true,
                                line: Some(display_line),
                                message: None,
                                source: Some(Source {
                                    path: Some(path.to_string_lossy().to_string()),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        }))
                        .await?;
                    }
                }
                updated.push(bp);
            }

            self.breakpoints.insert(path, updated);
        }

        Ok(())
    }
}
