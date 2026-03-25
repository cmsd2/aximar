use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::capture::CaptureOutputSink;
use crate::log::ServerLog;
use crate::notebook::Notebook;
use crate::session::SessionManager;

pub type NotebookId = String;

/// Per-notebook state: notebook data, Maxima session, output capture, and server log.
pub struct NotebookContext {
    pub notebook: Arc<Mutex<Notebook>>,
    pub session: Arc<SessionManager>,
    pub capture_sink: Arc<CaptureOutputSink>,
    pub server_log: Arc<ServerLog>,
    pub path: Option<PathBuf>,
}

impl NotebookContext {
    pub fn new() -> Self {
        let server_log = Arc::new(ServerLog::new());
        let capture_sink = Arc::new(CaptureOutputSink::new(server_log.clone()));
        NotebookContext {
            notebook: Arc::new(Mutex::new(Notebook::new())),
            session: Arc::new(SessionManager::new()),
            capture_sink,
            server_log,
            path: None,
        }
    }
}

/// Cheaply cloneable snapshot of Arc references from a NotebookContext.
/// Avoids holding the registry lock during long operations.
#[derive(Clone)]
pub struct NotebookContextRef {
    pub id: NotebookId,
    pub notebook: Arc<Mutex<Notebook>>,
    pub session: Arc<SessionManager>,
    pub capture_sink: Arc<CaptureOutputSink>,
    pub server_log: Arc<ServerLog>,
    pub path: Option<PathBuf>,
}

/// Summary info for listing notebooks.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NotebookInfo {
    pub id: String,
    pub title: String,
    pub path: Option<String>,
    pub is_active: bool,
}

const DEFAULT_NOTEBOOK_ID: &str = "default";

/// Registry of open notebooks. Wraps a map from NotebookId to NotebookContext.
pub struct NotebookRegistry {
    notebooks: HashMap<NotebookId, NotebookContext>,
    active: NotebookId,
    next_counter: u32,
}

impl NotebookRegistry {
    /// Create a new registry with one default notebook.
    pub fn new() -> Self {
        let mut notebooks = HashMap::new();
        notebooks.insert(DEFAULT_NOTEBOOK_ID.to_string(), NotebookContext::new());
        NotebookRegistry {
            notebooks,
            active: DEFAULT_NOTEBOOK_ID.to_string(),
            next_counter: 1,
        }
    }

    /// Get a cheaply cloneable reference to a notebook context.
    pub fn get(&self, id: &str) -> Result<NotebookContextRef, String> {
        let ctx = self
            .notebooks
            .get(id)
            .ok_or_else(|| format!("Notebook '{}' not found", id))?;
        Ok(NotebookContextRef {
            id: id.to_string(),
            notebook: ctx.notebook.clone(),
            session: ctx.session.clone(),
            capture_sink: ctx.capture_sink.clone(),
            server_log: ctx.server_log.clone(),
            path: ctx.path.clone(),
        })
    }

    /// Get the active notebook's ID.
    pub fn active_id(&self) -> &str {
        &self.active
    }

    /// Set the active notebook.
    pub fn set_active(&mut self, id: &str) -> Result<(), String> {
        if !self.notebooks.contains_key(id) {
            return Err(format!("Notebook '{}' not found", id));
        }
        self.active = id.to_string();
        Ok(())
    }

    /// Create a new notebook, returning its ID.
    pub fn create(&mut self) -> NotebookId {
        let id = format!("nb-{}", self.next_counter);
        self.next_counter += 1;
        self.notebooks.insert(id.clone(), NotebookContext::new());
        id
    }

    /// Remove a notebook from the registry. Returns the context for cleanup.
    /// Cannot close the last notebook.
    pub fn close(&mut self, id: &str) -> Result<NotebookContext, String> {
        if self.notebooks.len() <= 1 {
            return Err("Cannot close the last notebook".to_string());
        }
        let ctx = self
            .notebooks
            .remove(id)
            .ok_or_else(|| format!("Notebook '{}' not found", id))?;
        // If the active notebook was closed, switch to another one
        if self.active == id {
            self.active = self.notebooks.keys().next().unwrap().clone();
        }
        Ok(ctx)
    }

    /// List all open notebooks with summary info.
    pub fn list(&self) -> Vec<NotebookInfo> {
        self.notebooks
            .iter()
            .map(|(id, ctx)| {
                let title = ctx
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled".to_string());
                NotebookInfo {
                    id: id.clone(),
                    title,
                    path: ctx.path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    is_active: *id == self.active,
                }
            })
            .collect()
    }

    /// Set the file path for a notebook.
    pub fn set_path(&mut self, id: &str, path: Option<PathBuf>) -> Result<(), String> {
        let ctx = self
            .notebooks
            .get_mut(id)
            .ok_or_else(|| format!("Notebook '{}' not found", id))?;
        ctx.path = path;
        Ok(())
    }

    /// Get a context ref for the active notebook.
    pub fn active_context(&self) -> Result<NotebookContextRef, String> {
        self.get(&self.active)
    }

    /// Resolve a notebook_id: use the given ID if Some, otherwise the active notebook.
    pub fn resolve(&self, notebook_id: Option<&str>) -> Result<NotebookContextRef, String> {
        let id = notebook_id.unwrap_or(&self.active);
        self.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_has_one_notebook() {
        let reg = NotebookRegistry::new();
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.active_id(), DEFAULT_NOTEBOOK_ID);
    }

    #[test]
    fn get_default_notebook() {
        let reg = NotebookRegistry::new();
        let ctx = reg.get(DEFAULT_NOTEBOOK_ID).unwrap();
        assert_eq!(ctx.id, DEFAULT_NOTEBOOK_ID);
    }

    #[test]
    fn get_nonexistent_notebook_errors() {
        let reg = NotebookRegistry::new();
        assert!(reg.get("nonexistent").is_err());
    }

    #[test]
    fn create_notebook() {
        let mut reg = NotebookRegistry::new();
        let id = reg.create();
        assert_eq!(reg.list().len(), 2);
        assert!(reg.get(&id).is_ok());
    }

    #[test]
    fn close_notebook() {
        let mut reg = NotebookRegistry::new();
        let id = reg.create();
        reg.close(&id).unwrap();
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn close_last_notebook_errors() {
        let mut reg = NotebookRegistry::new();
        assert!(reg.close(DEFAULT_NOTEBOOK_ID).is_err());
    }

    #[test]
    fn close_active_switches_to_another() {
        let mut reg = NotebookRegistry::new();
        let id = reg.create();
        reg.set_active(&id).unwrap();
        reg.close(&id).unwrap();
        assert_eq!(reg.active_id(), DEFAULT_NOTEBOOK_ID);
    }

    #[test]
    fn set_active_nonexistent_errors() {
        let mut reg = NotebookRegistry::new();
        assert!(reg.set_active("nonexistent").is_err());
    }

    #[test]
    fn resolve_with_none_uses_active() {
        let reg = NotebookRegistry::new();
        let ctx = reg.resolve(None).unwrap();
        assert_eq!(ctx.id, DEFAULT_NOTEBOOK_ID);
    }

    #[test]
    fn resolve_with_id_uses_id() {
        let mut reg = NotebookRegistry::new();
        let id = reg.create();
        let ctx = reg.resolve(Some(&id)).unwrap();
        assert_eq!(ctx.id, id);
    }

    #[test]
    fn set_path() {
        let mut reg = NotebookRegistry::new();
        reg.set_path(DEFAULT_NOTEBOOK_ID, Some(PathBuf::from("/tmp/test.ipynb")))
            .unwrap();
        let info = reg.list();
        assert_eq!(info[0].path.as_deref(), Some("/tmp/test.ipynb"));
    }
}
