//! Shared session startup logic used by both the Tauri GUI and MCP server.
//!
//! This module provides [`ensure_session`] and [`spawn_and_init_session`] so that
//! both transports funnel through the same code path, avoiding duplication and
//! ensuring that status callbacks fire consistently.

use std::sync::Arc;

use crate::catalog::search::Catalog;
use crate::error::AppError;
use crate::maxima::backend::Backend;
use crate::maxima::output::OutputSink;
use crate::maxima::process::MaximaProcess;
use crate::maxima::protocol;
use crate::maxima::types::SessionStatus;
use crate::maxima::plotting::{plotting_init_code, plotting_lisp_stdin};
use crate::maxima::unicode::build_texput_init;
use crate::registry::NotebookContextRef;

/// Callback invoked on session status transitions.
/// Parameters: `(notebook_id, new_status)`.
pub type SessionStatusCallback = Arc<dyn Fn(&str, SessionStatus) + Send + Sync>;

/// Spawn a new Maxima process and run initialization code (texput setup).
///
/// **Precondition:** the caller must have already called
/// `ctx.session.begin_start()` and notified any status callback with
/// `SessionStatus::Starting`.
///
/// On success the session transitions to `Ready`.
/// On failure the session transitions to `Error`.
pub async fn spawn_and_init_session(
    ctx: &NotebookContextRef,
    backend: Backend,
    maxima_path: Option<String>,
    output_sink: Arc<dyn OutputSink>,
    catalog: &Catalog,
    eval_timeout: u64,
    on_status: Option<&SessionStatusCallback>,
) -> Result<(), AppError> {
    // ctx.path may be a directory (from create_session's path param) or a
    // file (from open_notebook). Use it directly if it's a directory,
    // otherwise take the parent.
    let cwd = ctx.path.as_deref().map(|p| {
        if p.is_dir() { p } else { p.parent().unwrap_or(p) }
    });
    match MaximaProcess::spawn_with_cwd(backend, maxima_path, output_sink, cwd).await {
        Ok(process) => {
            ctx.session.set_ready(process).await;

            // Configure texput so Greek letters render correctly in TeX output
            let init = build_texput_init();
            let mut guard = ctx.session.lock().await;
            if let Ok(p) = guard.process_mut() {
                let _ =
                    protocol::evaluate(p, "__init__", &init, catalog, eval_timeout).await;
                // Configure plot2d to produce SVG files (detectable by parser)
                // instead of opening a gnuplot window (not useful in notebook/MCP context).
                let _ =
                    protocol::evaluate(p, "__init__", "set_plot_option([plot_format, svg])$", catalog, eval_timeout)
                        .await;
                // Load Lisp helpers for plotting (ax__mktemp)
                let lisp_init = plotting_lisp_stdin();
                p.write_stdin(lisp_init).await.ok();
                // Load Aximar plotting functions (ax_plot2d, ax_draw2d, ax_draw3d)
                let plot_init = plotting_init_code();
                let _ =
                    protocol::evaluate(p, "__init__", plot_init, catalog, eval_timeout)
                        .await;
            }
            drop(guard);

            if let Some(cb) = on_status {
                cb(&ctx.id, SessionStatus::Ready);
            }
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            ctx.session.set_error(msg.clone()).await;
            if let Some(cb) = on_status {
                cb(&ctx.id, SessionStatus::Error(msg));
            }
            Err(e)
        }
    }
}

/// Ensure the Maxima session for a notebook is running.
///
/// - `Ready | Busy` → no-op.
/// - `Stopped | Error` → spawn a new process via [`spawn_and_init_session`].
/// - `Starting` → poll until the session resolves (up to 5 seconds).
///
/// The `build_sink` closure is only called when a new process must be spawned,
/// so callers don't need to pre-allocate an output sink on every call.
pub async fn ensure_session(
    ctx: &NotebookContextRef,
    backend: Backend,
    maxima_path: Option<String>,
    build_sink: impl FnOnce(&NotebookContextRef) -> Arc<dyn OutputSink>,
    catalog: &Catalog,
    eval_timeout: u64,
    on_status: Option<&SessionStatusCallback>,
) -> Result<(), AppError> {
    let status = ctx.session.status();
    match status {
        SessionStatus::Ready | SessionStatus::Busy => Ok(()),
        SessionStatus::Stopped | SessionStatus::Error(_) => {
            ctx.session.begin_start().await;
            if let Some(cb) = on_status {
                cb(&ctx.id, SessionStatus::Starting);
            }
            let output_sink = build_sink(ctx);
            spawn_and_init_session(
                ctx,
                backend,
                maxima_path,
                output_sink,
                catalog,
                eval_timeout,
                on_status,
            )
            .await
        }
        SessionStatus::Starting => {
            // Another task is starting the session; poll until it resolves.
            for _ in 0..50 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                match ctx.session.status() {
                    SessionStatus::Ready | SessionStatus::Busy => return Ok(()),
                    SessionStatus::Error(msg) => {
                        return Err(AppError::CommunicationError(msg));
                    }
                    SessionStatus::Stopped => {
                        return Err(AppError::ProcessNotRunning);
                    }
                    _ => continue,
                }
            }
            Err(AppError::ProcessNotRunning)
        }
    }
}
