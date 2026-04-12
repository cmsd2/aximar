//! Variable inspection and evaluation: stack trace, scopes, variables, evaluate.

use super::*;

impl DapServer {
    pub(super) async fn handle_stack_trace(
        &mut self,
        request: &Request,
        _args: &emmy_dap_types::requests::StackTraceArguments,
    ) -> Result<(), TransportError> {
        let (bt_lines, bt_frame_args, canonical_paths) = match self.get_backtrace().await {
            Ok(result) => result,
            Err(e) => {
                return self
                    .send_error_response(
                        request.seq,
                        &format!("failed to get backtrace: {}", e),
                    )
                    .await;
            }
        };

        let program_path = self.program_path.clone().unwrap_or_default();
        let path_remaps = self.build_path_remaps();
        let cwd = self.launch_args.as_ref().and_then(|a| a.cwd.as_deref()).map(Path::new);
        let stack_frames =
            frames::parse_backtrace(&bt_lines, &self.source_index, &program_path, &path_remaps, cwd, &canonical_paths);

        // Cache for scopes/variables requests
        self.cached_frame_args = bt_frame_args;
        self.cached_frames = stack_frames.clone();

        // Reset variable references
        self.var_refs.clear();
        self.next_var_ref = 1;

        let total_frames = stack_frames.len() as i64;

        self.send_response(
            request,
            Some(ResponseBody::StackTrace(StackTraceResponse {
                stack_frames,
                total_frames: Some(total_frames),
            })),
        )
        .await
    }

    pub(super) async fn handle_scopes(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::ScopesArguments,
    ) -> Result<(), TransportError> {
        let frame_id = args.frame_id as u32;

        // Create a variable reference for this frame's locals
        let var_ref = self.next_var_ref;
        self.next_var_ref += 1;
        self.var_refs
            .insert(var_ref, VariableRef::FrameLocals { frame_index: frame_id });

        let scopes = vec![Scope {
            name: "Locals".to_string(),
            presentation_hint: Some(ScopePresentationhint::Locals),
            variables_reference: var_ref,
            named_variables: None,
            indexed_variables: None,
            expensive: false,
            source: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        }];

        self.send_response(
            request,
            Some(ResponseBody::Scopes(ScopesResponse { scopes })),
        )
        .await
    }

    pub(super) async fn handle_variables(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::VariablesArguments,
    ) -> Result<(), TransportError> {
        let var_ref = args.variables_reference;

        let variables = match self.var_refs.get(&var_ref).cloned() {
            Some(VariableRef::FrameLocals { frame_index }) => {
                let args_text = self
                    .cached_frame_args
                    .get(frame_index as usize)
                    .cloned()
                    .unwrap_or_default();
                let mut vars = frames::frame_args_to_variables(&args_text);

                // Also include block-local variables from the function definition
                let locals = self.get_block_locals_for_frame(frame_index).await;
                vars.extend(locals);

                vars
            }
            Some(VariableRef::Expandable { ref expression }) => {
                let expr = expression.clone();
                self.expand_variable(&expr).await
            }
            None => Vec::new(),
        };

        self.send_response(
            request,
            Some(ResponseBody::Variables(VariablesResponse { variables })),
        )
        .await
    }

    /// Look up block-local variables for a stack frame and evaluate their
    /// current values at the debugger prompt.
    async fn get_block_locals_for_frame(&mut self, frame_index: u32) -> Vec<Variable> {
        // Get the function name from the cached stack frame
        let function_name = match self.cached_frames.get(frame_index as usize) {
            Some(frame) => frame.name.clone(),
            None => return Vec::new(),
        };

        // Find block_locals from the parsed source
        let block_locals = self.find_block_locals(&function_name);
        if block_locals.is_empty() {
            return Vec::new();
        }

        // Suppress output while evaluating locals — these are internal
        // queries and should not appear in the Debug Console.
        self.suppress_output = true;

        let mut variables = Vec::new();
        for local_name in &block_locals {
            let value = match self.evaluate_at_debugger(local_name).await {
                Ok(v) => v.trim().to_string(),
                Err(_) => "?".to_string(),
            };
            variables.push(Variable {
                name: local_name.clone(),
                value,
                type_field: None,
                presentation_hint: None,
                evaluate_name: Some(local_name.clone()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                memory_reference: None,
            });
        }

        // Drain any remaining output from the evaluations while
        // still suppressed, so it goes to the protocol channel.
        let _ = self.flush_output().await;
        self.suppress_output = false;

        variables
    }

    /// Find block_locals for a function name from the source index.
    fn find_block_locals(&self, function_name: &str) -> Vec<String> {
        let program_path = match &self.program_path {
            Some(p) => p,
            None => return Vec::new(),
        };
        let mac_file = match self.source_index.get(program_path) {
            Some(f) => f,
            None => return Vec::new(),
        };
        for item in &mac_file.items {
            match item {
                MacItem::FunctionDef(f) | MacItem::MacroDef(f) if f.name == function_name => {
                    return f.block_locals.clone();
                }
                _ => continue,
            }
        }
        Vec::new()
    }

    pub(super) async fn handle_evaluate(
        &mut self,
        request: &Request,
        args: &emmy_dap_types::requests::EvaluateArguments,
    ) -> Result<(), TransportError> {
        use emmy_dap_types::types::EvaluateArgumentsContext;

        let expression = args.expression.clone();

        // Only allow evaluation when stopped at a debugger prompt
        if !matches!(self.state, DebugState::Stopped { .. }) {
            return self
                .send_error_response(
                    request.seq,
                    "can only evaluate expressions when stopped at a breakpoint",
                )
                .await;
        }

        // Suppress Maxima stdout for non-REPL evaluations (watch, hover,
        // variables panel) — these are internal queries whose raw output
        // should not leak to the Debug Console.
        let is_repl = matches!(
            args.context,
            Some(EvaluateArgumentsContext::Repl) | None
        );
        if !is_repl {
            self.suppress_output = true;
        }

        let result = match self.evaluate_at_debugger(&expression).await {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        };

        if !is_repl {
            let _ = self.flush_output().await;
            self.suppress_output = false;
        }

        self.send_response(
            request,
            Some(ResponseBody::Evaluate(EvaluateResponse {
                result,
                type_field: None,
                presentation_hint: None,
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                memory_reference: None,
            })),
        )
        .await
    }
}
