//! Shared session startup logic used by both the Tauri GUI and MCP server.
//!
//! This module provides [`ensure_session`] and [`spawn_and_init_session`] so that
//! both transports funnel through the same code path, avoiding duplication and
//! ensuring that status callbacks fire consistently.

use std::sync::Arc;

use crate::catalog::search::Catalog;
use crate::error::AppError;
use crate::maxima::backend::Backend;
use crate::maxima::envelope::EnvelopeObserver;
use crate::maxima::messages::messages_lisp_stdin;
use crate::maxima::output::OutputSink;
use crate::maxima::plotting::{plotting_init_code, plotting_lisp_stdin};
use crate::maxima::process::MaximaProcess;
use crate::maxima::protocol;
use crate::maxima::types::SessionStatus;
use crate::maxima::unicode::build_texput_init;
use crate::registry::NotebookContextRef;

/// Callback invoked on session status transitions.
/// Parameters: `(notebook_id, new_status)`.
pub type SessionStatusCallback = Arc<dyn Fn(&str, SessionStatus) + Send + Sync>;

/// Which Maxima a session runs, and how long one evaluation may take.
#[derive(Debug, Clone)]
pub struct SessionSettings {
    pub backend: Backend,
    pub maxima_path: Option<String>,
    pub eval_timeout: u64,
}

impl SessionSettings {
    /// A provider that always gives these settings, for front ends whose
    /// settings can't change while they run.
    pub fn fixed(self) -> SessionSettingsProvider {
        Arc::new(move || self.clone())
    }
}

/// Gives the settings in force now.  Front ends call it each time a session
/// starts or a cell runs, so a change in the settings applies to the next
/// session without restarting the server.
pub type SessionSettingsProvider = Arc<dyn Fn() -> SessionSettings + Send + Sync>;

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
    settings: &SessionSettings,
    output_sink: Arc<dyn OutputSink>,
    envelope_observer: Option<Arc<dyn EnvelopeObserver>>,
    catalog: &Catalog,
    on_status: Option<&SessionStatusCallback>,
) -> Result<(), AppError> {
    // ctx.path may be a directory (from create_session's path param) or a
    // file (from open_notebook). Use it directly if it's a directory,
    // otherwise take the parent.
    let cwd = ctx.path.as_deref().map(|p| {
        if p.is_dir() {
            p
        } else {
            p.parent().unwrap_or(p)
        }
    });
    let eval_timeout = settings.eval_timeout;
    match MaximaProcess::spawn_with_cwd_and_observer(
        settings.backend.clone(),
        settings.maxima_path.clone(),
        output_sink,
        cwd,
        envelope_observer,
    )
    .await
    {
        Ok(process) => {
            // The new process numbers its labels from the start again:
            // forget the old labels and number executions from 1 again
            ctx.notebook.lock().await.start_new_session();
            ctx.session.set_ready(process).await;

            // Configure texput so Greek letters render correctly in TeX output
            let init = build_texput_init();
            let mut guard = ctx.session.lock().await;
            if let Ok(p) = guard.process_mut() {
                let _ = protocol::evaluate(p, "__init__", &init, catalog, eval_timeout).await;
                // Configure plot2d to produce SVG files (detectable by parser)
                // instead of opening a gnuplot window (not useful in notebook/MCP context).
                let _ = protocol::evaluate(
                    p,
                    "__init__",
                    "set_plot_option([plot_format, svg])$",
                    catalog,
                    eval_timeout,
                )
                .await;
                // Load Lisp helpers for plotting (ax__mktemp)
                let lisp_init = plotting_lisp_stdin();
                p.write_stdin(lisp_init).await.ok();
                // Maxima's error messages in the user's language, as well as
                // the English Aximar reads (ax_messages.lisp).  Sent before
                // the next evaluation, which reads the :lisp lines' replies.
                p.write_stdin(messages_lisp_stdin()).await.ok();
                // Load Aximar plotting functions (ax_plot2d, ax_draw2d, ax_draw3d)
                let plot_init = plotting_init_code();
                let _ = protocol::evaluate(p, "__init__", plot_init, catalog, eval_timeout).await;
                // Maxima's error messages in the user's language
                let language = ctx.language.read().unwrap().clone();
                p.use_language(language.as_deref()).await.ok();
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
    settings: &SessionSettings,
    build_sink: impl FnOnce(&NotebookContextRef) -> Arc<dyn OutputSink>,
    build_observer: impl FnOnce(&NotebookContextRef) -> Option<Arc<dyn EnvelopeObserver>>,
    catalog: &Catalog,
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
            let envelope_observer = build_observer(ctx);
            spawn_and_init_session(
                ctx,
                settings,
                output_sink,
                envelope_observer,
                catalog,
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
