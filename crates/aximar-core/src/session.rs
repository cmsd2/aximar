use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex as StdMutex;

use tokio::sync::{Mutex, MutexGuard};

use crate::error::AppError;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::SessionStatus;

pub enum Session {
    Stopped,
    Starting,
    Ready { process: MaximaProcess },
    Busy { process: MaximaProcess },
    Error(String),
}

impl Session {
    fn status(&self) -> SessionStatus {
        match self {
            Session::Stopped => SessionStatus::Stopped,
            Session::Starting => SessionStatus::Starting,
            Session::Ready { .. } => SessionStatus::Ready,
            Session::Busy { .. } => SessionStatus::Busy,
            Session::Error(msg) => SessionStatus::Error(msg.clone()),
        }
    }

    /// any → Starting, extracting any existing process.
    fn into_starting(self) -> (Self, Option<MaximaProcess>) {
        match self {
            Session::Ready { process } | Session::Busy { process } => {
                (Session::Starting, Some(process))
            }
            _ => (Session::Starting, None),
        }
    }

    /// any → Stopped, extracting any existing process.
    fn into_stopped(self) -> (Self, Option<MaximaProcess>) {
        match self {
            Session::Ready { process } | Session::Busy { process } => {
                (Session::Stopped, Some(process))
            }
            _ => (Session::Stopped, None),
        }
    }

    /// Ready → Busy. Returns the new state, or the original state with an error.
    fn begin_eval(self) -> Result<Self, (Self, AppError)> {
        match self {
            Session::Ready { process } => Ok(Session::Busy { process }),
            session @ Session::Busy { .. } => Err((session, AppError::SessionBusy)),
            other => Err((other, AppError::ProcessNotRunning)),
        }
    }

    /// Busy → Ready. No-op if not Busy.
    fn end_eval(self) -> Self {
        match self {
            Session::Busy { process } => Session::Ready { process },
            other => other,
        }
    }
}

fn sync_status(status_code: &AtomicU8, error_message: &StdMutex<String>, session: &Session) {
    let status = session.status();
    if let SessionStatus::Error(ref msg) = status {
        if let Ok(mut err) = error_message.lock() {
            *err = msg.clone();
        }
    }
    status_code.store(status.as_code(), Ordering::Release);
}

pub struct SessionManager {
    session: Mutex<Session>,
    status_code: AtomicU8,
    error_message: StdMutex<String>,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            session: Mutex::new(Session::Stopped),
            status_code: AtomicU8::new(SessionStatus::Stopped.as_code()),
            error_message: StdMutex::new(String::new()),
        }
    }

    /// Apply a consuming transition to the session state under the lock.
    async fn apply<R>(&self, f: impl FnOnce(Session) -> (Session, R)) -> R {
        let mut guard = self.session.lock().await;
        let old = std::mem::replace(&mut *guard, Session::Stopped);
        let (new, result) = f(old);
        sync_status(&self.status_code, &self.error_message, &new);
        *guard = new;
        result
    }

    /// Transition to Starting, killing any existing process.
    pub async fn begin_start(&self) {
        let old = self.apply(Session::into_starting).await;
        if let Some(mut process) = old {
            let _ = process.kill().await;
        }
    }

    /// Transition to Ready with a new process.
    pub async fn set_ready(&self, process: MaximaProcess) {
        self.apply(|_| (Session::Ready { process }, ())).await
    }

    /// Transition to Error.
    pub async fn set_error(&self, msg: String) {
        self.apply(|_| (Session::Error(msg), ())).await
    }

    /// Transition to Stopped, killing any existing process.
    pub async fn stop(&self) -> Result<(), AppError> {
        let old = self.apply(Session::into_stopped).await;
        if let Some(mut process) = old {
            process.kill().await?;
        }
        Ok(())
    }

    /// Lock for operations that must hold state across async work (eval, variables).
    pub async fn lock(&self) -> SessionGuard<'_> {
        let guard = self.session.lock().await;
        SessionGuard {
            guard,
            status_code: &self.status_code,
            error_message: &self.error_message,
        }
    }

    /// Lock-free status read via atomic mirror.
    pub fn status(&self) -> SessionStatus {
        let code = self.status_code.load(Ordering::Acquire);
        let error_message = &self.error_message;
        SessionStatus::from_code(code, || {
            error_message
                .try_lock()
                .map(|g| g.clone())
                .unwrap_or_else(|_| "Error".to_string())
        })
    }
}

pub struct SessionGuard<'a> {
    guard: MutexGuard<'a, Session>,
    status_code: &'a AtomicU8,
    error_message: &'a StdMutex<String>,
}

impl<'a> SessionGuard<'a> {
    /// Transition Ready → Busy, returning a mutable reference to the process.
    pub fn try_begin_eval(&mut self) -> Result<&mut MaximaProcess, AppError> {
        let old = std::mem::replace(&mut *self.guard, Session::Stopped);
        match old.begin_eval() {
            Ok(new) => {
                *self.guard = new;
                sync_status(self.status_code, self.error_message, &*self.guard);
                match &mut *self.guard {
                    Session::Busy { process } => Ok(process),
                    _ => unreachable!(),
                }
            }
            Err((restored, err)) => {
                *self.guard = restored;
                Err(err)
            }
        }
    }

    /// Transition Busy → Ready after evaluation completes.
    pub fn end_eval(&mut self) {
        let old = std::mem::replace(&mut *self.guard, Session::Stopped);
        *self.guard = old.end_eval();
        sync_status(self.status_code, self.error_message, &*self.guard);
    }

    /// Get a mutable reference to the process if Ready.
    pub fn process_mut(&mut self) -> Result<&mut MaximaProcess, AppError> {
        match &mut *self.guard {
            Session::Ready { process } => Ok(process),
            Session::Busy { .. } => Err(AppError::SessionBusy),
            _ => Err(AppError::ProcessNotRunning),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Session transition tests (synchronous, no process needed) --

    #[test]
    fn stopped_into_starting() {
        let (session, process) = Session::Stopped.into_starting();
        assert_eq!(session.status(), SessionStatus::Starting);
        assert!(process.is_none());
    }

    #[test]
    fn starting_into_starting() {
        let (session, process) = Session::Starting.into_starting();
        assert_eq!(session.status(), SessionStatus::Starting);
        assert!(process.is_none());
    }

    #[test]
    fn error_into_starting() {
        let (session, process) = Session::Error("err".into()).into_starting();
        assert_eq!(session.status(), SessionStatus::Starting);
        assert!(process.is_none());
    }

    #[test]
    fn stopped_into_stopped() {
        let (session, process) = Session::Stopped.into_stopped();
        assert_eq!(session.status(), SessionStatus::Stopped);
        assert!(process.is_none());
    }

    #[test]
    fn error_into_stopped() {
        let (session, process) = Session::Error("err".into()).into_stopped();
        assert_eq!(session.status(), SessionStatus::Stopped);
        assert!(process.is_none());
    }

    #[test]
    fn begin_eval_on_stopped() {
        let result = Session::Stopped.begin_eval();
        assert!(matches!(result, Err((_, AppError::ProcessNotRunning))));
    }

    #[test]
    fn begin_eval_on_starting() {
        let result = Session::Starting.begin_eval();
        assert!(matches!(result, Err((_, AppError::ProcessNotRunning))));
    }

    #[test]
    fn begin_eval_on_error() {
        let result = Session::Error("err".into()).begin_eval();
        assert!(matches!(result, Err((_, AppError::ProcessNotRunning))));
    }

    #[test]
    fn end_eval_on_stopped_is_noop() {
        let session = Session::Stopped.end_eval();
        assert_eq!(session.status(), SessionStatus::Stopped);
    }

    #[test]
    fn end_eval_on_starting_is_noop() {
        let session = Session::Starting.end_eval();
        assert_eq!(session.status(), SessionStatus::Starting);
    }

    // -- SessionManager tests (async) --

    #[tokio::test]
    async fn initial_status_is_stopped() {
        let mgr = SessionManager::new();
        assert_eq!(mgr.status(), SessionStatus::Stopped);
    }

    #[tokio::test]
    async fn begin_start_updates_status() {
        let mgr = SessionManager::new();
        mgr.begin_start().await;
        assert_eq!(mgr.status(), SessionStatus::Starting);
    }

    #[tokio::test]
    async fn set_error_stores_message() {
        let mgr = SessionManager::new();
        mgr.set_error("spawn failed".into()).await;
        assert_eq!(
            mgr.status(),
            SessionStatus::Error("spawn failed".into())
        );
    }

    #[tokio::test]
    async fn stop_after_error_returns_stopped() {
        let mgr = SessionManager::new();
        mgr.set_error("boom".into()).await;
        mgr.stop().await.unwrap();
        assert_eq!(mgr.status(), SessionStatus::Stopped);
    }

    #[tokio::test]
    async fn stop_on_stopped_is_ok() {
        let mgr = SessionManager::new();
        mgr.stop().await.unwrap();
        assert_eq!(mgr.status(), SessionStatus::Stopped);
    }

    // -- SessionGuard tests (eval / process access) --

    #[tokio::test]
    async fn try_begin_eval_on_stopped_returns_process_not_running() {
        let mgr = SessionManager::new();
        let mut guard = mgr.lock().await;
        assert!(matches!(
            guard.try_begin_eval(),
            Err(AppError::ProcessNotRunning)
        ));
    }

    #[tokio::test]
    async fn try_begin_eval_on_starting_returns_process_not_running() {
        let mgr = SessionManager::new();
        mgr.begin_start().await;
        let mut guard = mgr.lock().await;
        assert!(matches!(
            guard.try_begin_eval(),
            Err(AppError::ProcessNotRunning)
        ));
    }

    #[tokio::test]
    async fn try_begin_eval_on_error_returns_process_not_running() {
        let mgr = SessionManager::new();
        mgr.set_error("err".into()).await;
        let mut guard = mgr.lock().await;
        assert!(matches!(
            guard.try_begin_eval(),
            Err(AppError::ProcessNotRunning)
        ));
    }

    #[tokio::test]
    async fn process_mut_on_stopped_returns_process_not_running() {
        let mgr = SessionManager::new();
        let mut guard = mgr.lock().await;
        assert!(matches!(
            guard.process_mut(),
            Err(AppError::ProcessNotRunning)
        ));
    }
}
